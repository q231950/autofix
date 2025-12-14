# Contract: LLMProvider Trait

**Date**: 2025-12-12
**Feature**: 001-llm-provider-support

## Overview

This contract defines the interface that all LLM providers (Claude, OpenAI, Ollama) must implement. The trait ensures consistent behavior across providers while abstracting provider-specific implementation details.

## Trait Definition

```rust
use async_trait::async_trait;
use crate::llm::{LLMRequest, LLMResponse, ProviderConfig};
use std::error::Error;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Create a new provider instance from configuration
    fn new(config: ProviderConfig) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    /// Get the provider type
    fn provider_type(&self) -> ProviderType;

    /// Send a request and get a complete response
    async fn complete(
        &self,
        request: LLMRequest,
    ) -> Result<LLMResponse, Box<dyn Error>>;

    /// Send a request and get a streaming response
    async fn complete_stream(
        &self,
        request: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LLMResponse, Box<dyn Error>>> + Send>>, Box<dyn Error>>;

    /// Estimate token count for a request (for rate limiting)
    fn estimate_tokens(&self, request: &LLMRequest) -> u32;

    /// Validate provider-specific configuration
    fn validate_config(config: &ProviderConfig) -> Result<(), Box<dyn Error>>;

    /// Get maximum context length for this provider/model
    fn max_context_length(&self) -> u32;

    /// Check if provider supports streaming
    fn supports_streaming(&self) -> bool {
        true // Default: most providers support streaming
    }

    /// Check if provider supports function/tool calling
    fn supports_tools(&self) -> bool {
        true // Default: most providers support tools
    }
}
```

## Method Specifications

### `new(config: ProviderConfig) -> Result<Self, Box<dyn Error>>`

**Purpose**: Construct a new provider instance from configuration.

**Preconditions**:
- `config` must have valid `api_key` for non-local providers
- `config.api_base` must be reachable URL
- `validate_config(&config)` must return `Ok(())`

**Postconditions**:
- Returns configured provider ready to accept requests
- Internal HTTP client created with retry/timeout settings
- Rate limiter initialized from `config.rate_limit_tpm`

**Errors**:
- `InvalidApiKey`: Missing or invalid API key
- `InvalidEndpoint`: Unreachable or malformed `api_base` URL
- `ConfigurationError`: Other configuration validation failures

**Example**:
```rust
let config = ProviderConfig {
    provider_type: ProviderType::OpenAI,
    api_key: SecretString::new(env::var("OPENAI_API_KEY")?),
    api_base: "https://api.openai.com/v1".to_string(),
    model: "gpt-4".to_string(),
    ..Default::default()
};

let provider = OpenAIProvider::new(config)?;
```

---

### `provider_type(&self) -> ProviderType`

**Purpose**: Return the provider type enum.

**Preconditions**: None

**Postconditions**: Returns `ProviderType::{Claude, OpenAI, Ollama}`

**Errors**: None (infallible)

**Example**:
```rust
let provider_type = provider.provider_type();
match provider_type {
    ProviderType::Claude => println!("Using Claude"),
    ProviderType::OpenAI => println!("Using OpenAI"),
    ProviderType::Ollama => println!("Using Ollama"),
}
```

---

### `complete(&self, request: LLMRequest) -> Result<LLMResponse, Box<dyn Error>>`

**Purpose**: Send a request and wait for complete response (non-streaming).

**Preconditions**:
- `request.messages` must not be empty
- `request.tools` must be valid ToolDefinitions
- Rate limiter must allow request (sufficient token budget)

**Postconditions**:
- Returns normalized `LLMResponse`
- `LLMResponse.usage` contains accurate token counts
- Rate limiter updated with actual token usage

**Errors**:
- `AuthenticationError`: Invalid API key
- `RateLimitError`: Rate limit exceeded (429 status)
- `NetworkError`: Connection timeout or network failure
- `ServerError`: Provider 5xx errors
- `InvalidRequest`: Malformed request rejected by provider

**Retry Behavior**:
- Automatic retry for transient errors (network, 5xx)
- Exponential backoff: 1s, 2s, 4s (max 3 retries)
- No retry for permanent errors (auth, invalid request)

