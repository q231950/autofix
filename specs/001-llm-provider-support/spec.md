# Feature Specification: LLM Provider Support

**Feature Branch**: `001-llm-provider-support`
**Created**: 2025-12-12
**Status**: Draft
**Input**: User description: "LLM Provider: all LLMs as described in the constitution should be supported"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Use Claude AI for Test Fixing (Priority: P1)

As a developer, I want to use Anthropic Claude to automatically fix my failing iOS UI tests so that I can leverage the most capable model for complex test analysis.

**Why this priority**: Claude is the primary LLM provider and must work reliably as it's the reference implementation for all tool-based operations.

**Independent Test**: Can be fully tested by running `autofix fix --provider claude` with a failing test and verifying the fix succeeds using Claude's API.

**Acceptance Scenarios**:

1. **Given** I have configured ANTHROPIC_API_KEY, **When** I run autofix with a failing test, **Then** the tool uses Claude API to analyze and fix the test
2. **Given** Claude API returns a successful response, **When** autofix processes the fix, **Then** the test passes and I see Claude-specific rate limit information
3. **Given** I select Claude Sonnet model, **When** autofix makes API calls, **Then** token usage and costs reflect Sonnet pricing

---

### User Story 2 - Use OpenAI-Compatible Endpoints (Priority: P2)

As a developer, I want to configure autofix to use OpenAI or OpenAI-compatible APIs so that I can choose providers based on cost, privacy, or performance requirements.

**Why this priority**: Enables users who prefer OpenAI's models or want to use OpenAI-compatible services for cost or data privacy reasons.

**Independent Test**: Can be fully tested by configuring an OpenAI API key and endpoint, running autofix, and verifying it uses the OpenAI API successfully.

**Acceptance Scenarios**:

1. **Given** I have configured OPENAI_API_KEY, **When** I run autofix with --provider openai, **Then** the tool uses OpenAI API for test analysis
2. **Given** I specify a custom OpenAI-compatible endpoint, **When** autofix connects, **Then** it successfully communicates using OpenAI's API contract
3. **Given** OpenAI returns streaming responses, **When** autofix processes them, **Then** progress is displayed in real-time

---

### User Story 3 - Use Local LLM via Ollama (Priority: P3)

As a developer, I want to run autofix using local LLM models via Ollama so that I can work offline, reduce costs, and keep my code private.

**Why this priority**: Provides maximum flexibility and privacy, but is lower priority as local models may have reduced capability compared to cloud APIs.

**Independent Test**: Can be fully tested by starting Ollama locally with a compatible model, configuring autofix to use it, and verifying test fixes work offline.

**Acceptance Scenarios**:

1. **Given** Ollama is running locally, **When** I configure autofix with --provider ollama --endpoint http://localhost:11434, **Then** autofix uses the local model
2. **Given** no internet connection, **When** I run autofix with Ollama provider, **Then** test fixing works without network access
3. **Given** I select a specific Ollama model, **When** autofix runs, **Then** it uses that model and shows model-specific context limits

---

### User Story 4 - Switch Between Providers Seamlessly (Priority: P2)

As a developer, I want to switch between different LLM providers without changing my workflow so that I can optimize for cost, performance, or availability.

**Why this priority**: Essential for the LLM-agnostic architecture principle - users must be able to change providers easily.

**Independent Test**: Can be fully tested by running the same failing test with different providers and verifying consistent behavior across all.

**Acceptance Scenarios**:

1. **Given** I have multiple providers configured, **When** I switch provider via --provider flag, **Then** autofix works identically with different backend
2. **Given** one provider is experiencing outages, **When** I switch to another provider, **Then** my workflow continues uninterrupted
3. **Given** I'm testing locally with Ollama, **When** I switch to Claude for production use, **Then** the same tools and commands work without modification

---

### Edge Cases

- What happens when a provider's API is unavailable or returns errors?
- How does the system handle rate limiting differences between providers?
- What happens when a local Ollama model doesn't support required context length?
- How does autofix behave when API keys are missing or invalid?
- What happens when switching providers mid-operation?
- How does the system handle different response formats between providers?
- What happens when a provider doesn't support streaming responses?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support Anthropic Claude API as the primary LLM provider
- **FR-002**: System MUST support OpenAI API and OpenAI-compatible endpoints
- **FR-003**: System MUST support local LLM models via Ollama
- **FR-004**: Users MUST be able to select LLM provider via configuration or command-line flag
- **FR-005**: System MUST abstract provider-specific API details behind a common interface
- **FR-006**: System MUST handle rate limiting transparently across all providers
- **FR-007**: System MUST estimate and display token usage for all providers
- **FR-008**: System MUST provide clear error messages when provider configuration is invalid
- **FR-009**: System MUST allow provider-specific configuration (model selection, endpoint URLs, timeouts)
- **FR-010**: System MUST validate API responses from all providers before processing
- **FR-011**: System MUST handle network failures gracefully regardless of provider
- **FR-012**: System MUST support streaming responses where provider supports them
- **FR-013**: System MUST preserve existing tool-based architecture across all providers
- **FR-014**: System MUST not expose API keys in logs or error messages
- **FR-015**: System MUST allow users to specify custom endpoints for OpenAI-compatible services

### Non-Functional Requirements

- **NFR-001**: Provider abstraction MUST NOT introduce significant performance overhead (< 50ms per request)
- **NFR-002**: Adding new providers MUST NOT require changes to existing tool implementations
- **NFR-003**: Configuration MUST be simple and follow CLI best practices (environment variables + flags)
- **NFR-004**: Rate limiting implementation MUST prevent API throttling for all providers
- **NFR-005**: Token estimation MUST be accurate within 10% for cost tracking

### Key Entities

- **LLM Provider**: Represents a configured LLM service (Claude, OpenAI, Ollama) with connection details, credentials, and capabilities
- **Provider Configuration**: Settings for a specific provider including API endpoint, model name, rate limits, and timeout values
- **Rate Limiter**: Tracks token usage per provider and enforces delays to prevent throttling
- **API Response**: Normalized response from any provider containing generated text, token counts, and metadata

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can switch between any supported provider using a single command-line flag without workflow changes
- **SC-002**: All existing autofix features (create, fix standard mode, fix knight rider mode) work identically across all three provider types
- **SC-003**: Rate limiting prevents API throttling with 99% success rate across 100 consecutive test fix operations
- **SC-004**: Token usage estimation is accurate within 10% across all providers for cost tracking
- **SC-005**: Users can configure and use a new provider in under 2 minutes
- **SC-006**: Provider-specific errors include actionable guidance in 100% of cases
- **SC-007**: Local Ollama provider works offline with zero network dependencies
- **SC-008**: Switching providers does not require code changes or rebuild - only configuration updates

### Assumptions

- **A-001**: All LLM providers support a request/response pattern (streaming optional)
- **A-002**: OpenAI-compatible endpoints follow OpenAI's API contract sufficiently for basic operations
- **A-003**: Ollama provides an HTTP API compatible with typical LLM interaction patterns
- **A-004**: Users will have appropriate API keys or local setup before using autofix
- **A-005**: Rate limits can be configured via environment variables per provider
- **A-006**: All providers can handle the tool-based prompting pattern defined in the constitution
