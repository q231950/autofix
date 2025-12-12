# Data Model: LLM Provider Support

**Date**: 2025-12-12
**Feature**: 001-llm-provider-support

## Overview

This document defines the core entities and their relationships for multi-provider LLM support in Autofix.

## Core Entities

### 1. ProviderType

**Purpose**: Enum representing supported LLM provider types.

**Fields**:
- `Claude`: Anthropic Claude API
- `OpenAI`: OpenAI API or OpenAI-compatible endpoints
- `Ollama`: Local Ollama models

**Validation Rules**:
- Must be one of the three defined variants
- Case-insensitive string parsing from CLI flags
- Default: `Claude` (primary provider per constitution)

**String Representation**:
- `"claude"` → `ProviderType::Claude`
- `"openai"` → `ProviderType::OpenAI`
- `"ollama"` → `ProviderType::Ollama`

**State Transitions**: N/A (immutable enum)

---

### 2. ProviderConfig

**Purpose**: Configuration settings for a specific LLM provider instance.

**Fields**:
| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `provider_type` | `ProviderType` | Yes | `Claude` | Which provider this config is for |
| `api_key` | `SecretString` | Conditional | None | API key (required for Claude/OpenAI, optional for Ollama) |
| `api_base` | `String` | No | Provider-specific | Base URL for API endpoint |
| `model` | `String` | No | Provider-specific | Model name to use |
| `timeout_secs` | `u64` | No | `30` | Request timeout in seconds |
| `max_retries` | `u32` | No | `3` | Maximum retry attempts for transient failures |
| `rate_limit_tpm` | `Option<u32>` | No | Provider-specific | Tokens per minute rate limit |

**Validation Rules**:
- `api_key` must not be empty for Claude/OpenAI providers
- `api_base` must be valid HTTP(S) URL
- `timeout_secs` must be between 5 and 600 seconds
- `max_retries` must be between 0 and 10
- `rate_limit_tpm` must be positive if set

**Default Values by Provider**:

| Provider | `api_base` | `model` | `rate_limit_tpm` |
|----------|------------|---------|------------------|
| Claude | `https://api.anthropic.com` | `claude-sonnet-4` | `30000` |
| OpenAI | `https://api.openai.com/v1` | `gpt-4` | `90000` |
| Ollama | `http://localhost:11434/v1` | `llama2` | `null` (unlimited) |

**Environment Variable Mapping**:
```
AUTOFIX_PROVIDER           → provider_type
ANTHROPIC_API_KEY          → api_key (if provider=claude)
OPENAI_API_KEY             → api_key (if provider=openai)
AUTOFIX_API_BASE           → api_base (overrides default)
AUTOFIX_MODEL              → model (overrides default)
AUTOFIX_TIMEOUT_SECS       → timeout_secs
AUTOFIX_MAX_RETRIES        → max_retries
AUTOFIX_RATE_LIMIT_TPM     → rate_limit_tpm
```

**State Transitions**: Immutable after construction from environment/CLI

---

### 3. LLMRequest

**Purpose**: Normalized request structure sent to any LLM provider.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `system_prompt` | `Option<String>` | No | System instruction (role context) |
| `messages` | `Vec<Message>` | Yes | Conversation history |
| `tools` | `Vec<ToolDefinition>` | No | Available tools (DirectoryInspector, CodeEditor, TestRunner) |
| `max_tokens` | `Option<u32>` | No | Maximum tokens to generate |
| `temperature` | `Option<f32>` | No | Sampling temperature (0.0-1.0) |
| `stream` | `bool` | No | Enable streaming responses (default: false) |

**Validation Rules**:
- `messages` must not be empty
- `messages` must alternate between user/assistant (provider-dependent)
- `max_tokens` must be positive if set
- `temperature` must be between 0.0 and 1.0 if set
- `tools` definitions must match provider capabilities

**Relationships**:
- Contains `Vec<Message>` (composition)
- Contains `Vec<ToolDefinition>` (composition)

---

### 4. Message

**Purpose**: Single message in a conversation.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `role` | `MessageRole` | Yes | Who sent this message |
| `content` | `String` | Yes | Message text content |

**MessageRole** Enum:
- `User`: Message from user/system
- `Assistant`: Message from LLM
- `Tool`: Tool execution result (provider-dependent)

**Validation Rules**:
- `content` must not be empty
- `role` must be valid enum variant

---

### 5. ToolDefinition

**Purpose**: Defines a tool available to the LLM.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `String` | Yes | Tool identifier (e.g., "directory_inspector") |
| `description` | `String` | Yes | What the tool does |
| `input_schema` | `serde_json::Value` | Yes | JSON schema for tool parameters |

**Validation Rules**:
- `name` must be snake_case alphanumeric
- `description` must be non-empty
- `input_schema` must be valid JSON Schema

**Existing Tools**:
1. **directory_inspector**: File exploration, reading, searching patterns
2. **code_editor**: Exact string replacement editing
3. **test_runner**: Build and test execution

---

### 6. LLMResponse

**Purpose**: Normalized response structure from any LLM provider.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `content` | `Option<String>` | Conditional | Generated text (if not tool call) |
| `tool_calls` | `Vec<ToolCall>` | Conditional | Tools the LLM wants to invoke |
| `stop_reason` | `StopReason` | Yes | Why generation stopped |
| `usage` | `TokenUsage` | Yes | Token consumption metrics |

