use crate::xc_test_result_attachment_handler::{
    AttachmentHandlerError, XCTestResultAttachmentHandler,
};
use crate::xc_workspace_file_locator::{FileLocatorError, XCWorkspaceFileLocator};
use crate::xctestresultdetailparser::XCTestResultDetail;
use anthropic_sdk::{Anthropic, ContentBlock, MessageCreateBuilder};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Failed to create temporary directory: {0}")]
    CreateDirectoryError(#[from] std::io::Error),

    #[error("Failed to fetch attachments: {0}")]
    AttachmentError(#[from] AttachmentHandlerError),

    #[error("Failed to locate file: {0}")]
    FileLocatorError(#[from] FileLocatorError),

    #[error("Anthropic API error: {0}")]
    AnthropicApiError(String),
}

pub struct AutofixPipeline {
    xcresult_path: PathBuf,
    workspace_path: PathBuf,
    temp_dir: PathBuf,
}

impl AutofixPipeline {
    /// Create a new AutofixPipeline and initialize the temporary directory
    pub fn new<P: AsRef<Path>>(xcresult_path: P, workspace_path: P) -> Result<Self, PipelineError> {
        // Create .autofix/tmp directory in current directory
        let base_dir = PathBuf::from(".autofix/tmp");
        fs::create_dir_all(&base_dir)?;

        // Create a UUID-named subdirectory
        let uuid = Uuid::new_v4();
        let temp_dir = base_dir.join(uuid.to_string());
        fs::create_dir_all(&temp_dir)?;

        println!("Created temporary directory: {}", temp_dir.display());

        Ok(Self {
            xcresult_path: xcresult_path.as_ref().to_path_buf(),
            workspace_path: workspace_path.as_ref().to_path_buf(),
            temp_dir,
        })
    }

    /// Step 1: Fetch attachments from the XCResult bundle
    fn fetch_attachments_step(&self, test_identifier_url: &str) -> Result<(), PipelineError> {
        println!("Step 1: Fetching attachments...");
        let attachment_handler = XCTestResultAttachmentHandler::new();

        match attachment_handler.fetch_attachments(
            test_identifier_url,
            &self.xcresult_path,
            &self.temp_dir,
        ) {
            Ok(attachments_dir) => {
                println!("✓ Attachments fetched to: {}", attachments_dir.display());

                // List the attachments
                if let Ok(entries) = fs::read_dir(&attachments_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_file() {
                            println!("  - {}", entry.file_name().to_string_lossy());
                        }
                    }
                }
            }
            Err(e) => {
                println!("⚠ No attachments found or error fetching: {}", e);
            }
        }

        println!();
        Ok(())
    }

    /// Step 2: Locate the test file in the workspace
    fn locate_test_file_step(&self, test_identifier_url: &str) -> Result<PathBuf, PipelineError> {
        println!("Step 2: Locating test file...");
        let file_locator = XCWorkspaceFileLocator::new(&self.workspace_path);

        match file_locator.locate_file(test_identifier_url) {
            Ok(file_path) => {
                println!("✓ Test file located at: {}", file_path.display());
                println!(
                    "  File URL: file://{}",
                    file_path
                        .canonicalize()
                        .unwrap_or_else(|_| file_path.clone())
                        .display()
                );
                println!();
                Ok(file_path)
            }
            Err(e) => {
                println!("✗ Failed to locate file: {}", e);
                println!();
                Err(e.into())
            }
        }
    }

    /// Step 3: Perform autofix using Claude AI
    async fn autofix_step(&self) -> Result<(), PipelineError> {
        println!("Step 3: Running autofix with Claude AI...");

        // Create Anthropic client from environment
        let client =
            Anthropic::from_env().map_err(|e| PipelineError::AnthropicApiError(e.to_string()))?;

        // Create the prompt
        let prompt = "Hi, I am autofix, a tool to automatically fix UI tests on iOS and Android. Who are you?";

        // Create and send the message request
        let message = client
            .messages()
            .create(
                MessageCreateBuilder::new("claude-3-5-sonnet-latest", 1024)
                    .user(prompt)
                    .build(),
            )
            .await;

        // Handle the response
        match message {
            Ok(response) => {
                println!("✓ Received response from Claude:");
                println!();

                // Extract and print the text from the response
                for content in &response.content {
                    if let ContentBlock::Text { text } = content {
                        println!("  {}", text);
                    }
                }
                println!();
                Ok(())
            }
            Err(e) => {
                println!("✗ Failed to get response from Claude: {}", e);
                println!();
                Err(PipelineError::AnthropicApiError(e.to_string()))
            }
        }
    }

    /// Run the autofix pipeline for a given test result detail
    pub async fn run(&self, detail: &XCTestResultDetail) -> Result<(), PipelineError> {
        println!("\n========================================");
        println!("Running Autofix Pipeline");
        println!("========================================\n");

        self.fetch_attachments_step(&detail.test_identifier_url)?;
        self.locate_test_file_step(&detail.test_identifier_url)?;
        self.autofix_step().await?;

        println!("========================================");
        println!("Pipeline completed");
        println!("========================================\n");

        Ok(())
    }

    /// Clean up the temporary directory
    pub fn cleanup(&self) -> Result<(), PipelineError> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir)?;
            println!(
                "Cleaned up temporary directory: {}",
                self.temp_dir.display()
            );
        }
        Ok(())
    }
}

impl Drop for AutofixPipeline {
    fn drop(&mut self) {
        // Attempt to clean up on drop, but don't panic if it fails
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = AutofixPipeline::new("tests/fixtures/sample.xcresult", "path/to/workspace");

        assert!(pipeline.is_ok());
        let pipeline = pipeline.unwrap();

        // Verify temp directory was created
        assert!(pipeline.temp_dir.exists());
        assert!(pipeline.temp_dir.starts_with(".autofix/tmp"));

        // Cleanup
        pipeline.cleanup().unwrap();
    }

    #[test]
    fn test_pipeline_temp_dir_has_uuid() {
        let pipeline =
            AutofixPipeline::new("tests/fixtures/sample.xcresult", "path/to/workspace").unwrap();

        let dir_name = pipeline.temp_dir.file_name().unwrap().to_string_lossy();

        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&dir_name).is_ok());

        // Cleanup
        pipeline.cleanup().unwrap();
    }
}
