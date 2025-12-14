# Tasks: LLM Provider Support

**Input**: Design documents from `/specs/001-llm-provider-support/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are NOT explicitly requested in the specification, so NO test tasks are included.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root (Rust edition 2024)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Add new dependencies to Cargo.toml (async-openai 0.20, reqwest-middleware 0.2, reqwest-retry 0.4, secrecy 0.8, dotenvy 0.15)
- [X] T002 [P] Create src/llm/mod.rs module structure with provider trait export
- [X] T003 [P] Create src/llm/config.rs for provider configuration types

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Define ProviderType enum in src/llm/config.rs (Claude, OpenAI, Ollama variants)
- [X] T005 [P] Define ProviderConfig struct in src/llm/config.rs with SecretString api_key, api_base, model, timeout_secs, max_retries, rate_limit_tpm fields
- [X] T006 [P] Define LLMRequest struct in src/llm/mod.rs with system_prompt, messages, tools, max_tokens, temperature, stream fields
- [X] T007 [P] Define LLMResponse struct in src/llm/mod.rs with content, tool_calls, stop_reason, usage fields
- [X] T008 [P] Define Message struct in src/llm/mod.rs with role and content fields
- [X] T009 [P] Define MessageRole enum in src/llm/mod.rs (User, Assistant, Tool variants)
- [X] T010 [P] Define ToolDefinition struct in src/llm/mod.rs with name, description, input_schema fields
- [X] T011 [P] Define ToolCall struct in src/llm/mod.rs with id, name, input fields
- [X] T012 [P] Define TokenUsage struct in src/llm/mod.rs with input_tokens, output_tokens, total_tokens fields
- [X] T013 [P] Define StopReason enum in src/llm/mod.rs (EndTurn, MaxTokens, StopSequence, ToolUse, Error variants)
- [X] T014 [P] Define LLMError enum in src/llm/mod.rs using thiserror (AuthenticationError, RateLimitError, NetworkError, ServerError, InvalidRequest, StreamingNotSupported, ConfigurationError variants)
- [X] T015 Create LLMProvider trait in src/llm/provider_trait.rs with async_trait (new, provider_type, complete, complete_stream, estimate_tokens, validate_config, max_context_length, supports_streaming, supports_tools methods)
- [X] T016 Implement ProviderConfig::from_env() in src/llm/config.rs for loading configuration from environment variables (AUTOFIX_PROVIDER, ANTHROPIC_API_KEY, OPENAI_API_KEY, AUTOFIX_API_BASE, AUTOFIX_MODEL, AUTOFIX_TIMEOUT_SECS, AUTOFIX_MAX_RETRIES, AUTOFIX_RATE_LIMIT_TPM)
- [X] T017 [P] Implement ProviderConfig default values per provider in src/llm/config.rs (Claude: api.anthropic.com with claude-sonnet-4 and 30000 TPM, OpenAI: api.openai.com/v1 with gpt-4 and 90000 TPM, Ollama: localhost:11434/v1 with llama2 and unlimited TPM)
- [X] T018 [P] Create RateLimiter struct in src/rate_limiter.rs with provider_type, tokens_per_minute, window_start, tokens_used fields (refactor existing rate limiter to be provider-aware)
- [X] T019 Implement RateLimiter state transitions in src/rate_limiter.rs (reset window after 60s, consume tokens before request with wait logic, update with actual usage after response)
- [X] T020 Create ProviderFactory in src/llm/mod.rs with create() method that validates config and returns Box<dyn LLMProvider>

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Use Claude AI for Test Fixing (Priority: P1) 🎯 MVP

**Goal**: Enable Autofix to use Anthropic Claude API for test analysis and fixing with proper rate limiting and error handling

**Independent Test**: Run `autofix fix --provider claude --test-result <path>` with ANTHROPIC_API_KEY configured and verify test fix succeeds using Claude API

### Implementation for User Story 1

- [X] T021 [P] [US1] Create ClaudeProvider struct in src/llm/claude_provider.rs with config, client, rate_limiter fields
- [X] T022 [US1] Implement LLMProvider::new() for ClaudeProvider in src/llm/claude_provider.rs (validate config, create anthropic-sdk-rust client with retry middleware)
- [X] T023 [US1] Implement LLMProvider::validate_config() for ClaudeProvider in src/llm/claude_provider.rs (check api_key required, endpoint must be HTTPS, model must be valid Claude model)
- [X] T024 [US1] Implement LLMProvider::provider_type() for ClaudeProvider in src/llm/claude_provider.rs (return ProviderType::Claude)
- [X] T025 [US1] Implement LLMRequest to Claude native format conversion in src/llm/claude_provider.rs (convert messages, tools, max_tokens, temperature)
- [X] T026 [US1] Implement Claude native response to LLMResponse conversion in src/llm/claude_provider.rs (normalize content, tool_calls, stop_reason, usage)
- [X] T027 [US1] Implement LLMProvider::complete() for ClaudeProvider in src/llm/claude_provider.rs (check rate limiter, send request with retry logic, normalize response, update rate limiter with actual usage)
- [X] T028 [US1] Implement LLMProvider::complete_stream() for ClaudeProvider in src/llm/claude_provider.rs (stream Server-Sent Events, yield chunks as LLMResponse, final chunk includes complete TokenUsage)
- [X] T029 [US1] Implement LLMProvider::estimate_tokens() for ClaudeProvider in src/llm/claude_provider.rs (4 chars per token heuristic, add tool schema overhead, add max_tokens estimate)
- [X] T030 [US1] Implement LLMProvider::max_context_length() for ClaudeProvider in src/llm/claude_provider.rs (return 200000 for claude-sonnet-4 and claude-haiku-3.5)
- [X] T031 [US1] Implement LLMProvider::supports_streaming() for ClaudeProvider in src/llm/claude_provider.rs (return true)
- [X] T032 [US1] Implement LLMProvider::supports_tools() for ClaudeProvider in src/llm/claude_provider.rs (return true)
- [ ] T033 [US1] Refactor existing DirectoryInspectorTool in src/tools/directory_inspector_tool.rs to use Box<dyn LLMProvider> trait instead of direct anthropic-sdk-rust calls
- [ ] T034 [US1] Refactor existing CodeEditorTool in src/tools/code_editor_tool.rs to use Box<dyn LLMProvider> trait instead of direct anthropic-sdk-rust calls
- [ ] T035 [US1] Refactor existing TestRunnerTool in src/tools/test_runner_tool.rs to use Box<dyn LLMProvider> trait instead of direct anthropic-sdk-rust calls
- [ ] T036 [US1] Update src/autofix_command.rs to instantiate provider via ProviderFactory::create() based on config
- [ ] T037 [US1] Update src/pipeline/autofix_pipeline.rs to accept Box<dyn LLMProvider> instead of hardcoded Claude client
- [ ] T038 [US1] Add error sanitization for API keys in error messages in src/llm/claude_provider.rs (strip sk-ant-* patterns, use SecretString::expose_secret only when needed)
- [ ] T039 [US1] Update src/main.rs CLI to add --provider flag (claude, openai, ollama) with claude as default

**Checkpoint**: At this point, User Story 1 should be fully functional - Claude provider works with all existing autofix features

---

## Phase 4: User Story 2 - Use OpenAI-Compatible Endpoints (Priority: P2)

**Goal**: Enable Autofix to use OpenAI or OpenAI-compatible APIs (Together.ai, Groq, Azure) with the same workflow

**Independent Test**: Run `autofix fix --provider openai --test-result <path>` with OPENAI_API_KEY configured and verify test fix succeeds using OpenAI API

### Implementation for User Story 2

- [ ] T040 [P] [US2] Create OpenAIProvider struct in src/llm/openai_provider.rs with config, client (async-openai Client), rate_limiter fields
- [ ] T041 [US2] Implement LLMProvider::new() for OpenAIProvider in src/llm/openai_provider.rs (validate config, create async-openai Client with custom endpoint via OpenAIConfig::new().with_api_base())
- [ ] T042 [US2] Implement LLMProvider::validate_config() for OpenAIProvider in src/llm/openai_provider.rs (check api_key required, endpoint must be valid HTTP/HTTPS URL, model name validation)
- [ ] T043 [US2] Implement LLMProvider::provider_type() for OpenAIProvider in src/llm/openai_provider.rs (return ProviderType::OpenAI)
- [ ] T044 [US2] Implement LLMRequest to OpenAI native format conversion in src/llm/openai_provider.rs (convert messages using CreateChatCompletionRequestArgs, map tools to function calling format)
- [ ] T045 [US2] Implement OpenAI native response to LLMResponse conversion in src/llm/openai_provider.rs (normalize ChatCompletionResponse to LLMResponse, map stop reasons: stop→EndTurn, length→MaxTokens, tool_calls→ToolUse)
- [ ] T046 [US2] Implement LLMProvider::complete() for OpenAIProvider in src/llm/openai_provider.rs (check rate limiter, call client.chat().create() with retry middleware, normalize response, update rate limiter)
- [ ] T047 [US2] Implement LLMProvider::complete_stream() for OpenAIProvider in src/llm/openai_provider.rs (use client.chat().create_stream(), yield chunks as LLMResponse, accumulate for final TokenUsage)
- [ ] T048 [US2] Implement LLMProvider::estimate_tokens() for OpenAIProvider in src/llm/openai_provider.rs (same 4 chars per token heuristic, add tool overhead, add max_tokens)
- [ ] T049 [US2] Implement LLMProvider::max_context_length() for OpenAIProvider in src/llm/openai_provider.rs (return 128000 for gpt-4-turbo, 8192 for gpt-4, allow custom model context via config)
- [ ] T050 [US2] Implement LLMProvider::supports_streaming() for OpenAIProvider in src/llm/openai_provider.rs (return true)
- [ ] T051 [US2] Implement LLMProvider::supports_tools() for OpenAIProvider in src/llm/openai_provider.rs (return true)
- [ ] T052 [US2] Update ProviderFactory::create() in src/llm/mod.rs to handle ProviderType::OpenAI and instantiate OpenAIProvider
- [ ] T053 [US2] Add AUTOFIX_API_BASE support to ProviderConfig::from_env() in src/llm/config.rs for custom OpenAI-compatible endpoints (Together.ai, Groq, Azure OpenAI)

**Checkpoint**: At this point, User Stories 1 AND 2 should both work - users can switch between Claude and OpenAI providers seamlessly

---

## Phase 5: User Story 3 - Use Local LLM via Ollama (Priority: P3)

**Goal**: Enable Autofix to use local Ollama models for offline, private, cost-free test fixing

**Independent Test**: Start Ollama locally with `ollama serve`, run `autofix fix --provider ollama --test-result <path>`, verify test fix works offline without network access

### Implementation for User Story 3

- [ ] T054 [P] [US3] Create OllamaProvider struct in src/llm/ollama_provider.rs with config, client (reuse async-openai Client with Ollama endpoint), rate_limiter fields
- [ ] T055 [US3] Implement LLMProvider::new() for OllamaProvider in src/llm/ollama_provider.rs (create async-openai Client with api_base=http://localhost:11434/v1, use dummy api_key since Ollama doesn't require authentication)
- [ ] T056 [US3] Implement LLMProvider::validate_config() for OllamaProvider in src/llm/ollama_provider.rs (api_key optional, endpoint must be http://localhost:*, model must be non-empty)
- [ ] T057 [US3] Implement LLMProvider::provider_type() for OllamaProvider in src/llm/ollama_provider.rs (return ProviderType::Ollama)
- [ ] T058 [US3] Implement LLMRequest to Ollama format conversion in src/llm/ollama_provider.rs (reuse OpenAI format since Ollama is OpenAI-compatible, use Ollama model names like llama2, codellama, mistral)
- [ ] T059 [US3] Implement Ollama response to LLMResponse conversion in src/llm/ollama_provider.rs (same as OpenAI normalization, handle limited stop reasons: stop, length)
- [ ] T060 [US3] Implement LLMProvider::complete() for OllamaProvider in src/llm/ollama_provider.rs (check rate limiter if configured, call client.chat().create(), normalize response, update rate limiter - skip rate limiting if rate_limit_tpm is 0 for unlimited local usage)
- [ ] T061 [US3] Implement LLMProvider::complete_stream() for OllamaProvider in src/llm/ollama_provider.rs (use streaming endpoint, yield chunks, handle potentially incomplete streaming support depending on model)
- [ ] T062 [US3] Implement LLMProvider::estimate_tokens() for OllamaProvider in src/llm/ollama_provider.rs (same heuristic as other providers)
- [ ] T063 [US3] Implement LLMProvider::max_context_length() for OllamaProvider in src/llm/ollama_provider.rs (return 4096 for llama2, 16384 for codellama, 32768 for mistral, allow model-specific mapping)
- [ ] T064 [US3] Implement LLMProvider::supports_streaming() for OllamaProvider in src/llm/ollama_provider.rs (return true)
- [ ] T065 [US3] Implement LLMProvider::supports_tools() for OllamaProvider in src/llm/ollama_provider.rs (return false by default - model-dependent, can be enhanced later)
- [ ] T066 [US3] Update ProviderFactory::create() in src/llm/mod.rs to handle ProviderType::Ollama and instantiate OllamaProvider
- [ ] T067 [US3] Add graceful error handling for Ollama connection failures in src/llm/ollama_provider.rs (detect connection refused, provide actionable error: "Ollama not running. Start with: ollama serve")
- [ ] T068 [US3] Add graceful error handling for missing Ollama models in src/llm/ollama_provider.rs (detect model not found error, provide actionable guidance: "Model not found. Pull with: ollama pull <model>")

**Checkpoint**: All three providers (Claude, OpenAI, Ollama) now work independently and can be switched via configuration

---

## Phase 6: User Story 4 - Switch Between Providers Seamlessly (Priority: P2)

**Goal**: Enable users to switch between providers without code changes, only configuration updates

**Independent Test**: Run the same failing test with `--provider claude`, `--provider openai`, and `--provider ollama`, verify consistent behavior across all providers

### Implementation for User Story 4

- [ ] T069 [P] [US4] Add CLI flag parsing in src/main.rs for --provider (default: claude from config or env AUTOFIX_PROVIDER)
- [ ] T070 [P] [US4] Add CLI flag parsing in src/main.rs for --model to override default model per provider
- [ ] T071 [P] [US4] Create .env.example file at repository root with sample configuration for all three providers (AUTOFIX_PROVIDER, ANTHROPIC_API_KEY, OPENAI_API_KEY, AUTOFIX_API_BASE, AUTOFIX_MODEL, AUTOFIX_RATE_LIMIT_TPM, AUTOFIX_TIMEOUT_SECS, AUTOFIX_MAX_RETRIES)
- [ ] T072 [US4] Update src/main.rs to load .env file using dotenvy::dotenv() at startup
- [ ] T073 [US4] Implement provider display in verbose output in src/autofix_command.rs (show provider type, model, endpoint, rate limit status when --verbose flag is used)
- [ ] T074 [US4] Add token usage display in verbose output in src/pipeline/autofix_pipeline.rs (show input_tokens, output_tokens, total_tokens per request when --verbose)
- [ ] T075 [US4] Add rate limit status display in verbose output in src/rate_limiter.rs (show tokens_used, tokens_per_minute, time until window reset when --verbose)
- [ ] T076 [US4] Validate that all three providers work with existing DirectoryInspectorTool, CodeEditorTool, TestRunnerTool through provider trait abstraction
- [ ] T077 [US4] Ensure switching providers mid-workflow is handled gracefully (new provider instance created, old rate limiter state doesn't interfere)

**Checkpoint**: Users can now seamlessly switch between any provider using CLI flags or environment variables without workflow changes

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and final quality checks

- [ ] T078 [P] Run cargo clippy to check for warnings across all new llm/ module files
- [ ] T079 [P] Run cargo fmt to format all code per Rust 2024 edition standards
- [ ] T080 [P] Add documentation comments (///) to all public types and methods in src/llm/ modules
- [ ] T081 [P] Verify all error messages are user-friendly and actionable (no raw API errors exposed)
- [ ] T082 [P] Validate SecretString usage prevents API key leakage in Debug output and logs
- [ ] T083 [P] Verify performance overhead is < 50ms per request per NFR-001 (measure token estimation, request normalization, response normalization times)
- [ ] T084 Update README.md with provider configuration instructions (how to set API keys, switch providers, configure rate limits)
- [ ] T085 Update CLAUDE.md with new Rust modules added (src/llm/, provider types, configuration)
- [ ] T086 [P] Verify all providers handle network failures gracefully with retry logic (test with timeout scenarios, connection refused)
- [ ] T087 Run quickstart.md validation for all three providers (Claude setup, OpenAI setup, Ollama setup)
- [ ] T088 Final smoke test across all user stories (US1: Claude works, US2: OpenAI works, US3: Ollama works offline, US4: switching works)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User Story 1 (Claude) can start after Foundational
  - User Story 2 (OpenAI) can start after Foundational (independent of US1, but US1 establishes pattern)
  - User Story 3 (Ollama) can start after Foundational (reuses OpenAI pattern, so logically after US2)
  - User Story 4 (Seamless Switching) depends on US1, US2, US3 being complete
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (US1 - Claude, P1)**: Can start after Foundational - No dependencies on other stories, establishes the provider pattern
- **User Story 2 (US2 - OpenAI, P2)**: Can start after Foundational - Logically after US1 to reuse provider trait pattern
- **User Story 3 (US3 - Ollama, P3)**: Can start after Foundational - Logically after US2 as it reuses OpenAI client library
- **User Story 4 (US4 - Seamless Switching, P2)**: Requires US1, US2, US3 complete - Tests integration across all providers

### Within Each User Story

**User Story 1 (Claude)**:
- T021 (ClaudeProvider struct) before all other US1 tasks
- T022-T024 (new, validate_config, provider_type) can be parallel
- T025-T026 (conversions) can be parallel
- T027-T032 (trait methods) can be parallel after T025-T026
- T033-T035 (tool refactoring) can be parallel after T027
- T036-T039 (pipeline integration) sequential after T033-T035

**User Story 2 (OpenAI)**:
- T040 (OpenAIProvider struct) before all other US2 tasks
- T041-T043 (new, validate_config, provider_type) can be parallel
- T044-T045 (conversions) can be parallel
- T046-T051 (trait methods) can be parallel after T044-T045
- T052-T053 (factory and config updates) after T046-T051

**User Story 3 (Ollama)**:
- T054 (OllamaProvider struct) before all other US3 tasks
- T055-T057 (new, validate_config, provider_type) can be parallel
- T058-T059 (conversions) can be parallel
- T060-T065 (trait methods) can be parallel after T058-T059
- T066-T068 (factory and error handling) after T060-T065

**User Story 4 (Seamless Switching)**:
- T069-T071 (CLI flags and .env) can be parallel
- T072 (dotenv loading) after T071
- T073-T075 (verbose output) can be parallel after T072
- T076-T077 (validation) after T073-T075

### Parallel Opportunities

**Phase 1 (Setup)**:
- T002 and T003 can run in parallel

**Phase 2 (Foundational)**:
- T005-T014 (all struct/enum definitions) can run in parallel
- T017 and T018 can run in parallel with data structures

**Phase 3 (User Story 1 - Claude)**:
- T022-T024 can run in parallel
- T025-T026 can run in parallel
- T027-T032 can run in parallel (after T025-T026 complete)
- T033-T035 can run in parallel (after T027 complete)

**Phase 4 (User Story 2 - OpenAI)**:
- T041-T043 can run in parallel
- T044-T045 can run in parallel
- T046-T051 can run in parallel (after T044-T045 complete)

**Phase 5 (User Story 3 - Ollama)**:
- T055-T057 can run in parallel
- T058-T059 can run in parallel
- T060-T065 can run in parallel (after T058-T059 complete)

**Phase 6 (User Story 4 - Seamless Switching)**:
- T069-T071 can run in parallel
- T073-T075 can run in parallel

**Phase 7 (Polish)**:
- T078-T082 can run in parallel
- T086-T087 can run in parallel

---

## Parallel Example: User Story 1 (Claude Provider)

```bash
# Launch all basic implementation tasks together:
Task: T022 "Implement LLMProvider::new() for ClaudeProvider"
Task: T023 "Implement LLMProvider::validate_config() for ClaudeProvider"
Task: T024 "Implement LLMProvider::provider_type() for ClaudeProvider"

