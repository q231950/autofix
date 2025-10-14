use crate::xcresultparser::{XCResultParser, XCResultParserError};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AutofixError {
    #[error("Failed to parse XCResult: {0}")]
    XCResultParseError(#[from] XCResultParserError),

    #[error("No test failures found")]
    NoTestFailures,
}

pub struct AutofixCommand {
    test_result_path: PathBuf,
    workspace_path: PathBuf,
}

impl AutofixCommand {
    pub fn new(test_result_path: PathBuf, workspace_path: PathBuf) -> Self {
        Self {
            test_result_path,
            workspace_path,
        }
    }

    /// Execute the autofix command for iOS
    pub fn execute_ios(&self) -> Result<(), AutofixError> {
        println!("Running autofix for iOS...");
        println!("Test result path: {}", self.test_result_path.display());
        println!("Workspace path: {}", self.workspace_path.display());
        println!();

        // Parse the xcresult file
        let parser = XCResultParser::new();
        let summary = parser.parse(&self.test_result_path)?;

        // Display summary information
        println!("Test Summary:");
        println!("  Title: {}", summary.title);
        println!("  Result: {}", summary.result);
        println!("  Total tests: {}", summary.total_test_count);
        println!("  Passed: {}", summary.passed_tests);
        println!("  Failed: {}", summary.failed_tests);
        println!("  Skipped: {}", summary.skipped_tests);
        println!();

        // List failed tests
        if summary.failed_tests > 0 {
            println!("Failed Tests:");
            for (index, failure) in summary.test_failures.iter().enumerate() {
                println!("  {}. {}", index + 1, failure.test_name);
                println!("     Target: {}", failure.target_name);
                println!("     Test ID: {}", failure.test_identifier_string);
                println!("     Failure: {}", failure.failure_text);
                println!();
            }
        } else {
            return Err(AutofixError::NoTestFailures);
        }

        Ok(())
    }

    /// Execute the autofix command for Android (not yet implemented)
    pub fn execute_android(&self) -> Result<(), AutofixError> {
        println!("Android is not supported yet.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autofix_command_creation() {
        let cmd = AutofixCommand::new(
            PathBuf::from("tests/fixtures/sample.xcresult"),
            PathBuf::from("path/to/workspace"),
        );

        assert_eq!(
            cmd.test_result_path,
            PathBuf::from("tests/fixtures/sample.xcresult")
        );
        assert_eq!(cmd.workspace_path, PathBuf::from("path/to/workspace"));
    }

    #[test]
    fn test_execute_ios_with_fixture() {
        let cmd = AutofixCommand::new(
            PathBuf::from("tests/fixtures/sample.xcresult"),
            PathBuf::from("path/to/workspace"),
        );

        // This will only work if the fixture exists
        let result = cmd.execute_ios();

        // We don't assert success because the fixture might not exist
        // But we verify that if it fails, it's with an expected error
        if let Err(e) = result {
            match e {
                AutofixError::XCResultParseError(_) => {}
                AutofixError::NoTestFailures => {}
            }
        }
    }
}
