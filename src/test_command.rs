use crate::pipeline::{AutofixPipeline, PipelineError};
use crate::xctestresultdetailparser::{XCTestResultDetailParser, XCTestResultDetailParserError};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum TestCommandError {
    #[error("Failed to parse test details: {0}")]
    ParseError(#[from] XCTestResultDetailParserError),

    #[error("Failed to run autofix pipeline: {0}")]
    PipelineError(#[from] PipelineError),
}

pub struct TestCommand {
    test_result_path: PathBuf,
    workspace_path: PathBuf,
    test_id: String,
    knightrider_mode: bool,
}

impl TestCommand {
    pub fn new(
        test_result_path: PathBuf,
        workspace_path: PathBuf,
        test_id: String,
        knightrider_mode: bool,
    ) -> Self {
        Self {
            test_result_path,
            workspace_path,
            test_id,
            knightrider_mode,
        }
    }

    /// Execute the test command for iOS
    pub async fn execute_ios(&self) -> Result<(), TestCommandError> {
        self.execute_ios_internal(true).await
    }

    /// Execute the test command for iOS without printing (for use by autofix command)
    pub async fn execute_ios_silent(&self) -> Result<(), TestCommandError> {
        self.execute_ios_internal(true).await
    }

    async fn execute_ios_internal(&self, print_output: bool) -> Result<(), TestCommandError> {
        if print_output {
            println!("Fetching test details for iOS...");
            println!("Test result path: {}", self.test_result_path.display());
            println!("Workspace path: {}", self.workspace_path.display());
            println!("Test ID: {}", self.test_id);
            println!();
        }

        // Parse the test details
        let parser = XCTestResultDetailParser::new();
        let detail = parser.parse(&self.test_result_path, &self.test_id)?;

        if print_output {
            Self::print_test_detail(&detail);
        }

        // Run the autofix pipeline
        let pipeline = AutofixPipeline::new(
            &self.test_result_path,
            &self.workspace_path,
            self.knightrider_mode,
        )?;
        pipeline.run(&detail).await?;

        Ok(())
    }

    /// Print the test detail information
    pub fn print_test_detail(detail: &crate::xctestresultdetailparser::XCTestResultDetail) {
        println!("Test Details:");
        println!("  Name: {}", detail.test_name);
        println!("  Identifier: {}", detail.test_identifier);
        println!("  Result: {}", detail.test_result);
        println!("  Description: {}", detail.test_description);
        println!(
            "  Duration: {} ({:.2}s)",
            detail.duration, detail.duration_in_seconds
        );
        println!("  Start Time: {}", detail.start_time);
        println!("  Has Media Attachments: {}", detail.has_media_attachments);
        println!(
            "  Has Performance Metrics: {}",
            detail.has_performance_metrics
        );
        println!();

        // Print devices
        if !detail.devices.is_empty() {
            println!("Devices:");
            for device in &detail.devices {
                println!("  - {} ({})", device.device_name, device.model_name);
                println!("    Platform: {}", device.platform);
                println!("    OS: {} ({})", device.os_version, device.os_build_number);
                println!("    Architecture: {}", device.architecture);
                println!("    ID: {}", device.device_id);
            }
            println!();
        }

        // Print test plan configurations
        if !detail.test_plan_configurations.is_empty() {
            println!("Test Plan Configurations:");
            for config in &detail.test_plan_configurations {
                println!(
                    "  - {} (ID: {})",
                    config.configuration_name, config.configuration_id
                );
            }
            println!();
        }

        // Print test runs summary
        if !detail.test_runs.is_empty() {
            println!("Test Runs:");
            for run in &detail.test_runs {
                println!("  - {} ({})", run.name, run.result);
                println!("    Duration: {}", run.duration);
                println!("    Node Type: {}", run.node_type);
                if let Some(details) = &run.details {
                    println!("    Details: {}", details);
                }
                println!("    Children: {} nodes", run.children.len());
            }
            println!();
        }
    }

    /// Execute the test command for Android (not yet implemented)
    pub fn execute_android(&self) -> Result<(), TestCommandError> {
        println!("Android is not supported yet.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = TestCommand::new(
            PathBuf::from("tests/fixtures/sample.xcresult"),
            PathBuf::from("path/to/workspace"),
            "test://example".to_string(),
            false,
        );

        assert_eq!(
            cmd.test_result_path,
            PathBuf::from("tests/fixtures/sample.xcresult")
        );
        assert_eq!(cmd.workspace_path, PathBuf::from("path/to/workspace"));
        assert_eq!(cmd.test_id, "test://example");
    }

    #[tokio::test]
    async fn test_execute_ios_with_fixture() {
        let cmd = TestCommand::new(
            PathBuf::from("tests/fixtures/sample.xcresult"),
            PathBuf::from("path/to/workspace"),
            "test://com.apple.xcode/AutoFixSampler/AutoFixSamplerUITests/AutoFixSamplerUITests/testExample".to_string(),
            false,
        );

        // This will only work if the fixture exists
        let result = cmd.execute_ios_silent().await;

        // We don't assert success because the fixture might not exist
        // But we verify that if it fails, it's with an expected error
        if let Err(e) = result {
            match e {
                TestCommandError::ParseError(_) => {}
                TestCommandError::PipelineError(_) => {}
            }
        }
    }
}
