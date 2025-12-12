// Ollama provider implementation
// Reuses async-openai client since Ollama is OpenAI-compatible

use super::{
    LLMError, LLMRequest, LLMResponse, MessageRole, ProviderConfig, ProviderType, StopReason,
    TokenUsage, ToolCall, ToolDefinition,
};
use crate::llm::provider_trait::LLMProvider;
use crate::rate_limiter::RateLimiter;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        ChatCompletionTool, ChatCompletionToolChoiceOption, ChatCompletionToolType,
        CreateChatCompletionRequestArgs, FinishReason, FunctionObjectArgs,
    },
};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Ollama provider implementation
/// Uses async-openai client with Ollama endpoint for local model access
pub struct OllamaProvider {
    config: ProviderConfig,
    client: Client<OpenAIConfig>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl OllamaProvider {
    /// Convert tool definitions to Ollama format (same as OpenAI)
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Result<Vec<ChatCompletionTool>, LLMError> {
        tools
            .iter()
            .map(|tool| {
                let function = FunctionObjectArgs::default()
                    .name(&tool.name)
                    .description(&tool.description)
                    .parameters(tool.input_schema.clone())
                    .build()
                    .map_err(|e| {
                        LLMError::InvalidRequest(format!("Failed to build function object: {}", e))
                    })?;

                Ok(ChatCompletionTool {
                    r#type: ChatCompletionToolType::Function,
                    function,
                })
            })
            .collect()
    }

    /// Convert Ollama response to LLMResponse (same as OpenAI)
    fn convert_response(
        &self,
        response: async_openai::types::CreateChatCompletionResponse,
    ) -> Result<LLMResponse, LLMError> {
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LLMError::InvalidRequest("No choices in response".to_string()))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        // Extract content
        if let Some(msg_content) = &choice.message.content {
            content = msg_content.clone();
        }

        // Extract tool calls (may not be supported by all Ollama models)
        if let Some(calls) = &choice.message.tool_calls {
            for call in calls {
                tool_calls.push(ToolCall {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    input: serde_json::from_str(&call.function.arguments).unwrap_or_default(),
                });
            }
        }

        // Convert stop reason (Ollama may have limited stop reasons)
        let stop_reason = match choice.finish_reason {
            Some(FinishReason::Stop) => StopReason::EndTurn,
            Some(FinishReason::Length) => StopReason::MaxTokens,
            Some(FinishReason::ToolCalls) => StopReason::ToolUse,
            Some(FinishReason::FunctionCall) => StopReason::ToolUse,
            Some(FinishReason::ContentFilter) => StopReason::Error,
            None => StopReason::EndTurn, // Ollama often doesn't provide finish_reason
        };

        // Extract token usage (may not be provided by all Ollama models)
        let usage = if let Some(usage_info) = response.usage {
            TokenUsage::new(
                usage_info.prompt_tokens,
                usage_info.completion_tokens,
            )
        } else {
            // Ollama may not provide usage info, estimate based on content
            let estimated_output = (content.len() / 4) as u32;
            TokenUsage::new(0, estimated_output)
        };

        Ok(LLMResponse {
            content: if content.is_empty() {
                None
            } else {
                Some(content)
            },
            tool_calls,
            stop_reason,
            usage,
        })
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn new(config: ProviderConfig) -> Result<Self, LLMError> {
        // Validate configuration
        Self::validate_config(&config)?;

        // Create OpenAI-compatible client for Ollama
        // Ollama doesn't require authentication, but async-openai needs a key
        let api_key = if config.api_key().is_empty() || config.api_key() == "ollama" {
            "ollama".to_string()
        } else {
            config.api_key().to_string()
        };

        let openai_config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(&config.api_base);

        let client = Client::with_config(openai_config);

        // Create rate limiter (often unlimited for local usage)
        let rate_limiter = Arc::new(Mutex::new(RateLimiter::for_provider(
            config.provider_type,
            config.rate_limit_tpm,
        )));

        Ok(Self {
            config,
            client,
            rate_limiter,
        })
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Ollama
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        // Estimate tokens and check rate limiter (skip if rate_limit_tpm is 0 or None)
        let should_rate_limit =
            self.config.rate_limit_tpm.is_some() && self.config.rate_limit_tpm != Some(0);

        if should_rate_limit {
            let estimated_tokens = self.estimate_tokens(&request);
            let limiter = self.rate_limiter.lock().await;
            if let Err(wait_duration) = limiter.check_and_wait(estimated_tokens as usize) {
                // Wait for rate limit to reset
                tokio::time::sleep(wait_duration).await;
            }
        }

        // Build messages (same as OpenAI)
        let mut messages: Vec<ChatCompletionRequestMessage> = Vec::new();

        // Add system prompt if present
        if let Some(system) = &request.system_prompt {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system.clone())
                    .build()
                    .map_err(|e| {
                        LLMError::InvalidRequest(format!("Failed to build system message: {}", e))
                    })?
                    .into(),
            );
        }

