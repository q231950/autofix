use super::prompts;
use crate::tools::{
    CodeEditorInput, CodeEditorTool, DirectoryInspectorInput, DirectoryInspectorTool,
    TestRunnerInput, TestRunnerTool,
};
use crate::xc_test_result_attachment_handler::{
    AttachmentHandlerError, XCTestResultAttachmentHandler,
};
use crate::xc_workspace_file_locator::{FileLocatorError, XCWorkspaceFileLocator};
use crate::xctestresultdetailparser::XCTestResultDetail;
use anthropic_sdk::{
    Anthropic, ContentBlock, ContentBlockParam, MessageContent, MessageCreateBuilder, Tool,
};
use base64::Engine;
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
    knightrider_mode: bool,
}

impl AutofixPipeline {
    /// Create a new AutofixPipeline and initialize the temporary directory
    pub fn new<P: AsRef<Path>>(
        xcresult_path: P,
        workspace_path: P,
        knightrider_mode: bool,
    ) -> Result<Self, PipelineError> {
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
            knightrider_mode,
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

    /// Helper function to find the latest simulator snapshot image
    fn find_latest_snapshot(&self) -> Option<PathBuf> {
        let attachments_dir = self.temp_dir.join("attachments");
        if !attachments_dir.exists() {
            return None;
        }

        // Look for image files (png, jpg, jpeg)
        let mut image_files: Vec<_> = fs::read_dir(&attachments_dir)
            .ok()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let path = entry.path();
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg"))
                        .unwrap_or(false)
            })
            .collect();

