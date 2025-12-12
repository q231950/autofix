// Claude AI provider implementation

use super::{
    LLMError, LLMRequest, LLMResponse, MessageRole, ProviderConfig, ProviderType,
    StopReason, TokenUsage, ToolCall, ToolDefinition,
};
use crate::llm::provider_trait::LLMProvider;
use crate::rate_limiter::RateLimiter;
use anthropic_sdk::{
    Anthropic, ContentBlock, ContentBlockParam, MessageContent, MessageCreateBuilder,
    StopReason as AnthropicStopReason, Tool as AnthropicTool, ToolChoice,
};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Claude provider implementation
pub struct ClaudeProvider {
    config: ProviderConfig,
    client: Anthropic,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl ClaudeProvider {
    /// Convert tool definitions to Claude format
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Result<Vec<AnthropicTool>, LLMError> {
        tools
            .iter()
            .map(|tool| {
                // The Tool type can be deserialized from JSON
                serde_json::from_value(serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema,
                }))
                .map_err(|e| {
                    LLMError::InvalidRequest(format!("Failed to convert tool definition: {}", e))
                })
            })
            .collect()
    }

    /// Convert Claude response to LLMResponse
    fn convert_response(
        &self,
        response: anthropic_sdk::Message,
    ) -> Result<LLMResponse, LLMError> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        // Extract content and tool calls from response
        for block in response.content {
            match block {
                ContentBlock::Text { text } => {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&text);
                }
                ContentBlock::ToolUse {
                    id,
                    name,
                    input,
                } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        input,
                    });
                }
                ContentBlock::Image { .. } => {
                    // Images in responses are not expected in this context
                    // Silently skip them for now
                }
                ContentBlock::ToolResult { .. } => {
                    // Tool results in responses are not expected in this context
                    // Silently skip them for now
                }
            }
        }

        // Convert stop reason
        let stop_reason = match response.stop_reason {
            Some(AnthropicStopReason::EndTurn) => StopReason::EndTurn,
            Some(AnthropicStopReason::MaxTokens) => StopReason::MaxTokens,
            Some(AnthropicStopReason::StopSequence) => StopReason::StopSequence,
            Some(AnthropicStopReason::ToolUse) => StopReason::ToolUse,
            None => StopReason::Error,
        };

        // Extract token usage
        let usage = TokenUsage::new(
            response.usage.input_tokens as u32,
            response.usage.output_tokens as u32,
        );

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
impl LLMProvider for ClaudeProvider {
    fn new(config: ProviderConfig) -> Result<Self, LLMError> {
        // Validate configuration
        Self::validate_config(&config)?;

        // Create Anthropic client
        let client = Anthropic::from_env().map_err(|e| {
            LLMError::ConfigurationError(format!("Failed to create Anthropic client: {}", e))
        })?;

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
        ProviderType::Claude
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

        // Determine max_tokens - required parameter
        let max_tokens = request.max_tokens.unwrap_or(4096);

        // Build request with model and max_tokens (both required in constructor)
        let mut builder = MessageCreateBuilder::new(&self.config.model, max_tokens);

        // Add system prompt if present
        if let Some(system) = &request.system_prompt {
            builder = builder.system(system.clone());
        }

        // Add messages - alternate between user and assistant
        for message in &request.messages {
            let content_block = ContentBlockParam::Text {
                text: message.content.clone(),
            };
            let content = MessageContent::Blocks(vec![content_block]);

            builder = match message.role {
                MessageRole::User | MessageRole::Tool => builder.user(content),
                MessageRole::Assistant => builder.assistant(content),
            };
        }

        // Add tools if present
        if !request.tools.is_empty() {
            let tools = self.convert_tools(&request.tools)?;
            builder = builder.tools(tools).tool_choice(ToolChoice::Auto);
        }

        // Add temperature if present
        if let Some(temperature) = request.temperature {
            builder = builder.temperature(temperature as f32);
        }

        // Send request
        let response = self
            .client
            .messages()
            .create(builder.build())
            .await
            .map_err(|e| {
                // Sanitize error message to remove potential API keys
                let error_msg = format!("{}", e);
                let sanitized = error_msg
                    .replace(self.config.api_key(), "[REDACTED]")
                    .replace("sk-ant-", "[REDACTED]");
                LLMError::InvalidRequest(sanitized)
            })?;

        // Record actual usage
        {
            let limiter = self.rate_limiter.lock().await;
            limiter.record_usage((response.usage.input_tokens + response.usage.output_tokens) as usize);
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
        if config.provider_type != ProviderType::Claude {
            return Err(LLMError::ConfigurationError(
                "Invalid provider type for Claude provider".to_string(),
            ));
        }

        // Check API key is not empty
        if config.api_key().is_empty() {
            return Err(LLMError::ConfigurationError(
                "API key is required for Claude provider".to_string(),
            ));
        }

        // Check endpoint is HTTPS
        if !config.api_base.starts_with("https://") {
            return Err(LLMError::ConfigurationError(
                "Claude API endpoint must use HTTPS".to_string(),
            ));
        }

        // Check model is valid (basic check)
        if !config.model.starts_with("claude-") {
            return Err(LLMError::ConfigurationError(format!(
                "Invalid Claude model: {}",
                config.model
            )));
        }

        Ok(())
    }

    fn max_context_length(&self) -> u32 {
        // Claude Sonnet 4 and Haiku 3.5 have 200k context
        if self.config.model.contains("sonnet")
            || self.config.model.contains("haiku")
            || self.config.model.contains("opus")
        {
            200000
        } else {
            // Default for older models
            100000
        }
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }
}
