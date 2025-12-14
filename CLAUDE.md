# autofix Development Guidelines

Auto-generated from all feature plans. Last updated: 2025-12-12

## Active Technologies
- Rust (edition 2024 per CLAUDE.md) + GitHub Actions workflows, Rust toolchain with aarch64-apple-darwin targe (002-release-build-artifacts)
- N/A (GitHub handles artifact storage) (002-release-build-artifacts)

- Rust edition 2024 (current project standard)
- LLM Provider Abstraction:
  - anthropic-sdk (v0.2+) for Claude/Anthropic API
  - async-openai (v0.20+) for OpenAI and compatible APIs
  - async-trait for trait-based provider abstraction
  - secrecy crate for API key protection
  - dotenvy for .env file configuration

## Project Structure

```text
src/
├── llm/                    # LLM provider abstraction
│   ├── mod.rs              # Core types, factory, error types
│   ├── provider_trait.rs   # LLMProvider trait definition
│   ├── config.rs           # Provider configuration & env loading
│   ├── claude_provider.rs  # Anthropic Claude implementation
│   ├── openai_provider.rs  # OpenAI API implementation
│   └── ollama_provider.rs  # Ollama local models implementation
├── pipeline/               # Autofix pipeline logic
├── tools/                  # LLM agent tools
├── rate_limiter.rs         # Provider-aware rate limiting
└── ...
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust edition 2024 (current project standard): Follow standard conventions

## Recent Changes
- 002-release-build-artifacts: Added Rust (edition 2024 per CLAUDE.md) + GitHub Actions workflows, Rust toolchain with aarch64-apple-darwin targe

- 001-llm-provider-support: Comprehensive LLM provider abstraction
  - Added support for Claude, OpenAI, and Ollama providers
  - Implemented LLMProvider trait for provider-agnostic interface
  - Added ProviderFactory for runtime provider instantiation
  - Provider-aware rate limiting with per-provider defaults
  - Configuration via environment variables or .env file
  - Pipeline refactored to use provider abstraction
  - CLI flags: --provider, --model for runtime configuration

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