        // Sort by modification time (newest first)
        image_files.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .map(|t| std::cmp::Reverse(t))
        });

        image_files.first().map(|entry| entry.path())
    }

    /// Step 3: Perform autofix using Claude AI
    async fn autofix_step(
        &self,
        detail: &XCTestResultDetail,
        test_file_path: &Path,
    ) -> Result<(), PipelineError> {
        println!("Step 3: Running autofix with Claude AI...");

        // Create Anthropic client from environment
        let client =
            Anthropic::from_env().map_err(|e| PipelineError::AnthropicApiError(e.to_string()))?;

        // Read the test file contents
        let test_file_contents = fs::read_to_string(test_file_path)?;

        // Find the latest simulator snapshot
        let snapshot_path = self.find_latest_snapshot();
        let has_snapshot = snapshot_path.is_some();

        // Generate the prompt based on mode
        let prompt = if self.knightrider_mode {
            prompts::generate_knightrider_prompt(
                detail,
                &test_file_contents,
                &self.workspace_path,
                has_snapshot,
            )
        } else {
            prompts::generate_standard_prompt(detail, &test_file_contents, has_snapshot)
        };

        // Print the prompt
        println!("Sending prompt to Claude:");
        println!("─────────────────────────────────────────");
        println!("{}", prompt);
        println!("─────────────────────────────────────────");
        println!();

        // Build the message content with text and optionally an image
        let mut content_blocks = vec![ContentBlockParam::text(&prompt)];

        // Add the image if available
        if let Some(img_path) = snapshot_path {
            println!("Adding simulator snapshot: {}", img_path.display());
            if let Ok(image_data) = fs::read(&img_path) {
                // Convert image to base64
                let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_data);
                content_blocks.push(ContentBlockParam::image_base64("image/jpeg", &base64_image));
            }
        }

        // Both modes use tools - the difference is in the prompt guidance
        self.run_with_tools(client, content_blocks, detail).await
    }

    async fn run_with_tools(
        &self,
        client: Anthropic,
        initial_content: Vec<ContentBlockParam>,
        _detail: &XCTestResultDetail,
    ) -> Result<(), PipelineError> {
        // Create tool instances
        let dir_tool = DirectoryInspectorTool::new();
        let code_tool = CodeEditorTool::new();
        let test_tool = TestRunnerTool::new();

        // Build tools for Anthropic API
        let tools: Vec<Tool> = vec![
            serde_json::from_value(dir_tool.to_anthropic_tool()).unwrap(),
            serde_json::from_value(code_tool.to_anthropic_tool()).unwrap(),
            serde_json::from_value(test_tool.to_anthropic_tool()).unwrap(),
        ];

        // Track conversation history: (user_content, assistant_content)
        let mut conversation_history: Vec<(Vec<ContentBlockParam>, Vec<ContentBlock>)> = vec![];
        let mut current_user_content = initial_content;
        let max_iterations = 20; // Prevent infinite loops

        for iteration in 0..max_iterations {
            println!("\n🤖 Knight Rider iteration {}...", iteration + 1);

            // Build the message with full conversation history
            let mut builder = MessageCreateBuilder::new("claude-3-5-haiku-latest", 1024);

            // Add all previous conversation turns
            for (user_content, assistant_content) in &conversation_history {
                builder = builder.user(MessageContent::Blocks(user_content.clone()));

                // Convert ContentBlock (response) to ContentBlockParam (request) for assistant message
                let assistant_blocks: Vec<ContentBlockParam> = assistant_content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => {
                            Some(ContentBlockParam::Text { text: text.clone() })
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            Some(ContentBlockParam::ToolUse {
                                id: id.clone(),
                                name: name.clone(),
                                input: input.clone(),
                            })
                        }
                        _ => None,
                    })
                    .collect();

                if !assistant_blocks.is_empty() {
                    builder = builder.assistant(MessageContent::Blocks(assistant_blocks));
                }
            }

            // Add current user message
            builder = builder.user(MessageContent::Blocks(current_user_content.clone()));

            // Add tools
            builder = builder.tools(tools.clone());

            let message = client.messages().create(builder.build()).await;

            let response = match message {
                Ok(resp) => resp,
                Err(e) => {
                    println!("✗ API Error: {}", e);
                    return Err(PipelineError::AnthropicApiError(e.to_string()));
                }
            };

            // Check stop reason
            let has_tool_use = response
                .content
                .iter()
                .any(|c| matches!(c, ContentBlock::ToolUse { .. }));

            // Print text responses
            for content in &response.content {
                if let ContentBlock::Text { text } = content {
                    println!("\n💭 Claude says:\n{}\n", text);
                }
            }

            if !has_tool_use {
                println!("\n✓ Knight Rider finished!");
                return Ok(());
            }

            // Execute tool calls
            let mut tool_results = Vec::new();
            for content in &response.content {
                if let ContentBlock::ToolUse { id, name, input } = content {
                    println!("\n🔧 Tool call: {} (id: {})", name, id);
                    println!(
                        "   Input: {}",
                        serde_json::to_string_pretty(input).unwrap_or_default()
                    );

                    let result = match name.as_str() {
                        "directory_inspector" => {
                            let tool_input: DirectoryInspectorInput =
                                serde_json::from_value(input.clone()).map_err(|e| {
                                    PipelineError::AnthropicApiError(format!(
                                        "Invalid tool input: {}",
                                        e
                                    ))
                                })?;
                            let result = dir_tool.execute(tool_input, &self.workspace_path);
                            serde_json::to_value(&result).unwrap()
                        }
                        "code_editor" => {
                            let tool_input: CodeEditorInput = serde_json::from_value(input.clone())
                                .map_err(|e| {
                                    PipelineError::AnthropicApiError(format!(
                                        "Invalid tool input: {}",
                                        e
                                    ))
                                })?;
                            let result = code_tool.execute(tool_input, &self.workspace_path);
                            println!("   ✏️ Edit result: {}", result.message);
                            serde_json::to_value(&result).unwrap()
                        }
                        "test_runner" => {
                            let tool_input: TestRunnerInput = serde_json::from_value(input.clone())
                                .map_err(|e| {
                                    PipelineError::AnthropicApiError(format!(
                                        "Invalid tool input: {}",
                                        e
                                    ))
                                })?;
                            let result = test_tool.execute(tool_input, &self.workspace_path);
                            println!(
                                "   🧪 Test result: {} (exit code: {})",
                                result.message, result.exit_code
                            );
                            if result.success {
                                println!("   ✅ SUCCESS!");
                            }
                            serde_json::to_value(&result).unwrap()
                        }
                        _ => serde_json::json!({"error": format!("Unknown tool: {}", name)}),
                    };

                    tool_results.push(ContentBlockParam::ToolResult {
                        tool_use_id: id.clone(),
                        content: Some(result.to_string()),
                        is_error: Some(false),
                    });
                }
            }

            // Save this turn to conversation history
            conversation_history.push((current_user_content.clone(), response.content.clone()));

            // Update current_user_content to be the tool results for the next iteration
            if !tool_results.is_empty() {
                current_user_content = tool_results;
            } else {
                // No tool results but Claude didn't finish - shouldn't happen but handle it
                break;
            }
        }

        println!("\n⚠️ Maximum iterations reached");
        Ok(())
    }

    /// Run the autofix pipeline for a given test result detail
    pub async fn run(&self, detail: &XCTestResultDetail) -> Result<(), PipelineError> {
        println!("\n========================================");
        println!("Running Autofix Pipeline");
        println!("========================================\n");

        self.fetch_attachments_step(&detail.test_identifier_url)?;
        let test_file_path = self.locate_test_file_step(&detail.test_identifier_url)?;
        self.autofix_step(detail, &test_file_path).await?;

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
        let pipeline =
            AutofixPipeline::new("tests/fixtures/sample.xcresult", "path/to/workspace", false);

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
            AutofixPipeline::new("tests/fixtures/sample.xcresult", "path/to/workspace", false)
                .unwrap();

        let dir_name = pipeline.temp_dir.file_name().unwrap().to_string_lossy();

        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&dir_name).is_ok());

        // Cleanup
        pipeline.cleanup().unwrap();
    }
}
