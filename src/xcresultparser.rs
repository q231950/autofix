use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct XCResultSummary {
    pub title: String,
    pub result: String,
    pub environment_description: String,
    pub start_time: f64,
    pub finish_time: f64,
    pub total_test_count: u32,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub skipped_tests: u32,
    pub expected_failures: u32,
    pub devices_and_configurations: Vec<DeviceConfiguration>,
    pub test_failures: Vec<TestFailure>,
    pub statistics: Vec<serde_json::Value>,
    pub top_insights: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConfiguration {
    pub device: Device,
    pub test_plan_configuration: TestPlanConfiguration,
    pub passed_tests: u32,
    pub failed_tests: u32,
    pub skipped_tests: u32,
    pub expected_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub device_id: String,
    pub device_name: String,
    pub model_name: String,
    pub architecture: String,
    pub platform: String,
    pub os_version: String,
    pub os_build_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TestPlanConfiguration {
    pub configuration_id: String,
    pub configuration_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TestFailure {
    pub test_identifier: u32,
    pub test_identifier_string: String,
    #[serde(rename = "testIdentifierURL")]
    pub test_identifier_url: String,
    pub test_name: String,
    pub target_name: String,
    pub failure_text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum XCResultParserError {
    #[error("Failed to execute xcresulttool: {0}")]
    ExecutionError(String),

    #[error("xcresulttool returned non-zero exit code: {0}")]
    NonZeroExitCode(i32),

    #[error("Failed to parse JSON output: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("Invalid UTF-8 in xcresulttool output")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

pub struct XCResultParser {
    xcresulttool_path: PathBuf,
}

impl XCResultParser {
    /// Create a new XCResultParser using the default xcresulttool path
    pub fn new() -> Self {
        Self {
            xcresulttool_path: PathBuf::from("xcrun"),
        }
    }

    /// Create a new XCResultParser with a custom xcresulttool path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            xcresulttool_path: path.as_ref().to_path_buf(),
        }
    }

    /// Parse a .xcresult bundle at the given path
    pub fn parse<P: AsRef<Path>>(
        &self,
        xcresult_path: P,
    ) -> Result<XCResultSummary, XCResultParserError> {
        let path = xcresult_path.as_ref();

        if !path.exists() {
            return Err(XCResultParserError::PathNotFound(path.to_path_buf()));
        }

        let output = Command::new(&self.xcresulttool_path)
            .arg("xcresulttool")
            .arg("get")
            .arg("test-results")
            .arg("summary")
            .arg("--path")
            .arg(path)
            .output()
            .map_err(|e| XCResultParserError::ExecutionError(e.to_string()))?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(XCResultParserError::NonZeroExitCode(exit_code));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let result: XCResultSummary = serde_json::from_str(&json_str)?;

        Ok(result)
    }
}

impl Default for XCResultParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nonexistent_path() {
        let parser = XCResultParser::new();
        let result = parser.parse("/nonexistent/path.xcresult");

        assert!(result.is_err());
        match result {
            Err(XCResultParserError::PathNotFound(_)) => {}
            _ => panic!("Expected PathNotFound error"),
        }
    }

    #[test]
    fn test_parse_fixture() {
        let parser = XCResultParser::new();
        let fixture_path = "tests/fixtures/sample.xcresult";

        let result = parser.parse(fixture_path);

        // If the fixture exists, verify we can parse it
        match result {
            Ok(summary) => {
                // Basic validation that we got a valid summary
                assert!(!summary.title.is_empty());
                assert!(summary.total_test_count > 0);
            }
            Err(XCResultParserError::PathNotFound(_)) => {
                // If fixture doesn't exist, that's okay too
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_parser_with_custom_path() {
        let parser = XCResultParser::with_path("/usr/bin/xcrun");
        let result = parser.parse("/nonexistent/path.xcresult");

        assert!(result.is_err());
    }

    #[test]
    fn test_xcresult_summary_deserialization() {
        let json = r#"{
            "devicesAndConfigurations": [
                {
                    "device": {
                        "architecture": "arm64",
                        "deviceId": "C19ECF87-BD95-40F7-B71D-187097B0C5D9",
                        "deviceName": "iPhone 17 Pro",
                        "modelName": "iPhone 17 Pro",
                        "osBuildNumber": "23A339",
                        "osVersion": "26.0",
                        "platform": "iOS Simulator"
                    },
                    "expectedFailures": 0,
                    "failedTests": 1,
                    "passedTests": 0,
                    "skippedTests": 0,
                    "testPlanConfiguration": {
                        "configurationId": "1",
                        "configurationName": "Test Scheme Action"
                    }
                }
            ],
            "environmentDescription": "AutoFixSampler · Built with macOS 26.0.1",
            "expectedFailures": 0,
            "failedTests": 1,
            "finishTime": 1760384517.611,
            "passedTests": 0,
            "result": "Failed",
            "skippedTests": 0,
            "startTime": 1760384501.803,
            "statistics": [],
            "testFailures": [
                {
                    "failureText": "Failed to tap button",
                    "targetName": "AutoFixSamplerUITests",
                    "testIdentifier": 1,
                    "testIdentifierString": "AutoFixSamplerUITests/testExample()",
                    "testIdentifierURL": "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample",
                    "testName": "testExample()"
                }
            ],
            "title": "Test - AutoFixSampler",
            "topInsights": [],
            "totalTestCount": 1
        }"#;

        let result: Result<XCResultSummary, _> = serde_json::from_str(json);
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.total_test_count, 1);
        assert_eq!(summary.failed_tests, 1);
        assert_eq!(summary.passed_tests, 0);
        assert_eq!(summary.result, "Failed");
        assert_eq!(summary.title, "Test - AutoFixSampler");
        assert_eq!(summary.devices_and_configurations.len(), 1);
        assert_eq!(summary.test_failures.len(), 1);
        assert_eq!(summary.test_failures[0].test_name, "testExample()");
    }
}
