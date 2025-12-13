// LLM Provider trait - unified interface for all LLM providers

use super::{LLMError, LLMRequest, LLMResponse, ProviderConfig, ProviderType};
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

/// Trait that all LLM providers must implement
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Create a new provider instance from configuration
    fn new(config: ProviderConfig) -> Result<Self, LLMError>
    where
        Self: Sized;

    /// Get the provider type
    fn provider_type(&self) -> ProviderType;

    /// Send a request and get a complete response (non-streaming)
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;

    /// Send a request and get a streaming response
    #[allow(dead_code)] // Streaming not yet used in pipeline but implemented in providers
    async fn complete_stream(
        &self,
        request: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LLMResponse, LLMError>> + Send>>, LLMError>;

    /// Estimate token count for a request (for rate limiting)
    fn estimate_tokens(&self, request: &LLMRequest) -> u32;

    /// Validate provider-specific configuration
    fn validate_config(config: &ProviderConfig) -> Result<(), LLMError>
    where
        Self: Sized;

    /// Get maximum context length for this provider/model
    #[allow(dead_code)] // Not yet used but part of provider trait interface
    fn max_context_length(&self) -> u32;

    /// Check if provider supports streaming
    #[allow(dead_code)] // Not yet used but part of provider trait interface
    fn supports_streaming(&self) -> bool {
        true // Default: most providers support streaming
    }

    /// Check if provider supports function/tool calling
    fn supports_tools(&self) -> bool {
        true // Default: most providers support tools
    }
}
