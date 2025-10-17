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

YOUR TASK: Use the available tools to automatically fix this test. You should:

1. Use `directory_inspector` to explore the codebase and understand the app structure
2. Use `directory_inspector` to read relevant source files that the test interacts with
3. Identify the root cause of the test failure
4. Use `code_editor` to make necessary code changes to fix the issue
5. Use `test_runner` with operation "build" to verify your changes compile
6. Use `test_runner` with operation "test" to verify the test now passes

IMPORTANT INSTRUCTIONS:
- You MUST use the tools to make actual changes to the code
- Make targeted, minimal changes to fix the specific test failure
- After each code change, build and test to verify
- If the first fix doesn't work, iterate and try different approaches
- Focus on fixing the app code or test code based on what's actually wrong
- Common issues: missing UI elements, incorrect accessibility IDs, timing/race conditions, wrong assertions

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

Please analyze the failed test and the simulator snapshot (if available) to:
1. Identify what might have caused the test to fail
2. Suggest possible solutions or fixes to make the test pass
3. Provide specific code changes if applicable

Focus on common UI test issues like:
- Element not found or timing issues
- Incorrect selectors or accessibility identifiers
- Race conditions or animations
- UI state mismatches
- Assertion failures"#,
        detail.test_name,
        test_file_contents,
        if has_snapshot {
            "**Simulator Snapshot:** I've attached the latest simulator screenshot showing the state when the test failed."
        } else {
            "**Note:** No simulator snapshot was available for this test."
        }
    )
}
