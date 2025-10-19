# 🤖 Autofix ✨

An autonomous AI agent that automatically fixes failing iOS UI tests using Claude AI and intelligent code analysis tools.

## 🎯 Overview

Autofix analyzes failed iOS UI tests, explores your codebase, and autonomously makes code changes to fix the failures. It can work in two modes depending on whether you want to fix your application code or your test code.

### Key Features

- 🔍 **Intelligent Test Analysis**: Parses XCTest results and identifies failure details
- 🖼️ **Visual Context**: Analyzes simulator screenshots to understand UI state
- 🛠️ **Autonomous Code Editing**: Makes targeted code changes automatically
- ✅ **Verification Loop**: Builds and runs tests to verify fixes work
- 🎭 **Dual Modes**: Fix app code OR test code based on your needs
- 🔧 **Tool-Based Architecture**: Uses specialized tools for inspection, editing, and testing

## 📦 Installation

### Prerequisites

- Rust (edition 2024)
- Xcode and `xcodebuild` command-line tools
- Anthropic API key

### Build from Source

```bash
git clone <repository-url>
cd autofix
cargo build --release
```

### Environment Setup

Set your Anthropic API key:

```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

#### Optional: Rate Limiting Configuration

Autofix includes smart rate limiting to prevent hitting Anthropic's API limits. Configure these environment variables:

```bash
# Maximum input tokens per minute (default: 50000)
export ANTHROPIC_RATE_LIMIT_TPM=50000

# Enable/disable rate limiting (default: true)
export ANTHROPIC_RATE_LIMIT_ENABLED=true
```

**How it works:**
- Autofix estimates token usage before each API request
- If the request would exceed your per-minute limit, it automatically waits
- The tool displays a message when waiting: `⏸️ Rate limit approaching. Waiting X seconds...`
- Adjust `ANTHROPIC_RATE_LIMIT_TPM` based on your API tier:
  - Free tier: Lower limits (check Anthropic docs)
  - Claude Sonnet 4.x: 30,000 tokens/minute (default tier)
  - Claude Haiku 3.5: 50,000 tokens/minute
  - Higher tiers: Increase as needed

**Tip:** Set `ANTHROPIC_RATE_LIMIT_ENABLED=false` to disable rate limiting entirely if you have unlimited access or want to handle rate limits manually.

## 🚀 Usage

### Standard Mode (Fix Test Code)

Assumes your **app is correct** and the **test needs adjustment**:

```bash
autofix --ios \
  --test-result path/to/test.xcresult \
  --workspace path/to/workspace
```

**What it does:**
- ✅ Analyzes test failures
- ✅ Fixes test code (selectors, waits, expectations)
- ✅ Adds accessibility identifiers to app code (for testability)
- ✅ Verifies fixes by running tests

### Knight Rider Mode (Fix App Code)

Assumes your **test is correct** and the **app needs fixing**:

```bash
autofix --ios \
  --test-result path/to/test.xcresult \
  --workspace path/to/workspace \
  --knightrider
```

**What it does:**
- ✅ Treats test as source of truth
- ✅ Fixes application source code only
- ✅ Adds missing UI elements, labels, identifiers
- ✅ Never modifies test files

### Test a Specific Test

Get detailed analysis for a single test:

```bash
autofix test --ios \
  --test-result path/to/test.xcresult \
  --workspace path/to/workspace \
  --test-id "test://com.apple.xcode/MyApp/MyTests/MyTests/testExample"
