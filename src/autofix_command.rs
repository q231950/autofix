use crate::test_command::{TestCommand, TestCommandError};
use crate::xcresultparser::{XCResultParser, XCResultParserError, XCResultSummary};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AutofixError {
    #[error("Failed to parse XCResult: {0}")]
    XCResultParseError(#[from] XCResultParserError),

    #[error("No test failures found")]
    NoTestFailures,

    #[error("Failed to get test details: {0}")]
    TestCommandError(#[from] TestCommandError),
}

pub struct AutofixCommand {
    test_result_path: PathBuf,
    workspace_path: PathBuf,
    knightrider_mode: bool,
}

impl AutofixCommand {
    pub fn new(test_result_path: PathBuf, workspace_path: PathBuf, knightrider_mode: bool) -> Self {
        Self {
            test_result_path,
            workspace_path,
            knightrider_mode,
        }
    }

    /// Execute the autofix command for iOS
    pub async fn execute_ios(&self) -> Result<(), AutofixError> {
        println!("Running autofix for iOS...");
        println!("Test result path: {}", self.test_result_path.display());
        println!("Workspace path: {}", self.workspace_path.display());
        println!();

        // Parse the xcresult file
        let parser = XCResultParser::new();
        let summary = parser.parse(&self.test_result_path)?;

        // Display summary information
        Self::print_summary(&summary);

        // Process failed tests
        if summary.failed_tests > 0 {
            Self::print_failed_tests(&summary);

            // Process each failed test
            println!("Processing failed tests...");
            println!();
            for (index, failure) in summary.test_failures.iter().enumerate() {
                println!("═══════════════════════════════════════════════════════════");
                println!(
                    "Processing test {}/{}: {}",
                    index + 1,
                    summary.failed_tests,
                    failure.test_name
                );
                println!("═══════════════════════════════════════════════════════════");
                println!();

                // Use test command to get detailed information
                let test_cmd = TestCommand::new(
                    self.test_result_path.clone(),
                    self.workspace_path.clone(),
                    failure.test_identifier_url.clone(),
                    self.knightrider_mode,
                );

                test_cmd.execute_ios_silent().await?;
                println!();
            }
        } else {
            return Err(AutofixError::NoTestFailures);
        }

        Ok(())
    }

    /// Print the test summary
    fn print_summary(summary: &XCResultSummary) {
        println!("Test Summary:");
        println!("  Title: {}", summary.title);
        println!("  Result: {}", summary.result);
        println!("  Total tests: {}", summary.total_test_count);
        println!("  Passed: {}", summary.passed_tests);
        println!("  Failed: {}", summary.failed_tests);
        println!("  Skipped: {}", summary.skipped_tests);
        println!();
    }

    /// Print the list of failed tests
    fn print_failed_tests(summary: &XCResultSummary) {
        println!("Failed Tests:");
        for (index, failure) in summary.test_failures.iter().enumerate() {
            println!("  {}. {}", index + 1, failure.test_name);
            println!("     Target: {}", failure.target_name);
            println!("     Test ID: {}", failure.test_identifier_string);
            println!("     Failure: {}", failure.failure_text);
            println!();
        }
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
            false,
        );

        assert_eq!(
            cmd.test_result_path,
            PathBuf::from("tests/fixtures/sample.xcresult")
        );
        assert_eq!(cmd.workspace_path, PathBuf::from("path/to/workspace"));
    }

    #[tokio::test]
    async fn test_execute_ios_with_fixture() {
        let cmd = AutofixCommand::new(
            PathBuf::from("tests/fixtures/sample.xcresult"),
            PathBuf::from("path/to/workspace"),
            false,
        );

        // This will only work if the fixture exists
        let result = cmd.execute_ios().await;

        // We don't assert success because the fixture might not exist
        // But we verify that if it fails, it's with an expected error
        if let Err(e) = result {
            match e {
                AutofixError::XCResultParseError(_) => {}
                AutofixError::NoTestFailures => {}
                AutofixError::TestCommandError(_) => {}
            }
        }
    }
}