**Validation Rules**:
- Either `content` or `tool_calls` must be present (not both empty)
- `usage` must have valid token counts

**Relationships**:
- Contains `Vec<ToolCall>` (composition)
- Contains `TokenUsage` (composition)

---

### 7. ToolCall

**Purpose**: LLM's request to invoke a tool.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `String` | Yes | Unique identifier for this tool call |
| `name` | `String` | Yes | Tool to invoke (matches ToolDefinition.name) |
| `input` | `serde_json::Value` | Yes | Tool parameters (matches input_schema) |

**Validation Rules**:
- `id` must be unique within a response
- `name` must match an available ToolDefinition
- `input` must conform to tool's input_schema

---

### 8. TokenUsage

**Purpose**: Token consumption metrics for cost tracking.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `input_tokens` | `u32` | Yes | Tokens in request (prompt + context) |
| `output_tokens` | `u32` | Yes | Tokens in response (generated text) |
| `total_tokens` | `u32` | Yes | Sum of input + output tokens |

**Validation Rules**:
- `total_tokens` must equal `input_tokens + output_tokens`
- All fields must be non-negative

**Purpose**: Used for rate limiting and cost estimation per NFR-005 (10% accuracy requirement)

---

### 9. StopReason

**Purpose**: Enum indicating why LLM generation stopped.

**Variants**:
- `EndTurn`: Natural completion point
- `MaxTokens`: Hit token limit
- `StopSequence`: Encountered stop sequence
- `ToolUse`: Wants to invoke tools
- `Error`: Generation error occurred

**Mapping to Providers**:
| Provider | Stop Reasons |
|----------|--------------|
| Claude | `end_turn`, `max_tokens`, `stop_sequence`, `tool_use` |
| OpenAI | `stop`, `length`, `tool_calls`, `content_filter` |
| Ollama | `stop`, `length` (limited set) |

---

### 10. RateLimiter

**Purpose**: Tracks token usage and enforces rate limits per provider.

**Fields**:
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `provider_type` | `ProviderType` | Yes | Which provider this limiter is for |
| `tokens_per_minute` | `u32` | Yes | Configured TPM limit |
| `window_start` | `Instant` | Yes | Start of current rate limit window |
| `tokens_used` | `u32` | Yes | Tokens consumed in current window |

**Validation Rules**:
- `tokens_used` must not exceed `tokens_per_minute` before request
- Window resets every 60 seconds
- Estimation accuracy within 10% (NFR-005)

**State Transitions**:
1. **Reset**: When 60 seconds elapsed since `window_start`
   - Set `window_start` = now
   - Set `tokens_used` = 0
2. **Consume**: Before each request
   - Estimate request tokens
   - If `tokens_used + estimate > tokens_per_minute`: Wait until reset
   - Add estimate to `tokens_used`
3. **Update**: After response received
   - Replace estimate with actual `usage.total_tokens`

**Relationships**:
- One RateLimiter per ProviderConfig instance
- Updated by TokenUsage from LLMResponse

---

## Entity Relationship Diagram

```
ProviderConfig
├── provider_type: ProviderType (enum)
├── api_key: SecretString
└── rate_limiter: RateLimiter
        ├── provider_type: ProviderType
        ├── tokens_per_minute: u32
        ├── window_start: Instant
        └── tokens_used: u32

LLMRequest
├── system_prompt: Option<String>
├── messages: Vec<Message>
│       ├── role: MessageRole (enum)
│       └── content: String
└── tools: Vec<ToolDefinition>
        ├── name: String
        ├── description: String
        └── input_schema: JSON Schema

LLMResponse
├── content: Option<String>
├── tool_calls: Vec<ToolCall>
│       ├── id: String
│       ├── name: String
│       └── input: JSON
├── stop_reason: StopReason (enum)
└── usage: TokenUsage
        ├── input_tokens: u32
        ├── output_tokens: u32
        └── total_tokens: u32
```

## Data Flow

1. **Configuration Loading**:
   - Environment variables → `ProviderConfig`
   - CLI flags override environment defaults
   - `ProviderConfig` instantiates provider-specific client

2. **Request Processing**:
   - Application creates `LLMRequest`
   - `RateLimiter` estimates and validates token budget
   - Provider-specific client converts `LLMRequest` to native API format
   - Network call with retry logic

3. **Response Processing**:
   - Provider-specific response → normalized `LLMResponse`
   - `TokenUsage` updates `RateLimiter` state
   - Application processes `content` or `tool_calls`

4. **Tool Execution**:
   - `ToolCall` → Tool implementation (DirectoryInspector, CodeEditor, TestRunner)
   - Tool result → new `Message` with role=Tool
   - Loop until `stop_reason` is not `ToolUse`

## Serialization Formats

All entities support JSON serialization via `serde`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig { /* fields */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest { /* fields */ }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse { /* fields */ }
```

**Purpose**: Enable configuration files, debugging, and testing.

## Security Considerations

- `api_key` field uses `SecretString` type (prevents Debug output)
- Logs must sanitize error messages containing API responses
- Environment variables cleared after loading into `ProviderConfig`
- No API keys persisted to disk

## Performance Considerations

- `RateLimiter` uses monotonic `Instant` (no clock skew issues)
- Token estimation cached per request type (NFR-001: < 50ms overhead)
- Connection pooling via `reqwest::Client` (reuse HTTP connections)
- Streaming responses processed incrementally (no full buffer required)
