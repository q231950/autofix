# Research: LLM Provider Support

**Date**: 2025-12-12
**Feature**: 001-llm-provider-support

## Overview

This document resolves all NEEDS CLARIFICATION items from the Technical Context and provides concrete technical decisions for implementing multi-provider LLM support.

## Research Findings

### 1. OpenAI Rust SDK

**Decision**: Use `async-openai` crate (version 0.20+)

**Rationale**:
- Most comprehensive and actively maintained OpenAI Rust client
- Full support for streaming responses via Server-Sent Events
- **Native support for custom endpoints** - enables OpenAI-compatible API integration
- Built-in exponential backoff retry for rate limits
- Supports both standard OpenAI and Azure OpenAI Service
- Compatible with Ollama's OpenAI-compatible endpoint (`/v1` path)

**Alternatives Considered**:
- `openai-api-rs`: Simpler but synchronous-only, doesn't fit our async architecture
- Direct HTTP with `reqwest`: More work, no built-in retry/streaming logic
- **Rejected**: Both lack the maturity and feature completeness of `async-openai`

**Implementation Pattern**:
```rust
use async_openai::{Client, config::OpenAIConfig};

// OpenAI
let config = OpenAIConfig::new()
    .with_api_key(env::var("OPENAI_API_KEY")?)
    .with_api_base("https://api.openai.com/v1");

// Custom endpoint (e.g., Azure, Together.ai, etc.)
let config = OpenAIConfig::new()
    .with_api_key(env::var("API_KEY")?)
    .with_api_base("https://custom-endpoint.com/v1");

let client = Client::with_config(config);
```

### 2. Ollama Client Library

**Decision**: Use `async-openai` with Ollama's OpenAI-compatible endpoint

**Rationale**:
- Ollama provides OpenAI-compatible API at `http://localhost:11434/v1`
- **One codebase** works for both OpenAI and Ollama by changing base URL
- Eliminates need for separate Ollama-specific client (`ollama-rs`)
- Reduces dependency count and maintenance burden
- Simplifies provider abstraction layer

**Alternatives Considered**:
- `ollama-rs` (version 0.1.6+): Native Ollama client with model management features
- **Rejected**: Adds complexity; OpenAI compatibility provides sufficient functionality for our use case

**Implementation Pattern**:
```rust
// Ollama using OpenAI-compatible endpoint
let config = OpenAIConfig::new()
    .with_api_base("http://localhost:11434/v1")
    .with_api_key("ollama"); // Dummy key (Ollama doesn't require it)

let client = Client::with_config(config);

// Use standard OpenAI API calls with Ollama model names
let request = CreateChatCompletionRequestArgs::default()
    .model("llama2")  // Ollama model name
    .messages(vec![...])
    .build()?;
```

**Note**: If advanced Ollama features (model pulling, management) are needed later, add `ollama-rs` as optional dependency.

### 3. Offline Capability for Ollama

**Decision**: Ollama runs fully offline after initial model download

**Confirmation**:
- **Model Download**: Requires internet connection initially via `ollama pull <model>`
- **Runtime**: Once downloaded, runs entirely locally with zero network dependencies
- **API**: Local HTTP server on `127.0.0.1:11434` (no external network calls)

**Implementation Requirements**:
- Document pre-download requirement in user guide
- Provide `ollama list` command to verify available models
- Handle errors gracefully when Ollama service not running
- No special offline handling needed in code - Ollama is always offline at runtime

**User Workflow**:
```bash
# One-time setup
ollama pull llama2
ollama serve

# Autofix works offline after this
autofix fix --provider ollama --test-result ...
```

### 4. Network Resilience Patterns

**Decision**: Use `reqwest-middleware` + `reqwest-retry` for HTTP retry logic

**Rationale**:
- Exponential backoff with configurable retry bounds
- Automatic retry for transient errors (5xx, timeouts, connection failures)
- Middleware pattern integrates cleanly with existing async architecture
- Works with both `async-openai` and Anthropic SDK's internal `reqwest` usage

**Dependencies**:
```toml
reqwest = { version = "0.11", features = ["json", "stream"] }
reqwest-middleware = "0.2"
reqwest-retry = "0.4"
```

**Implementation Pattern**:
```rust
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

let retry_policy = ExponentialBackoff::builder()
    .retry_bounds(Duration::from_secs(1), Duration::from_secs(60))
    .build_with_max_retries(3);

let client = ClientBuilder::new(reqwest::Client::new())
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build();
```

**Timeout Configuration**:
```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))              // Total request timeout
    .connect_timeout(Duration::from_secs(10))      // Connection timeout
    .pool_idle_timeout(Duration::from_secs(90))    // Connection pool idle
    .pool_max_idle_per_host(10)                    // Max idle connections
    .tcp_keepalive(Duration::from_secs(60))        // TCP keepalive
    .build()?;
```