        // Add conversation messages
        for message in &request.messages {
            let msg = match message.role {
                MessageRole::User | MessageRole::Tool => {
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(message.content.clone())
                        .build()
                        .map_err(|e| {
                            LLMError::InvalidRequest(format!("Failed to build user message: {}", e))
                        })?
                        .into()
                }
                MessageRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(message.content.clone())
                    .build()
                    .map_err(|e| {
                        LLMError::InvalidRequest(format!(
                            "Failed to build assistant message: {}",
                            e
                        ))
                    })?
                    .into(),
            };
            messages.push(msg);
        }

        // Build request
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.config.model).messages(messages);

        // Add tools if present (note: not all Ollama models support tools)
        if !request.tools.is_empty() && self.supports_tools() {
            let tools = self.convert_tools(&request.tools)?;
            request_builder
                .tools(tools)
                .tool_choice(ChatCompletionToolChoiceOption::Auto);
        }

        // Add parameters
        if let Some(max_tokens) = request.max_tokens {
            request_builder.max_tokens(max_tokens as u16);
        }
        if let Some(temperature) = request.temperature {
            request_builder.temperature(temperature);
        }

        let chat_request = request_builder
            .build()
            .map_err(|e| LLMError::InvalidRequest(format!("Failed to build request: {}", e)))?;

        // Send request to local Ollama instance
        let response = self.client.chat().create(chat_request).await.map_err(|e| {
            let error_msg = format!("{}", e);
            LLMError::InvalidRequest(format!("Ollama error: {}", error_msg))
        })?;

        // Record actual usage (if rate limiting is enabled)
        if should_rate_limit
            && let Some(usage_info) = &response.usage {
                let limiter = self.rate_limiter.lock().await;
                limiter.record_usage(
                    (usage_info.prompt_tokens + usage_info.completion_tokens) as usize,
                );
            }

        // Convert to LLMResponse
        self.convert_response(response)
    }

    async fn complete_stream(
        &self,
        _request: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LLMResponse, LLMError>> + Send>>, LLMError> {
        // Streaming support to be implemented
        // Note: Ollama supports streaming but implementation depends on model
        Err(LLMError::StreamingNotSupported)
    }

    fn estimate_tokens(&self, request: &LLMRequest) -> u32 {
        // Same heuristic as other providers: 4 characters = 1 token
        let mut char_count = 0;

        // Count system prompt
        if let Some(system) = &request.system_prompt {
            char_count += system.len();
        }

        // Count messages
        for message in &request.messages {
            char_count += message.content.len();
        }

        let input_tokens = (char_count / 4) as u32;

        // Add tool definitions overhead (if tools are supported)
        let tool_tokens: u32 = if self.supports_tools() {
            request
                .tools
                .iter()
                .map(|t| ((t.description.len() + t.input_schema.to_string().len()) / 4) as u32)
                .sum()
        } else {
            0
        };

        // Estimate output tokens
        let output_tokens = request.max_tokens.unwrap_or(1000);

        input_tokens + tool_tokens + output_tokens
    }

    fn validate_config(config: &ProviderConfig) -> Result<(), LLMError> {
        // Check provider type
        if config.provider_type != ProviderType::Ollama {
            return Err(LLMError::ConfigurationError(
                "Invalid provider type for Ollama provider".to_string(),
            ));
        }

        // API key is optional for Ollama (local usage)
        // Just check that it's not required to be set

        // Check endpoint is localhost
        if !config.api_base.starts_with("http://localhost:")
            && !config.api_base.starts_with("http://127.0.0.1:")
        {
            return Err(LLMError::ConfigurationError(
                "Ollama endpoint must be localhost (http://localhost:11434/v1 or similar)"
                    .to_string(),
            ));
        }

        // Check model is not empty
        if config.model.is_empty() {
            return Err(LLMError::ConfigurationError(
                "Model name is required for Ollama provider".to_string(),
            ));
        }

        Ok(())
    }

    fn max_context_length(&self) -> u32 {
        // Return context length based on model name
        // These are typical values for popular Ollama models
        if self.config.model.contains("codellama") {
            16384
        } else if self.config.model.contains("mistral") {
            32768
        } else if self.config.model.contains("llama2") {
            4096
        } else if self.config.model.contains("llama3") {
            8192
        } else if self.config.model.contains("phi") {
            2048
        } else {
            // Default for unknown models
            4096
        }
    }

    fn supports_streaming(&self) -> bool {
        // Ollama supports streaming for most models
        true
    }

    fn supports_tools(&self) -> bool {
        // Tool support is model-dependent in Ollama
        // For now, return false by default - can be enhanced later
        // Models like codellama and mistral may support function calling
        false
    }
}
