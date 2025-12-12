// OpenAI provider implementation

use super::{
    LLMError, LLMRequest, LLMResponse, MessageRole, ProviderConfig, ProviderType,
    StopReason, TokenUsage, ToolCall, ToolDefinition,
};
use crate::llm::provider_trait::LLMProvider;
use crate::rate_limiter::RateLimiter;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionTool, ChatCompletionToolType, ChatCompletionToolChoiceOption,
        CreateChatCompletionRequestArgs, FinishReason, FunctionObjectArgs,
    },
    Client,
};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// OpenAI provider implementation
pub struct OpenAIProvider {
    config: ProviderConfig,
    client: Client<OpenAIConfig>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl OpenAIProvider {
    /// Convert tool definitions to OpenAI format
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

    /// Convert OpenAI response to LLMResponse
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

        // Extract tool calls
        if let Some(calls) = &choice.message.tool_calls {
            for call in calls {
                tool_calls.push(ToolCall {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    input: serde_json::from_str(&call.function.arguments).unwrap_or_default(),
                });
            }
        }

        // Convert stop reason
        let stop_reason = match choice.finish_reason {
            Some(FinishReason::Stop) => StopReason::EndTurn,
            Some(FinishReason::Length) => StopReason::MaxTokens,
            Some(FinishReason::ToolCalls) => StopReason::ToolUse,
            Some(FinishReason::FunctionCall) => StopReason::ToolUse, // Legacy function calling
            Some(FinishReason::ContentFilter) => StopReason::Error,
            None => StopReason::Error,
        };

        // Extract token usage
        let usage = if let Some(usage_info) = response.usage {
            TokenUsage::new(
                usage_info.prompt_tokens as u32,
                usage_info.completion_tokens as u32,
            )
        } else {
            TokenUsage::new(0, 0)
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
impl LLMProvider for OpenAIProvider {
    fn new(config: ProviderConfig) -> Result<Self, LLMError> {
        // Validate configuration
        Self::validate_config(&config)?;

        // Create OpenAI client with custom endpoint
        let openai_config = OpenAIConfig::new()
            .with_api_key(config.api_key())
            .with_api_base(&config.api_base);

        let client = Client::with_config(openai_config);

        // Create rate limiter
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
        ProviderType::OpenAI
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        // Estimate tokens and check rate limiter
        let estimated_tokens = self.estimate_tokens(&request);
        {
            let limiter = self.rate_limiter.lock().await;
            if let Err(wait_duration) = limiter.check_and_wait(estimated_tokens as usize) {
                // Wait for rate limit to reset
                tokio::time::sleep(wait_duration).await;
            }
        }

        // Build messages
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
                MessageRole::Assistant => {
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(message.content.clone())
                        .build()
                        .map_err(|e| {
                            LLMError::InvalidRequest(format!(
                                "Failed to build assistant message: {}",
                                e
                            ))
                        })?
                        .into()
                }
            };
            messages.push(msg);
        }

        // Build request
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder.model(&self.config.model).messages(messages);

        // Add tools if present
        if !request.tools.is_empty() {
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
            request_builder.temperature(temperature as f32);
        }

        let chat_request = request_builder.build().map_err(|e| {
            LLMError::InvalidRequest(format!("Failed to build request: {}", e))
        })?;

        // Send request
        let response = self
            .client
            .chat()
            .create(chat_request)
            .await
            .map_err(|e| {
                // Sanitize error message to remove potential API keys
                let error_msg = format!("{}", e);
                let sanitized = error_msg.replace(self.config.api_key(), "[REDACTED]");
                LLMError::InvalidRequest(sanitized)
            })?;

        // Record actual usage
        {
            let limiter = self.rate_limiter.lock().await;
            if let Some(usage_info) = &response.usage {
                limiter.record_usage(
                    (usage_info.prompt_tokens + usage_info.completion_tokens) as usize,
                );
            }
        }

        // Convert to LLMResponse
        self.convert_response(response)
    }

    async fn complete_stream(
        &self,
        _request: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LLMResponse, LLMError>> + Send>>, LLMError> {
        // Streaming support to be implemented
        Err(LLMError::StreamingNotSupported)
    }

    fn estimate_tokens(&self, request: &LLMRequest) -> u32 {
        // Rough heuristic: 4 characters = 1 token
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

        // Add tool definitions overhead
        let tool_tokens: u32 = request
            .tools
            .iter()
            .map(|t| ((t.description.len() + t.input_schema.to_string().len()) / 4) as u32)
            .sum();

        // Estimate output tokens
        let output_tokens = request.max_tokens.unwrap_or(1000);

        input_tokens + tool_tokens + output_tokens
    }

    fn validate_config(config: &ProviderConfig) -> Result<(), LLMError> {
        // Check provider type
        if config.provider_type != ProviderType::OpenAI {
            return Err(LLMError::ConfigurationError(
                "Invalid provider type for OpenAI provider".to_string(),
            ));
        }

        // Check API key is not empty
        if config.api_key().is_empty() {
            return Err(LLMError::ConfigurationError(
                "API key is required for OpenAI provider".to_string(),
            ));
        }

        // Check endpoint is valid HTTP/HTTPS URL
        if !config.api_base.starts_with("http://") && !config.api_base.starts_with("https://") {
            return Err(LLMError::ConfigurationError(
                "OpenAI API endpoint must be a valid HTTP or HTTPS URL".to_string(),
            ));
        }

        // Check model is not empty
        if config.model.is_empty() {
            return Err(LLMError::ConfigurationError(
                "Model name is required for OpenAI provider".to_string(),
            ));
        }

        Ok(())
    }

    fn max_context_length(&self) -> u32 {
        // Return context length based on model name
        if self.config.model.contains("gpt-4-turbo") || self.config.model.contains("gpt-4o") {
            128000
        } else if self.config.model.contains("gpt-4") {
            8192
        } else if self.config.model.contains("gpt-3.5-turbo") {
            16385
        } else {
            // Default for unknown models
            8192
        }
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
