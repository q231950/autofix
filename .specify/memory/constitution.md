<!--
Sync Impact Report
==================
Version Change: N/A → 1.0.0 (Initial constitution)
Rationale: First version of constitution establishing foundational principles

Modified Principles: N/A (initial creation)
Added Sections:
  - Core Principles (6 principles)
  - Development Constraints
  - Quality Standards
  - Governance

Removed Sections: N/A

Templates Status:
  ✅ .specify/templates/plan-template.md - Reviewed, compatible
  ✅ .specify/templates/spec-template.md - Reviewed, compatible
  ✅ .specify/templates/tasks-template.md - Reviewed, compatible
  ✅ commands/speckit.constitution.md - Reviewed, compatible

Follow-up TODOs:
  - RATIFICATION_DATE needs to be set when officially adopted by team
-->

# Autofix Constitution

## Core Principles

### I. Dual-Command Architecture

Autofix MUST provide two primary commands with distinct responsibilities:

- **Create Command**: Interactive dialog-driven UI test generation
  - Guides users through test scenarios step-by-step
  - Generates working tests that pass on first run when possible
  - Prioritizes user control and understanding over speed

- **Fix Command**: Autonomous test failure resolution
  - Operates in two modes with different source-of-truth assumptions
  - Analyzes failures using visual and structural context
  - Automatically verifies fixes work before completion

Commands MUST have clear boundaries and not overlap in functionality. The create command is interactive; the fix command is autonomous.

**Rationale**: UI test workflows have two distinct phases: creation (requires human guidance) and maintenance (can be automated). Separating these as distinct commands provides clarity and appropriate UX for each use case.

### II. Fix Dual-Mode Operation

The fix command MUST support two operational modes with clear behavioral contracts:

- **Standard Mode** (default): Assumes application code is correct
  - Modifies test code to make tests pass
  - May add accessibility identifiers to app code for testability
  - Fixes test selectors, waits, and assertions

- **Knight Rider Mode** (`--knightrider` flag): Assumes test code is correct
  - Modifies application code to make tests pass
  - Never modifies test files
  - Adds missing UI elements, labels, identifiers to app

Each mode MUST maintain its assumptions consistently. Mixing behaviors within a single fix operation is prohibited.

**Rationale**: UI tests can fail because tests are wrong OR because apps are wrong. Supporting both fix modes enables developers to choose which source of truth to maintain based on their development workflow.

### III. LLM-Agnostic Architecture

The Autofix tool MUST support multiple LLM providers to ensure flexibility and avoid vendor lock-in:

- MUST support Anthropic Claude API as primary provider
- MUST support OpenAI-compatible endpoints (including local models via Ollama)
- MUST abstract LLM provider selection through configuration
- MUST handle rate limiting and token budgets transparently across providers
- New features MUST NOT assume a specific LLM provider

**Rationale**: Users need flexibility to choose LLM providers based on cost, privacy, and availability requirements. Supporting local models enables offline testing and reduced costs.

### IV. Tool-Based Intelligence

All LLM-driven operations MUST use structured tools rather than unguided prompting:

- **DirectoryInspectorTool**: File exploration, reading, searching patterns
  - Operations: `list`, `read`, `search`, `find`

- **CodeEditorTool**: Exact string replacement editing
  - Validates old content exists before replacing
  - Atomic file updates

- **TestRunnerTool**: Build and test execution
  - Operations: `build`, `test`
  - Captures exit codes, stdout, stderr

All new capabilities MUST be exposed as tools with clear input/output contracts. Tools MUST be composable and reusable across different LLM providers.

**Rationale**: Structured tools provide reliable, predictable behavior regardless of LLM provider. They enable testing, debugging, and incremental improvement of capabilities.

### V. Verification-First (NON-NEGOTIABLE)

Every code modification MUST be verified before considering the task complete:

- After editing test code → run the specific test
- After editing app code → build and run affected tests
- Test MUST pass (exit code 0) to consider fix successful
- Failures MUST trigger additional analysis and retry

No fix is complete without verification. The verification loop MUST be automatic and non-optional.

**Rationale**: Unverified code changes create false confidence and waste developer time. Autofix's value proposition depends on producing working fixes, not plausible-looking code.

### VI. Context-Driven Analysis

UI test analysis MUST incorporate multiple forms of context when available:

- MUST extract and analyze screenshots from `.xcresult` bundles when available
- MUST parse and utilize textual view hierarchy information from `.xcresult` bundles when available
- MUST use visual or structural context to understand actual UI state vs expected state
- MUST prefer concrete evidence (screenshots, view hierarchies) over code inference when diagnosing failures
- MUST preserve and reference context attachments in analysis

**Rationale**: UI tests often fail due to subtle differences in visual presentation or view hierarchy structure. Screenshots and view hierarchy dumps provide ground truth that code alone cannot reveal. Supporting both formats ensures comprehensive analysis regardless of available data.

## Development Constraints

### Technology Stack

- **Language**: Rust (edition 2024)
- **Primary Dependencies**: anthropic-sdk-rust, serde, tokio
- **Target Platform**: macOS with Xcode command-line tools
- **Build System**: Cargo
- **Testing**: Rust standard testing framework

### iOS Integration Requirements

- MUST use `xcodebuild` for all build and test operations
- MUST parse `.xcresult` bundles for test results and attachments
- MUST locate Swift test files within workspace structure
- MUST support standard Xcode project layouts

### Configuration Management

- API keys via environment variables (`ANTHROPIC_API_KEY`)
- Rate limiting configurable via environment variables
- Model selection configurable (Claude Sonnet, Haiku, etc.)
- Verbose mode for debugging (`--verbose` flag)

## Quality Standards

### Performance

- Rate limiting MUST prevent API throttling
- Token usage MUST be estimated before requests
- Large files MUST be handled efficiently (streaming, chunking)
- Response times MUST be acceptable for CLI interaction (< 5s for simple operations)

### Reliability

- MUST handle network failures gracefully
- MUST validate API responses before processing
- MUST provide clear error messages with actionable guidance
- MUST not corrupt source files on failure

### Observability

- Verbose mode MUST log all tool executions with inputs/outputs
- Token usage and rate limit status MUST be visible
- LLM conversation output MUST always be printed
- Progress indicators for long-running operations

### Security

- MUST NOT log or expose API keys
- MUST validate file paths to prevent directory traversal
- MUST sanitize user input before passing to shell commands
- MUST respect file permissions

## Governance

### Amendment Process

1. Propose constitution changes via pull request
2. Document rationale and impact on existing features
3. Update version according to semantic versioning rules
4. Propagate changes to dependent templates
5. Require team review and approval

### Versioning Policy

- **MAJOR**: Breaking changes to operational modes, tool contracts, or CLI interface
- **MINOR**: New principles, constraints, or quality standards
- **PATCH**: Clarifications, typo fixes, or non-semantic refinements

### Compliance

All feature development MUST verify compliance with constitution principles:

- Design reviews MUST reference relevant principles
- Pull requests MUST document principle adherence
- Breaking changes MUST be explicitly justified

This constitution supersedes all other development practices. When in doubt, refer to these principles.

**Version**: 1.0.0 | **Ratified**: 2025-12-12 | **Last Amended**: 2025-12-12