**Connection Pooling**: `reqwest::Client` provides automatic connection pooling. Reuse single client instance across requests for efficiency.

**Alternatives Considered**:
- `backoff` crate: More manual control, but requires explicit retry loop implementation
- **Rejected**: `reqwest-retry` middleware is cleaner and integrates better with async code

### 5. API Key Security in Logs

**Decision**: Use `secrecy` crate for secret management

**Rationale**:
- Prevents accidental logging via Debug/Display trait implementation
- Auto-zeroes memory on drop (using `zeroize` crate internally)
- Explicit access via `ExposeSecret` trait - makes secret usage visible in code
- Compatible with serde for configuration deserialization

**Dependencies**:
```toml
secrecy = { version = "0.8", features = ["serde"] }
dotenvy = "0.15"  # For .env file support
```

**Implementation Pattern**:
```rust
use secrecy::{SecretString, ExposeSecret};

#[derive(Debug)]
pub struct ProviderConfig {
    pub api_key: SecretString,
    pub endpoint: String,
}

impl ProviderConfig {
    pub fn from_env(key_var: &str) -> Self {
        let api_key = env::var(key_var)
            .unwrap_or_default();

        Self {
            api_key: SecretString::new(api_key),
            endpoint: "https://api.openai.com/v1".to_string(),
        }
    }
}

// Debug output shows: ProviderConfig { api_key: Secret([REDACTED]), endpoint: "..." }
println!("{:?}", config);

// Explicit access when needed
let client = Client::with_api_key(config.api_key.expose_secret());
```

**Error Sanitization**:
```rust
// Strip API key patterns from error messages
fn sanitize_error(msg: &str) -> String {
    regex::Regex::new(r"sk-[a-zA-Z0-9-]+")
        .unwrap()
        .replace_all(msg, "[REDACTED]")
        .to_string()
}
```

**Alternatives Considered**:
- Manual redaction in error messages: Error-prone and easy to miss
- `zeroize` alone: Doesn't prevent Debug/Display output
- **Rejected**: `secrecy` provides comprehensive protection with minimal overhead

## Dependency Summary

### New Dependencies Required

```toml
[dependencies]
# LLM Clients
async-openai = "0.20"                              # OpenAI + OpenAI-compatible APIs

# Network Resilience
reqwest = { version = "0.11", features = ["json", "stream"] }
reqwest-middleware = "0.2"
reqwest-retry = "0.4"

# Security
secrecy = { version = "0.8", features = ["serde"] }
dotenvy = "0.15"

# Existing (keep)
anthropic-sdk-rust = "0.1.0"
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
clap = { version = "4.5.48", features = ["derive"] }
serde = { version = "1.0.221", features = ["derive"] }
serde_json = "1.0.143"
thiserror = "2.0.17"
```

### Optional Dependencies (Future Enhancement)

```toml
ollama-rs = "0.1.6"  # Only if native Ollama model management needed
backoff = "0.4"      # Only if more advanced retry strategies needed
```

## Technical Constraints Resolved

| Original Constraint | Resolution |
|---------------------|------------|
| **NEEDS CLARIFICATION: OpenAI Rust SDK** | `async-openai` version 0.20+ |
| **NEEDS CLARIFICATION: Ollama client library** | Use `async-openai` with Ollama's OpenAI-compatible endpoint |
| **NEEDS CLARIFICATION: Offline capability for Ollama** | Fully offline after initial model download via `ollama pull` |
| **NEEDS CLARIFICATION: Network resilience** | `reqwest-retry` with exponential backoff (1s-60s, max 3 retries) |
| **NEEDS CLARIFICATION: API key security in logs** | `secrecy` crate with `SecretString` type |

## Architecture Decision

**Provider Abstraction Strategy**:

Use a unified trait-based abstraction layer:
- Claude: Use existing `anthropic-sdk-rust` client
- OpenAI: Use `async-openai` with standard endpoint
- Ollama: Use `async-openai` with Ollama endpoint (`http://localhost:11434/v1`)

**Benefits**:
- Single provider trait interface
- Minimal code duplication (OpenAI + Ollama share client code)
- Easy to add new OpenAI-compatible providers (Together.ai, Groq, etc.)
- Existing tools (DirectoryInspectorTool, etc.) work unchanged with provider trait

## Next Steps

Proceed to Phase 1:
- Generate data-model.md (provider configuration entities)
- Generate contracts/ (provider trait interface)
- Generate quickstart.md (setup guide for each provider)
- Update agent context with new dependencies
