use crate::xctestresultdetailparser::XCTestResultDetail;
use std::path::Path;

/// Generate the prompt for Knight Rider mode (autonomous fixing with tools)
pub fn generate_knightrider_prompt(
    detail: &XCTestResultDetail,
    test_file_contents: &str,
    workspace_path: &Path,
    has_snapshot: bool,
) -> String {
    format!(
        r#"I am analyzing a failed iOS UI test and need you to AUTOMATICALLY FIX IT using the provided tools.

**Failed Test:** {}
**Test Identifier:** {}
**Workspace Path:** {}

**Test File Contents:**
```swift
{}
```

{}

CRITICAL ASSUMPTION: THE TEST IS THE SOURCE OF TRUTH
- The test code is correct and should NOT be modified
- The application code needs to be fixed to match what the test expects
- You are fixing the app to pass the test, not adjusting the test to pass

YOUR TASK: Use the available tools to automatically fix the APPLICATION CODE. You should:

1. Use `directory_inspector` to explore the codebase and understand the app structure
2. Use `directory_inspector` to read the app source files that the test interacts with
3. Analyze the test to understand what it expects from the application
4. Identify what's missing or incorrect in the APPLICATION CODE
5. Use `code_editor` to make necessary changes to APPLICATION SOURCE CODE ONLY
6. Use `test_runner` with operation "test" to verify the test now passes

IMPORTANT INSTRUCTIONS:
- DO NOT modify any test files - only modify application source code
- You MUST use the tools to make actual changes to the application code
- Make targeted, minimal changes to fix the specific test failure
- After each code change, test to verify (testing also compiles the code)
- If the first fix doesn't work, iterate and try different approaches
- Common fixes needed in app code:
  * Add missing UI elements that the test expects
  * Add accessibility identifiers to UI elements so tests can find them
  * Fix incorrect labels, text, or button titles
  * Ensure proper view hierarchy and element visibility
  * Add missing navigation or view transitions

The test identifier format is: {}
Use this full identifier when calling test_runner."#,
        detail.test_name,
        detail.test_identifier_url,
        workspace_path.display(),
        test_file_contents,
        if has_snapshot {
            "**Simulator Snapshot:** I've attached the latest simulator screenshot showing the state when the test failed."
        } else {
            "**Note:** No simulator snapshot was available for this test."
        },
        detail.test_identifier_url
    )
}

/// Generate the prompt for standard mode (fix test code, optionally add accessibility to app)
pub fn generate_standard_prompt(
    detail: &XCTestResultDetail,
    test_file_contents: &str,
    workspace_path: &Path,
    has_snapshot: bool,
) -> String {
    format!(
        r#"I am analyzing a failed iOS UI test and need you to AUTOMATICALLY FIX IT using the provided tools.

**Failed Test:** {}
**Test Identifier:** {}
**Workspace Path:** {}

**Test File Contents:**
```swift
{}
```

{}

ASSUMPTION: THE APPLICATION CODE IS CORRECT
- The application is working as intended and should generally NOT be modified
- The test code needs to be adjusted to match the actual application behavior
- You may add accessibility identifiers to the app code ONLY if necessary for test discoverability

YOUR TASK: Use the available tools to automatically fix the TEST CODE. You should:

1. Use `directory_inspector` to explore the codebase and locate the test file
2. Use `directory_inspector` to read the test file and understand the test logic
3. Analyze the test to understand what it's trying to do
4. Identify what's wrong with the TEST CODE
5. Use `code_editor` to make necessary changes to the TEST FILE
6. If elements cannot be found, use `directory_inspector` to find the relevant app code
7. If needed, use `code_editor` to add accessibility identifiers to APP CODE (minimal changes only)
8. Use `test_runner` with operation "test" to verify the test now passes

IMPORTANT INSTRUCTIONS:
- Primary focus: Fix the TEST code to work with the current app
- Only modify APP code if you need to add accessibility identifiers for element discovery
- Make targeted, minimal changes to fix the specific test failure
- After each code change, test to verify (testing also compiles the code)
- If the first fix doesn't work, iterate and try different approaches
- Common fixes needed in test code:
  * Update selectors to match actual UI elements
  * Add proper waits/expectations for async operations
  * Update assertion VALUES to match current app (e.g., "Login" → "Sign In")
  * Update element queries to use correct identifiers
  * Handle animations and transitions properly
- When to update assertions:
  * App copy/text changed: Update expected strings
  * UI reorganization: Update expected element counts or positions
  * Design changes: Update expected properties (labels, button text, etc.)
  * ALWAYS explain what changed and why the assertion was updated
- If adding accessibility to app:
  * Use `.accessibilityIdentifier("...")` in SwiftUI
  * Use `element.accessibilityIdentifier = "..."` in UIKit
  * Keep identifier names clear and test-friendly

CRITICAL RULES ABOUT TEST ASSERTIONS:
- NEVER delete or comment out test assertions (XCTAssert*, XCTFail, etc.)
- NEVER remove test expectations or verification code
- You MAY update assertion values to match the current app behavior
- Assertions validate important app behavior - they must remain active
- Common assertion updates needed:
  * Update expected text/labels if app copy changed (e.g., "Login" → "Sign In")
  * Update expected counts if UI elements were reorganized
  * Update expected properties if design changed (e.g., button placement)
- If an assertion needs to be updated, make the change and explain why
- The assertion itself must stay - only the expected VALUES can change

GIVE UP POLICY:
- If you attempt to fix the test/app code 2 times and the assertion still fails in unexpected ways
- STOP and provide a final message with this exact format:

  GIVING UP: Unable to fix assertion failure after 2 attempts
  Failed assertion: [exact line of code from test file]
  File: [absolute file path starting from workspace]
  Line: [line number]
  Reason: [brief explanation of what you tried]

- Provide the FULL absolute path to the test file (e.g., {}/path/to/TestFile.swift)
- Provide the exact LINE NUMBER where the assertion appears
- This will automatically open Xcode at the failing assertion for manual review
- DO NOT make any more code changes after giving up
- DO NOT try alternative approaches beyond the 2 attempts

The test identifier format is: {}
Use this full identifier when calling test_runner."#,
        detail.test_name,
        detail.test_identifier_url,
        workspace_path.display(),
        test_file_contents,
        if has_snapshot {
            "**Simulator Snapshot:** I've attached the latest simulator screenshot showing the state when the test failed."
        } else {
            "**Note:** No simulator snapshot was available for this test."
        },
        workspace_path.display(),
        detail.test_identifier_url
    )
}