```

## 🎭 Mode Comparison

| Mode | Assumption | Primary Target | Can Modify App? | Can Modify Test? |
|------|------------|----------------|-----------------|------------------|
| **Standard** (default) | App is correct | Fix test code | ✅ Yes (accessibility) | ✅ Yes |
| **Knight Rider** (`--knightrider`) | Test is correct | Fix app code | ✅ Yes (only this) | ❌ No |

## 🛠️ How It Works

### Architecture

Autofix uses a multi-stage pipeline:

1. **Attachment Fetching**: Extracts screenshots and attachments from `.xcresult` bundles
2. **Test File Location**: Finds the Swift test file in your workspace
3. **AI Analysis**: Claude analyzes the failure with visual context
4. **Autonomous Fixing** (with tools):
   - `DirectoryInspectorTool`: Explores codebase, reads files, searches for patterns
   - `CodeEditorTool`: Makes precise code edits via string replacement
   - `TestRunnerTool`: Builds and runs tests to verify fixes

### Example Workflow

```
┌─────────────────────────────────────┐
│  Failed Test: testLoginButton()    │
│  Error: Button not found           │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  🤖 Autofix analyzes screenshot     │
│  Sees button exists visually        │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  🔍 Explores codebase               │
│  Finds LoginView.swift              │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  ✏️ Adds accessibility ID            │
│  Button("Login")                    │
│    .accessibilityIdentifier("...")  │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  🧪 Runs test                       │
│  ✅ Test passes!                    │
└─────────────────────────────────────┘
```

## 📂 Project Structure

```
autofix/
├── src/
│   ├── main.rs                          # CLI entry point
│   ├── pipeline/                        # Core pipeline logic
│   │   ├── mod.rs                       # Module declarations
│   │   ├── autofix_pipeline.rs          # Pipeline implementation
│   │   └── prompts.rs                   # AI prompt generation
│   ├── tools/                           # AI agent tools
│   │   ├── directory_inspector_tool.rs  # File exploration
│   │   ├── code_editor_tool.rs          # Code editing
│   │   └── test_runner_tool.rs          # Build & test execution
│   ├── autofix_command.rs               # Process all failed tests
│   ├── test_command.rs                  # Single test processing
│   ├── xcresultparser.rs                # Parse XCResult bundles
│   ├── xctestresultdetailparser.rs      # Parse test details
│   ├── xc_test_result_attachment_handler.rs  # Extract attachments
│   └── xc_workspace_file_locator.rs     # Locate test files
├── Cargo.toml
└── README.md
```

## 🔧 Tools

Autofix provides Claude AI with three specialized tools:

### DirectoryInspectorTool
- **Operations**: `list`, `read`, `search`, `find`
- **Purpose**: Explore workspace, read files, search for patterns
- **Example**: Find all Swift files with a specific class

### CodeEditorTool
- **Operation**: Exact string replacement
- **Purpose**: Make targeted code edits
- **Safety**: Validates old content exists before replacing

### TestRunnerTool
- **Operations**: `build`, `test`
- **Purpose**: Compile code and run specific tests
- **Output**: Exit codes, stdout, stderr for verification

## 📊 Example Output

```bash
🤖 Knight Rider iteration 1...

💭 Claude says:
I'll explore the codebase to understand the app structure and locate the relevant view files.

🔧 Tool call: directory_inspector (id: toolu_123)
   Input: {"operation": "list", "path": "MyApp"}

🔧 Tool call: directory_inspector (id: toolu_456)
   Input: {"operation": "read", "path": "MyApp/Views/LoginView.swift"}

🤖 Knight Rider iteration 2...

💭 Claude says:
I found the issue. The button exists but lacks an accessibility identifier.

🔧 Tool call: code_editor (id: toolu_789)
   Input: {...}
   ✏️ Edit result: Successfully edited file: MyApp/Views/LoginView.swift

🔧 Tool call: test_runner (id: toolu_abc)
   Input: {"operation": "test", "test_identifier": "..."}
   🧪 Test result: Test passed (exit code: 0)
   ✅ SUCCESS!

✓ Knight Rider finished!
```

## 🧪 Development

### Run Tests

```bash
cargo test
```

### Run with Debug Logging

```bash
RUST_LOG=debug cargo run -- --ios --test-result ... --workspace ...
```

### Build for Release

```bash
cargo build --release
./target/release/autofix --help
```

## 🎯 Common Use Cases

### 1. Missing Accessibility Identifiers

**Problem**: Test can't find UI elements
**Solution**: Autofix adds `.accessibilityIdentifier()` to views

### 2. Incorrect Test Selectors

**Problem**: Test uses wrong element query
**Solution**: Autofix updates test to use correct selector

### 3. Timing Issues

**Problem**: Test fails due to animation/loading
**Solution**: Autofix adds proper wait conditions

### 4. Wrong Assertions

**Problem**: Test expects incorrect text/state
**Solution**: Autofix updates test assertions

### 5. Missing UI Elements

**Problem**: App missing button/label test expects
**Solution**: (Knight Rider mode) Autofix adds missing elements to app

## ⚠️ Limitations

- iOS/Xcode projects only (Android support planned)
- Requires `xcodebuild` command-line tools
- Works best with structured, well-named code
- May need multiple iterations for complex fixes
- Requires valid Anthropic API key

## 🤝 Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

## 📄 License

[GPL-3.0](LICENSE)

## 🙏 Acknowledgments

- Built with [Anthropic Claude](https://anthropic.com) AI
- Uses [anthropic-sdk-rust](https://github.com/dimichgh/anthropic-sdk-rust)
- Inspired by the need for better UI test maintenance

---

**Made with ❤️ and 🤖 AI**
