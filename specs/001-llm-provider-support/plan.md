# Implementation Plan: LLM Provider Support

**Branch**: `001-llm-provider-support` | **Date**: 2025-12-12 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-llm-provider-support/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement comprehensive LLM provider support enabling Autofix to work with Anthropic Claude (primary), OpenAI-compatible APIs, and local Ollama models. Users must be able to seamlessly switch between providers via configuration without workflow changes, while maintaining the existing tool-based architecture across all providers.

## Technical Context

**Language/Version**: Rust edition 2024 (current project standard)
**Primary Dependencies**: anthropic-sdk-rust (existing), async-openai v0.20+ (OpenAI/Ollama via OpenAI-compatible endpoint), reqwest-retry, secrecy
**Storage**: N/A (stateless CLI tool with environment-based configuration)
**Testing**: cargo test (Rust standard testing framework)
**Target Platform**: macOS with Xcode command-line tools (existing)
**Project Type**: single (CLI application at repository root)
**Performance Goals**: < 50ms provider abstraction overhead per request, < 5s response time for simple operations
**Constraints**: Ollama fully offline after model download, reqwest-retry for network resilience (1s-60s exponential backoff), secrecy crate prevents API key logging
**Scale/Scope**: Support 3 provider types, handle 100+ consecutive operations without throttling

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle III: LLM-Agnostic Architecture

✅ **PASS** - This feature directly implements the constitution requirement:
- Will support Claude API as primary provider
- Will support OpenAI-compatible endpoints
- Will abstract provider selection through configuration
- Will handle rate limiting transparently across providers

### Principle IV: Tool-Based Intelligence

⚠️ **ATTENTION REQUIRED** - Existing tools (DirectoryInspectorTool, CodeEditorTool, TestRunnerTool) must work with all providers:
- Tools currently use anthropic-sdk-rust directly
- Must refactor to use provider abstraction layer
- Tool contracts must remain unchanged
- Verification needed: Do existing tools assume Claude-specific features?

### Principle V: Verification-First

✅ **PASS** - Provider switching does not affect verification logic:
- Verification loop (build & test) is independent of LLM provider
- All providers will use same tool-based verification

### Quality Standards: Performance

✅ **PASS** - NFR-001 specifies < 50ms overhead, within acceptable CLI interaction standards

### Quality Standards: Reliability

✅ **PASS** - Requirements include graceful network failure handling and response validation

### Quality Standards: Security

✅ **PASS** - FR-014 mandates no API key exposure in logs

### **GATE RESULT: CONDITIONAL PASS**

**Action Required Before Phase 0**:
1. Verify existing tool implementations don't have Claude-specific assumptions
2. Research how to abstract provider without breaking tool contracts

## Project Structure

### Documentation (this feature)

```text
specs/001-llm-provider-support/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs                                    # Entry point (existing)
├── autofix_command.rs                         # Autofix command handler (existing)
├── test_command.rs                            # Test command handler (existing)
├── rate_limiter.rs                            # Rate limiting (existing)
├── llm/                                       # NEW: LLM provider abstraction
│   ├── mod.rs                                 # Provider trait and factory
│   ├── provider_trait.rs                      # Common LLM provider interface
│   ├── claude_provider.rs                     # Anthropic Claude implementation
│   ├── openai_provider.rs                     # OpenAI/compatible implementation
│   ├── ollama_provider.rs                     # Ollama local model implementation
│   └── config.rs                              # Provider configuration
├── pipeline/                                  # Existing pipeline logic
│   ├── mod.rs
│   ├── autofix_pipeline.rs
│   └── prompts.rs
├── tools/                                     # Existing tools (refactor to use provider trait)
│   ├── directory_inspector_tool.rs
│   ├── code_editor_tool.rs
│   └── test_runner_tool.rs
├── xc_test_result_attachment_handler.rs       # Existing
├── xc_workspace_file_locator.rs               # Existing
├── xcresultparser.rs                          # Existing
└── xctestresultdetailparser.rs                # Existing

tests/
├── integration/                               # NEW: Integration tests
│   ├── provider_switching_test.rs             # Test switching between providers
│   ├── claude_provider_test.rs                # Claude-specific integration test
│   ├── openai_provider_test.rs                # OpenAI integration test
│   └── ollama_provider_test.rs                # Ollama integration test
└── unit/                                      # NEW: Unit tests
    ├── provider_config_test.rs                # Configuration parsing tests
    └── rate_limiter_test.rs                   # Rate limiting tests per provider
```

**Structure Decision**: Using Option 1 (Single project) as this is a standalone CLI tool. Added new `llm/` module for provider abstraction and expanded tests structure to validate multi-provider support.

## Complexity Tracking

> **No violations detected** - Feature aligns with all constitution principles and quality standards.