**Example**:
```rust
let request = LLMRequest {
    messages: vec![
        Message { role: MessageRole::User, content: "Hello!".to_string() }
    ],
    tools: vec![],
    max_tokens: Some(1000),
    ..Default::default()
};

let response = provider.complete(request).await?;
println!("Response: {}", response.content.unwrap());
println!("Tokens used: {}", response.usage.total_tokens);
```

---

### `complete_stream(&self, request: LLMRequest) -> Result<Stream<LLMResponse>, Box<dyn Error>>`

**Purpose**: Send a request and get streaming response chunks.

**Preconditions**: Same as `complete()`, plus `supports_streaming()` must return `true`

**Postconditions**:
- Returns async stream of `LLMResponse` chunks
- Each chunk contains partial `content` or complete `tool_calls`
- Final chunk has complete `usage` information
- Stream ends when `stop_reason` is terminal

**Errors**: Same as `complete()`, plus:
- `StreamingNotSupported`: Provider doesn't support streaming

**Stream Behavior**:
- Yields chunks as soon as available (real-time display)
- Accumulate chunks to build complete response
- Last chunk always contains final `TokenUsage`

**Example**:
```rust
let mut stream = provider.complete_stream(request).await?;

let mut full_content = String::new();
while let Some(result) = stream.next().await {
    let chunk = result?;
    if let Some(content) = chunk.content {
        full_content.push_str(&content);
        print!("{}", content); // Real-time display
    }
}
println!("\nTotal tokens: {}", stream.last().unwrap().usage.total_tokens);
```

---

### `estimate_tokens(&self, request: &LLMRequest) -> u32`

**Purpose**: Estimate token count for rate limiting before sending request.

**Preconditions**: Valid `LLMRequest` structure

**Postconditions**:
- Returns estimated token count (input + estimated output)
- Accuracy within 10% of actual usage (per NFR-005)
- Conservative estimate (prefer over-estimation to avoid rate limits)

**Errors**: None (best-effort estimation)

**Estimation Algorithm**:
```rust
// Rough heuristic: 4 characters = 1 token
let char_count: usize = request.messages.iter()
    .map(|m| m.content.len())
    .sum();

let input_tokens = (char_count / 4) as u32;

// Add tool definitions overhead (schema tokens)
let tool_tokens: u32 = request.tools.iter()
    .map(|t| (t.description.len() + t.input_schema.to_string().len()) / 4)
    .sum::<usize>() as u32;

// Estimate output tokens from max_tokens or default
let output_tokens = request.max_tokens.unwrap_or(1000);

input_tokens + tool_tokens + output_tokens
```

**Example**:
```rust
let estimated = provider.estimate_tokens(&request);
rate_limiter.check_and_reserve(estimated)?; // May wait if limit exceeded
```

---

### `validate_config(config: &ProviderConfig) -> Result<(), Box<dyn Error>>`

**Purpose**: Validate provider-specific configuration before construction.

**Preconditions**: None (static method)

**Postconditions**:
- Returns `Ok(())` if configuration is valid
- Returns `Err` with specific validation error

**Errors**:
- `MissingApiKey`: Required API key not provided
- `InvalidEndpoint`: Malformed or unreachable URL
- `UnsupportedModel`: Model not available for this provider
- `InvalidTimeout`: Timeout out of valid range

**Provider-Specific Rules**:

| Provider | API Key | Endpoint | Model |
|----------|---------|----------|-------|
| Claude | Required | Must start with `https://` | Must be valid Claude model |
| OpenAI | Required | Must be valid HTTP(S) URL | Must be OpenAI model name |
| Ollama | Optional | Must be `http://localhost:*` | Must be installed Ollama model |

**Example**:
```rust
let config = ProviderConfig::from_env()?;

ClaudeProvider::validate_config(&config)?; // Check before instantiation
let provider = ClaudeProvider::new(config)?;
```

---

### `max_context_length(&self) -> u32`

**Purpose**: Return maximum context window size for this provider/model.

**Preconditions**: Provider successfully constructed

**Postconditions**: Returns token count for max context

**Errors**: None (infallible)

**Model-Specific Values**:

| Provider | Model | Context Length |
|----------|-------|----------------|
| Claude | `claude-sonnet-4` | 200,000 |
| Claude | `claude-haiku-3.5` | 200,000 |
| OpenAI | `gpt-4-turbo` | 128,000 |
| OpenAI | `gpt-4` | 8,192 |
| Ollama | `llama2` | 4,096 |
| Ollama | `mistral` | 32,768 |

