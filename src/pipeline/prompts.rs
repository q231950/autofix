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
6. Use `test_runner` with operation "build" to verify your changes compile
7. Use `test_runner` with operation "test" to verify the test now passes

IMPORTANT INSTRUCTIONS:
- DO NOT modify any test files - only modify application source code
- You MUST use the tools to make actual changes to the application code
- Make targeted, minimal changes to fix the specific test failure
- After each code change, build and test to verify
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

/// Generate the prompt for standard mode (analysis and suggestions)
pub fn generate_standard_prompt(
    detail: &XCTestResultDetail,
    test_file_contents: &str,
    has_snapshot: bool,
) -> String {
    format!(
        r#"I am analyzing a failed iOS UI test and need your help to find a possible solution.

**Failed Test:** {}

**Test File Contents:**
```swift
{}
```

{}

ASSUMPTION: THE APPLICATION CODE IS CORRECT
- The application is working as intended
- The test code may need to be adjusted to match the actual application behavior
- You may need to suggest adding accessibility identifiers to the app code to help the test locate elements

Please analyze the failed test and the simulator snapshot (if available) to:
1. Identify what might have caused the test to fail
2. Suggest fixes to the TEST CODE to make it pass
3. If elements can't be found, suggest adding accessibility identifiers to the APP CODE
4. Provide specific code changes for both test adjustments and accessibility improvements

Focus on common UI test issues and solutions:
- **Element not found**: Suggest correct selectors or accessibility identifiers to add to app code
- **Timing issues**: Suggest adding proper waits or expectations to test code
- **Incorrect selectors**: Provide corrected XCUIElement queries in test code
- **Race conditions**: Suggest test code improvements for handling animations/transitions
- **Assertion failures**: Suggest adjusting test expectations or fixing test logic
- **Missing accessibility**: Suggest adding `.accessibilityIdentifier()` to app's SwiftUI views or setting `accessibilityIdentifier` on UIKit elements"#,
        detail.test_name,
        test_file_contents,
        if has_snapshot {
            "**Simulator Snapshot:** I've attached the latest simulator screenshot showing the state when the test failed."
        } else {
            "**Note:** No simulator snapshot was available for this test."
        }
    )
}
