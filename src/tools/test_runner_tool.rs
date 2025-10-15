use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

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
}

impl TestRunnerTool {
    pub fn new() -> Self {
        Self {
            name: "test_runner".to_string(),
            description: r#"A tool to build and run iOS UI tests to validate fixes.

Operations:
- "build": Compiles the project to check if code changes are valid
- "test": Runs the specific UI test to check if it passes

Input format:
{
  "operation": "build|test",
  "test_identifier": "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample"
}

The test_identifier format is: test://com.apple.xcode/{scheme}/{target}/{class}/{method}

For build: Uses the target component (e.g., AutoFixSamplerUITests)
For test: Uses the scheme component and full identifier

Returns exit code, stdout, stderr, and success status."#.to_string(),
        }
    }

    pub fn to_anthropic_tool(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": {
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["build", "test"],
                        "description": "The operation to perform: build or test"
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
            "build" => self.build_project(&input.test_identifier, workspace_root),
            "test" => self.run_test(&input.test_identifier, workspace_root),
            _ => TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!("Unknown operation: {}", input.operation),
            },
        }
    }

    fn parse_test_identifier(&self, test_identifier: &str) -> Option<(String, String, String)> {
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

        // Skip "com.apple.xcode" and get scheme, target, rest
        let scheme = parts.get(1)?.to_string();
        let target = parts.get(2)?.to_string();
        let full_test = parts[2..].join("/");

        Some((scheme, target, full_test))
    }

    fn build_project(&self, test_identifier: &str, workspace_root: &Path) -> TestRunnerResult {
        let (_, target, _) = match self.parse_test_identifier(test_identifier) {
            Some(parsed) => parsed,
            None => {
                return TestRunnerResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: String::new(),
                    message: format!("Invalid test identifier format: {}", test_identifier),
                };
            }
        };

        let output = Command::new("xcodebuild")
            .arg("build")
            .arg("-target")
            .arg(&target)
            .arg("-destination")
            .arg("platform=iOS Simulator,name=iPhone 17 Pro")
            .current_dir(workspace_root)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                TestRunnerResult {
                    success,
                    exit_code,
                    stdout: stdout.clone(),
                    stderr: stderr.clone(),
                    message: if success {
                        format!("Build succeeded for target: {}", target)
                    } else {
                        format!(
                            "Build failed for target: {} (exit code: {})",
                            target, exit_code
                        )
                    },
                }
            }
            Err(e) => TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!("Failed to execute xcodebuild: {}", e),
            },
        }
    }

    fn run_test(&self, test_identifier: &str, workspace_root: &Path) -> TestRunnerResult {
        let (scheme, _, full_test) = match self.parse_test_identifier(test_identifier) {
            Some(parsed) => parsed,
            None => {
                return TestRunnerResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: String::new(),
                    message: format!("Invalid test identifier format: {}", test_identifier),
                };
            }
        };

        let output = Command::new("xcodebuild")
            .arg("test")
            .arg("-scheme")
            .arg(&scheme)
            .arg("-destination")
            .arg("platform=iOS Simulator,name=iPhone 17 Pro")
            .arg(format!("-only-testing:{}", full_test))
            .current_dir(workspace_root)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

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
                }
            }
            Err(e) => TestRunnerResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                message: format!("Failed to execute xcodebuild: {}", e),
            },
        }
    }
}

impl Default for TestRunnerTool {
    fn default() -> Self {
        Self::new()
    }
}