# Launch all conversion tasks together:
Task: T025 "Implement LLMRequest to Claude native format conversion"
Task: T026 "Implement Claude native response to LLMResponse conversion"

# Launch all trait methods together (after conversions):
Task: T027 "Implement LLMProvider::complete() for ClaudeProvider"
Task: T028 "Implement LLMProvider::complete_stream() for ClaudeProvider"
Task: T029 "Implement LLMProvider::estimate_tokens() for ClaudeProvider"
Task: T030 "Implement LLMProvider::max_context_length() for ClaudeProvider"
Task: T031 "Implement LLMProvider::supports_streaming() for ClaudeProvider"
Task: T032 "Implement LLMProvider::supports_tools() for ClaudeProvider"

# Launch all tool refactoring together (after complete() works):
Task: T033 "Refactor DirectoryInspectorTool to use provider trait"
Task: T034 "Refactor CodeEditorTool to use provider trait"
Task: T035 "Refactor TestRunnerTool to use provider trait"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T020) - CRITICAL, blocks all stories
3. Complete Phase 3: User Story 1 - Claude Provider (T021-T039)
4. **STOP and VALIDATE**: Test autofix with Claude provider independently
5. Deploy/demo if ready - this is the MVP!

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 (Claude) → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 (OpenAI) → Test independently → Deploy/Demo
4. Add User Story 3 (Ollama) → Test independently → Deploy/Demo
5. Add User Story 4 (Seamless Switching) → Test all providers → Deploy/Demo
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together (T001-T020)
2. Once Foundational is done:
   - Developer A: User Story 1 (Claude) - T021-T039
   - Developer B: User Story 2 (OpenAI) - T040-T053 (starts slightly after A establishes pattern)
   - Developer C: User Story 3 (Ollama) - T054-T068 (starts after B shows OpenAI pattern)
