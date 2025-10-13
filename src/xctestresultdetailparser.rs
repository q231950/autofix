use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct XCTestResultDetail {
    pub test_identifier: String,
    #[serde(rename = "testIdentifierURL")]
    pub test_identifier_url: String,
    pub test_name: String,
    pub test_description: String,
    pub test_result: String,
    pub start_time: f64,
    pub duration: String,
    pub duration_in_seconds: f64,
    pub has_media_attachments: bool,
    pub has_performance_metrics: bool,
    pub devices: Vec<Device>,
    pub test_plan_configurations: Vec<TestPlanConfiguration>,
    pub test_runs: Vec<TestRun>,
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
pub struct TestRun {
    pub name: String,
    pub node_identifier: String,
    pub node_type: String,
    pub result: String,
    pub duration: String,
    pub duration_in_seconds: f64,
    #[serde(default)]
    pub details: Option<String>,
    pub children: Vec<TestNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TestNode {
    pub name: String,
    pub node_type: String,
    #[serde(default)]
    pub node_identifier: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub duration_in_seconds: Option<f64>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub children: Vec<TestNode>,
}

#[derive(Debug, thiserror::Error)]
pub enum XCTestResultDetailParserError {
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

    #[error("Test ID cannot be empty")]
    EmptyTestId,
}

pub struct XCTestResultDetailParser {
    xcresulttool_path: PathBuf,
}

impl XCTestResultDetailParser {
    /// Create a new XCTestResultDetailParser using the default xcresulttool path
    pub fn new() -> Self {
        Self {
            xcresulttool_path: PathBuf::from("xcrun"),
        }
    }

    /// Create a new XCTestResultDetailParser with a custom xcresulttool path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            xcresulttool_path: path.as_ref().to_path_buf(),
        }
    }

    /// Parse test details for a specific test ID from a .xcresult bundle
    pub fn parse<P: AsRef<Path>>(
        &self,
        xcresult_path: P,
        test_id: &str,
    ) -> Result<XCTestResultDetail, XCTestResultDetailParserError> {
        let path = xcresult_path.as_ref();

        if !path.exists() {
            return Err(XCTestResultDetailParserError::PathNotFound(
                path.to_path_buf(),
            ));
        }

        if test_id.is_empty() {
            return Err(XCTestResultDetailParserError::EmptyTestId);
        }

        let output = Command::new(&self.xcresulttool_path)
            .arg("xcresulttool")
            .arg("get")
            .arg("test-results")
            .arg("test-details")
            .arg("--test-id")
            .arg(test_id)
            .arg("--path")
            .arg(path)
            .output()
            .map_err(|e| XCTestResultDetailParserError::ExecutionError(e.to_string()))?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(XCTestResultDetailParserError::NonZeroExitCode(exit_code));
        }

        let json_str = String::from_utf8(output.stdout)?;
        let result: XCTestResultDetail = serde_json::from_str(&json_str)?;

        Ok(result)
    }
}

impl Default for XCTestResultDetailParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nonexistent_path() {
        let parser = XCTestResultDetailParser::new();
        let result = parser.parse("/nonexistent/path.xcresult", "test://example");

        assert!(result.is_err());
        match result {
            Err(XCTestResultDetailParserError::PathNotFound(_)) => {}
            _ => panic!("Expected PathNotFound error"),
        }
    }

    #[test]
    fn test_parse_empty_test_id() {
        let parser = XCTestResultDetailParser::new();
        // Create a temp directory that exists
        let temp_dir = std::env::temp_dir();
        let result = parser.parse(temp_dir, "");

        assert!(result.is_err());
        match result {
            Err(XCTestResultDetailParserError::EmptyTestId) => {}
            _ => panic!("Expected EmptyTestId error"),
        }
    }

    #[test]
    fn test_parser_with_custom_path() {
        let parser = XCTestResultDetailParser::with_path("/usr/bin/xcrun");
        let result = parser.parse("/nonexistent/path.xcresult", "test://example");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_fixture() {
        let parser = XCTestResultDetailParser::new();
        let fixture_path = "tests/fixtures/sample.xcresult";
        let test_id = "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample";

        let result = parser.parse(fixture_path, test_id);

        // If the fixture exists, verify we can parse it
        match result {
            Ok(detail) => {
                // Basic validation that we got a valid detail
                assert!(!detail.test_name.is_empty());
                assert_eq!(detail.test_result, "Failed");
                assert_eq!(detail.test_name, "testExample()");
            }
            Err(XCTestResultDetailParserError::PathNotFound(_)) => {
                // If fixture doesn't exist, that's okay too
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_detail_deserialization() {
        let json = std::fs::read_to_string("tests/fixtures/test_detail.json");

        match json {
            Ok(json_str) => {
                let result: Result<XCTestResultDetail, _> = serde_json::from_str(&json_str);
                assert!(result.is_ok());

                let detail = result.unwrap();
                assert_eq!(detail.test_name, "testExample()");
                assert_eq!(detail.test_result, "Failed");
                assert_eq!(
                    detail.test_identifier,
                    "AutoFixSamplerUITests/testExample()"
                );
                assert_eq!(detail.devices.len(), 1);
                assert_eq!(detail.devices[0].device_name, "iPhone 17 Pro");
                assert_eq!(detail.test_runs.len(), 1);
                assert_eq!(detail.test_runs[0].result, "Failed");
            }
            Err(_) => {
                // Fixture doesn't exist yet, skip this test
            }
        }
    }
}
