use crate::xc_test_result_attachment_handler::{
    AttachmentHandlerError, XCTestResultAttachmentHandler,
};
use crate::xc_workspace_file_locator::{FileLocatorError, XCWorkspaceFileLocator};
use crate::xctestresultdetailparser::XCTestResultDetail;
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

    /// Run the autofix pipeline for a given test result detail
    pub fn run(&self, detail: &XCTestResultDetail) -> Result<(), PipelineError> {
        println!("\n========================================");
        println!("Running Autofix Pipeline");
        println!("========================================\n");

        // Step 1: Fetch attachments
        println!("Step 1: Fetching attachments...");
        let attachment_handler = XCTestResultAttachmentHandler::new();

        match attachment_handler.fetch_attachments(
            &detail.test_identifier_url,
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

        // Step 2: Locate the test file
        println!("Step 2: Locating test file...");
        let file_locator = XCWorkspaceFileLocator::new(&self.workspace_path);

        match file_locator.locate_file(&detail.test_identifier_url) {
            Ok(file_path) => {
                println!("✓ Test file located at: {}", file_path.display());
                println!(
                    "  File URL: file://{}",
                    file_path.canonicalize().unwrap_or(file_path).display()
                );
            }
            Err(e) => {
                println!("✗ Failed to locate file: {}", e);
                return Err(e.into());
            }
        }

        println!();
        println!("========================================");
        println!("Pipeline completed");
        println!("========================================\n");

        Ok(())
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
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
        assert!(pipeline.temp_dir().exists());
        assert!(pipeline.temp_dir().starts_with(".autofix/tmp"));

        // Cleanup
        pipeline.cleanup().unwrap();
    }

    #[test]
    fn test_pipeline_temp_dir_has_uuid() {
        let pipeline =
            AutofixPipeline::new("tests/fixtures/sample.xcresult", "path/to/workspace").unwrap();

        let dir_name = pipeline.temp_dir().file_name().unwrap().to_string_lossy();

        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&dir_name).is_ok());

        // Cleanup
        pipeline.cleanup().unwrap();
    }
}