3. Once US1-3 complete:
   - Developer A: User Story 4 (Seamless Switching) - T069-T077
4. All developers: Polish (T078-T088)

---

## Task Count Summary

- **Phase 1 (Setup)**: 3 tasks
- **Phase 2 (Foundational)**: 17 tasks (BLOCKS all user stories)
- **Phase 3 (User Story 1 - Claude)**: 19 tasks
- **Phase 4 (User Story 2 - OpenAI)**: 14 tasks
- **Phase 5 (User Story 3 - Ollama)**: 15 tasks
- **Phase 6 (User Story 4 - Seamless Switching)**: 9 tasks
- **Phase 7 (Polish)**: 11 tasks

**Total**: 88 tasks

**Parallel Opportunities**:
- Setup: 2 tasks can run in parallel (T002, T003)
- Foundational: 12 tasks can run in parallel (struct/enum definitions)
- User Story 1: 15 tasks have parallel opportunities within phases
- User Story 2: 12 tasks have parallel opportunities within phases
- User Story 3: 12 tasks have parallel opportunities within phases
- User Story 4: 6 tasks have parallel opportunities
- Polish: 8 tasks can run in parallel

**MVP Scope**: Phase 1 + Phase 2 + Phase 3 = 39 tasks (User Story 1 - Claude provider only)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- No tests included as not explicitly requested in specification
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Focus on User Story 1 (Claude) for MVP - provides immediate value while establishing the provider pattern for other providers
