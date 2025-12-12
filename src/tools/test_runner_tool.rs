use crate::xctestresultdetailparser::{XCTestResultDetail, XCTestResultDetailParser};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunnerTool {
    name: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunnerInput {
    pub operation: String,
    pub test_identifier: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunnerResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_detail: Option<XCTestResultDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xcresult_path: Option<PathBuf>,
}

impl TestRunnerTool {
    pub fn new() -> Self {
        Self {
            name: "test_runner".to_string(),
            description: r#"A tool to run iOS UI tests to validate fixes.

Operation:
- "test": Runs the specific UI test to check if it passes

Input format:
{
  "operation": "test",
  "test_identifier": "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample"
}

The test_identifier format is: test://com.apple.xcode/{scheme}/{target}/{class}/{method}

Returns exit code, stdout, stderr, success status, and detailed test failure information if the test fails."#.to_string(),
        }
    }

    pub fn to_tool_definition(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["test"],
                        "description": "The operation to perform: test"
                    },
                    "test_identifier": {
                        "type": "string",
                        "description": "Full test identifier URL"
                    }
                },
                "required": ["operation", "test_identifier"]
            }
        })
    }

    pub fn execute(&self, input: TestRunnerInput, workspace_root: &Path) -> TestRunnerResult {
        match input.operation.as_str() {
            "test" => self.run_test(&input.test_identifier, workspace_root),
            _ => TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!(
                    "Unknown operation: {}. Only 'test' is supported.",
                    input.operation
                ),
                test_detail: None,
                xcresult_path: None,
            },
        }
    }

    fn parse_test_identifier(&self, test_identifier: &str) -> Option<(String, String)> {
        // Parse test://com.apple.xcode/{scheme}/{target}/{class}/{method}
        if !test_identifier.starts_with("test://") {
            return None;
        }

        let parts: Vec<&str> = test_identifier
            .strip_prefix("test://")
            .unwrap_or("")
            .split('/')
            .collect();

        if parts.len() < 4 {
            return None;
        }

        // Skip "com.apple.xcode" and get scheme, rest
        let scheme = parts.get(1)?.to_string();
        let full_test = parts[2..].join("/");

        Some((scheme, full_test))
    }

    fn run_test(&self, test_identifier: &str, workspace_root: &Path) -> TestRunnerResult {
        let (scheme, full_test) = match self.parse_test_identifier(test_identifier) {
            Some(parsed) => parsed,
            None => {
                return TestRunnerResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: String::new(),
                    message: format!("Invalid test identifier format: {}", test_identifier),
                    test_detail: None,
                    xcresult_path: None,
                };
            }
        };

        // Create temporary directories for this test run
        let uuid = Uuid::new_v4();
        let temp_base = workspace_root
            .join(".autofix/test-runner-tool")
            .join(uuid.to_string());
        let build_dir = temp_base.join("build");
        let test_dir = temp_base.join("test");

        // Create directories
        if let Err(e) = fs::create_dir_all(&build_dir) {
            return TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!("Failed to create build directory: {}", e),
                test_detail: None,
                xcresult_path: None,
            };
        }

        if let Err(e) = fs::create_dir_all(&test_dir) {
            return TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!("Failed to create test directory: {}", e),
                test_detail: None,
                xcresult_path: None,
            };
        }

        let result_bundle_path = test_dir.join("result.xcresult");

        let output = Command::new("xcodebuild")
            .arg("test")
            .arg("-scheme")
            .arg(&scheme)
            .arg("-destination")
            .arg("platform=iOS Simulator,name=iPhone 17 Pro")
            .arg(format!("-only-testing:{}", full_test))
            .arg("-derivedDataPath")
            .arg(&build_dir)
            .arg("-resultBundlePath")
            .arg(&result_bundle_path)
            .current_dir(workspace_root)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                // If test failed, parse the xcresult to get detailed failure information
                let (test_detail, xcresult_path) = if !success && result_bundle_path.exists() {
                    let parser = XCTestResultDetailParser::new();
                    match parser.parse(&result_bundle_path, test_identifier) {
                        Ok(detail) => (Some(detail), Some(result_bundle_path.clone())),
                        Err(e) => {
                            eprintln!("Failed to parse xcresult: {}", e);
                            (None, Some(result_bundle_path.clone()))
                        }
                    }
                } else {
                    (
                        None,
                        if result_bundle_path.exists() {
                            Some(result_bundle_path.clone())
                        } else {
                            None
                        },
                    )
                };

                TestRunnerResult {
                    success,
                    exit_code,
                    stdout: stdout.clone(),
                    stderr: stderr.clone(),
                    message: if success {
                        format!("Test passed: {}", full_test)
                    } else {
                        format!("Test failed: {} (exit code: {})", full_test, exit_code)
                    },
                    test_detail,
                    xcresult_path,
                }
            }
            Err(e) => TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!("Failed to execute xcodebuild: {}", e),
                test_detail: None,
                xcresult_path: None,
            },
        }
    }
}

impl Default for TestRunnerTool {
    fn default() -> Self {
        Self::new()
    }
}