**Example**:
```rust
let max_context = provider.max_context_length();
if request_tokens > max_context {
    return Err("Request exceeds maximum context length".into());
}
```

---

### `supports_streaming(&self) -> bool`

**Purpose**: Check if provider supports streaming responses.

**Preconditions**: None

**Postconditions**: Returns `true` if `complete_stream()` is supported

**Errors**: None (infallible)

**Provider Support**:
- Claude: `true` (Server-Sent Events)
- OpenAI: `true` (Server-Sent Events)
- Ollama: `true` (streaming via `/api/generate`)

**Example**:
```rust
if provider.supports_streaming() {
    let stream = provider.complete_stream(request).await?;
    // Process stream
} else {
    let response = provider.complete(request).await?;
    // Process complete response
}
```

---

### `supports_tools(&self) -> bool`

**Purpose**: Check if provider supports function/tool calling.

**Preconditions**: None

**Postconditions**: Returns `true` if provider can handle `request.tools`

**Errors**: None (infallible)

**Provider Support**:
- Claude: `true` (native tool use support)
- OpenAI: `true` (function calling)
- Ollama: **Depends on model** (some models support it, others don't)

**Example**:
```rust
if provider.supports_tools() {
    request.tools = vec![
        directory_inspector_tool(),
        code_editor_tool(),
        test_runner_tool(),
    ];
} else {
    // Fallback: include tool descriptions in system prompt
    request.system_prompt = Some(generate_tool_prompt());
}
```

---

## Provider Factory

```rust
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(config: ProviderConfig) -> Result<Box<dyn LLMProvider>, Box<dyn Error>> {
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
```

**Usage**:
```rust
let config = ProviderConfig::from_env()?;
let provider: Box<dyn LLMProvider> = ProviderFactory::create(config)?;

// Use provider trait methods
let response = provider.complete(request).await?;
```

## Error Handling

All provider implementations must use consistent error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Authentication failed: invalid API key")]
    AuthenticationError,

    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Server error: {status}")]
    ServerError { status: u16 },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Streaming not supported by this provider")]
    StreamingNotSupported,

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}
```

## Testing Contract Compliance

Each provider implementation must pass this test suite:

```rust
#[cfg(test)]
mod provider_contract_tests {
    use super::*;

    async fn test_complete<P: LLMProvider>(provider: P) {
        let request = simple_test_request();
        let response = provider.complete(request).await.unwrap();

        assert!(response.content.is_some() || !response.tool_calls.is_empty());
        assert!(response.usage.total_tokens > 0);
    }

    async fn test_streaming<P: LLMProvider>(provider: P) {
        if !provider.supports_streaming() {
            return; // Skip if not supported
        }

        let request = simple_test_request();
        let mut stream = provider.complete_stream(request).await.unwrap();

        let mut chunks = Vec::new();
        while let Some(result) = stream.next().await {
            chunks.push(result.unwrap());
        }

        assert!(!chunks.is_empty());
        assert!(chunks.last().unwrap().usage.total_tokens > 0);
    }

    async fn test_token_estimation<P: LLMProvider>(provider: P) {
        let request = simple_test_request();
        let estimated = provider.estimate_tokens(&request);

        let response = provider.complete(request).await.unwrap();
        let actual = response.usage.total_tokens;

        // Within 10% accuracy (NFR-005)
        let error_pct = ((actual as f64 - estimated as f64) / actual as f64).abs() * 100.0;
        assert!(error_pct < 10.0, "Estimation error: {}%", error_pct);
    }
}
```

## Performance Requirements

Per NFR-001, provider abstraction overhead must be < 50ms per request:

- Token estimation: < 1ms
- Request normalization (LLMRequest → native format): < 10ms
- Response normalization (native → LLMResponse): < 10ms
- Rate limiter check: < 1ms
- Total overhead budget: < 25ms (buffer for variability)

## Concurrency Safety

All provider implementations must be:
- **Send**: Can be transferred between threads
- **Sync**: Can be accessed from multiple threads concurrently
- **Thread-safe**: Internal state (rate limiter, HTTP client) must use appropriate synchronization

Use `Arc<dyn LLMProvider>` for sharing across threads.
