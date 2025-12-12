// LLM Provider abstraction module
// Provides a unified interface for multiple LLM providers (Claude, OpenAI, Ollama)

pub mod claude_provider;
pub mod config;
pub mod ollama_provider;
pub mod openai_provider;
pub mod provider_trait;

// Re-export core types
pub use claude_provider::ClaudeProvider;
pub use config::{ProviderConfig, ProviderType};
pub use ollama_provider::OllamaProvider;
pub use openai_provider::OpenAIProvider;
pub use provider_trait::LLMProvider;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

/// Role of a message sender
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

/// A request to an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

/// A response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
}

/// Definition of a tool available to the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// A tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// Token usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

/// Reason why LLM generation stopped
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    Error,
}

/// Errors that can occur with LLM providers
#[derive(Debug, Error)]
pub enum LLMError {
    #[error("Authentication failed: invalid API key")]
    AuthenticationError,

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Server error: status {status}")]
    ServerError { status: u16 },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Streaming not supported by this provider")]
    StreamingNotSupported,

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Factory for creating LLM providers
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from configuration
    pub fn create(config: ProviderConfig) -> Result<Box<dyn LLMProvider>, LLMError> {
        match config.provider_type {
            ProviderType::Claude => {
                ClaudeProvider::validate_config(&config)?;
                Ok(Box::new(ClaudeProvider::new(config)?))
            }
            ProviderType::OpenAI => {
                OpenAIProvider::validate_config(&config)?;
                Ok(Box::new(OpenAIProvider::new(config)?))
            }
            ProviderType::Ollama => {
                OllamaProvider::validate_config(&config)?;
                Ok(Box::new(OllamaProvider::new(config)?))
            }
        }
    }
}
