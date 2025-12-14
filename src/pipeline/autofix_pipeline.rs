use super::prompts;
use crate::llm::{LLMProvider, ProviderConfig, ProviderFactory};
use crate::rate_limiter::RateLimiter;
use crate::tools::{
    CodeEditorInput, CodeEditorTool, DirectoryInspectorInput, DirectoryInspectorTool,
    TestRunnerInput, TestRunnerTool,
};
use crate::xc_test_result_attachment_handler::{
    AttachmentHandlerError, XCTestResultAttachmentHandler,
};
use crate::xc_workspace_file_locator::{FileLocatorError, XCWorkspaceFileLocator};
use crate::xctestresultdetailparser::XCTestResultDetail;
use anthropic_sdk::{ContentBlock, ContentBlockParam, Tool};
use base64::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    verbose: bool,
    rate_limiter: Arc<RateLimiter>,
    provider: Box<dyn LLMProvider>,
    provider_config: ProviderConfig,
}

impl AutofixPipeline {
    /// Create a new AutofixPipeline and initialize the temporary directory
    pub fn new<P: AsRef<Path>>(
        xcresult_path: P,
        workspace_path: P,
        knightrider_mode: bool,
        verbose: bool,
        provider_config: ProviderConfig,
    ) -> Result<Self, PipelineError> {
        // Create .autofix/tmp directory in current directory
        let base_dir = PathBuf::from(".autofix/tmp");
        fs::create_dir_all(&base_dir)?;

        // Create a UUID-named subdirectory
        let uuid = Uuid::new_v4();
        let temp_dir = base_dir.join(uuid.to_string());
        fs::create_dir_all(&temp_dir)?;

        if verbose {
            println!(
                "  [DEBUG] Created temporary directory: {}",
                temp_dir.display()
            );
        }

        // Create provider from configuration
        let provider = ProviderFactory::create(provider_config.clone()).map_err(|e| {
            PipelineError::AnthropicApiError(format!("Failed to create provider: {}", e))
        })?;

        // Create rate limiter for the configured provider
        let rate_limiter = Arc::new(RateLimiter::from_env(
            provider_config.provider_type,
            verbose,
        ));

        Ok(Self {
            xcresult_path: xcresult_path.as_ref().to_path_buf(),
            workspace_path: workspace_path.as_ref().to_path_buf(),
            temp_dir,
            knightrider_mode,
            verbose,
            rate_limiter,
            provider,
            provider_config,
        })
    }

    /// Step 1: Fetch attachments from the XCResult bundle
    fn fetch_attachments_step(&self, test_identifier_url: &str) -> Result<(), PipelineError> {
        println!("Step 1: Fetching attachments...");

        if self.verbose {
            println!("  [DEBUG] XCResult path: {}", self.xcresult_path.display());
            println!("  [DEBUG] Temp directory: {}", self.temp_dir.display());
            println!("  [DEBUG] Test ID: {}", test_identifier_url);
        }

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

        if self.verbose {
            println!(
                "  [DEBUG] Workspace path: {}",
                self.workspace_path.display()
            );
            println!("  [DEBUG] Test identifier URL: {}", test_identifier_url);
        }

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
                .map(std::cmp::Reverse)
        });

        image_files.first().map(|entry| entry.path())
    }

    /// Step 3: Perform autofix using Claude AI
    async fn autofix_step(
        &self,
        detail: &XCTestResultDetail,
        test_file_path: &Path,
    ) -> Result<(), PipelineError> {
        println!("Step 3: Running autofix with LLM provider...");

        if self.verbose {
            println!(
                "  [DEBUG] Mode: {}",
                if self.knightrider_mode {
                    "Knight Rider"
                } else {
                    "Standard"
                }
            );
            println!("  [DEBUG] Provider: {:?}", self.provider.provider_type());
            println!("  [DEBUG] Model: {}", self.provider_config.model);
            println!("  [DEBUG] Test file path: {}", test_file_path.display());
            println!("  [DEBUG] Test name: {}", detail.test_name);
        }

        // Read the test file contents
        let test_file_contents = fs::read_to_string(test_file_path)?;

        if self.verbose {
            println!(
                "  [DEBUG] Test file size: {} bytes",
                test_file_contents.len()
            );
        }

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
            prompts::generate_standard_prompt(
                detail,
                &test_file_contents,
                &self.workspace_path,
                has_snapshot,
            )
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
        self.run_with_tools(content_blocks, detail, test_file_path)
            .await
    }

    /// Convert anthropic ContentBlock to provider-agnostic ToolCall
    fn content_block_to_tool_call(block: &ContentBlock) -> Option<crate::llm::ToolCall> {
        match block {
            ContentBlock::ToolUse { id, name, input } => Some(crate::llm::ToolCall {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            _ => None,
        }
    }

    /// Convert provider-agnostic LLMResponse to anthropic Message format
    fn llm_response_to_anthropic_message(
        response: crate::llm::LLMResponse,
        model: &str,
    ) -> anthropic_sdk::Message {
        use anthropic_sdk::{Message, Role, StopReason as AnthropicStopReason, Usage};

        // Convert content and tool calls to ContentBlocks
        let mut content_blocks = Vec::new();

        // Add text content if present
        if let Some(text) = response.content
            && !text.is_empty() {
                content_blocks.push(ContentBlock::Text { text });
            }

        // Add tool calls
        for tool_call in response.tool_calls {
            content_blocks.push(ContentBlock::ToolUse {
                id: tool_call.id,
                name: tool_call.name,
                input: tool_call.input,
            });
        }

        // Convert stop reason
        let stop_reason = Some(match response.stop_reason {
            crate::llm::StopReason::EndTurn => AnthropicStopReason::EndTurn,
            crate::llm::StopReason::MaxTokens => AnthropicStopReason::MaxTokens,
            crate::llm::StopReason::StopSequence => AnthropicStopReason::StopSequence,
            crate::llm::StopReason::ToolUse => AnthropicStopReason::ToolUse,
            crate::llm::StopReason::Error => AnthropicStopReason::EndTurn, // Map error to end turn
        });

        Message {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            type_: "message".to_string(),
            role: Role::Assistant,
            content: content_blocks,
            model: model.to_string(),
            stop_reason,
            stop_sequence: None,
            usage: Usage {
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
                server_tool_use: None,
                service_tier: None,
            },
            request_id: None,
        }
    }

    async fn run_with_tools(
        &self,
        initial_content: Vec<ContentBlockParam>,
        detail: &XCTestResultDetail,
        test_file_path: &Path,
    ) -> Result<(), PipelineError> {
        // Create tool instances
        let dir_tool = DirectoryInspectorTool::new();
        let code_tool = CodeEditorTool::new();
        let test_tool = TestRunnerTool::new();

        // Build tools for LLM API
        let tools: Vec<Tool> = vec![
            serde_json::from_value(dir_tool.to_tool_definition()).unwrap(),
            serde_json::from_value(code_tool.to_tool_definition()).unwrap(),
            serde_json::from_value(test_tool.to_tool_definition()).unwrap(),
        ];

        // Track conversation history: (user_content, assistant_content)
        let mut conversation_history: Vec<(Vec<ContentBlockParam>, Vec<ContentBlock>)> = vec![];
        let mut current_user_content = initial_content;
        let max_iterations = 20; // Prevent infinite loops
        #[allow(unused_assignments)]
        let mut test_failed_in_last_iteration = false;

        for iteration in 0..max_iterations {
            println!("\n🤖 autofix iteration {}...", iteration + 1);

            // Build the LLM request using provider-agnostic types
            let mut messages = Vec::new();

            // Add all previous conversation turns
            for (user_content, assistant_content) in &conversation_history {
                // Add user message
                let user_text = user_content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlockParam::Text { text } => Some(text.clone()),
                        ContentBlockParam::ToolResult { content, .. } => content.clone(),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !user_text.is_empty() {
                    messages.push(crate::llm::Message {
                        role: crate::llm::MessageRole::User,
                        content: user_text,
                    });
                }

                // Add assistant message
                let assistant_text = assistant_content
                    .iter()
                    .filter_map(|block| match block {
                        ContentBlock::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                if !assistant_text.is_empty() {
                    messages.push(crate::llm::Message {
                        role: crate::llm::MessageRole::Assistant,
                        content: assistant_text,
                    });
                }
            }

            // Add current user message
            let current_user_text = current_user_content
                .iter()
                .filter_map(|block| match block {
                    ContentBlockParam::Text { text } => Some(text.clone()),
                    ContentBlockParam::ToolResult { content, .. } => content.clone(),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            if !current_user_text.is_empty() {
                messages.push(crate::llm::Message {
                    role: crate::llm::MessageRole::User,
                    content: current_user_text,
                });
            }

            // Convert tools to provider-agnostic format
            let tool_definitions: Vec<crate::llm::ToolDefinition> = tools
                .iter()
                .map(|tool| crate::llm::ToolDefinition {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: serde_json::to_value(&tool.input_schema)
                        .unwrap_or(serde_json::json!({})),
                })
                .collect();

            // Estimate token count for rate limiting
            // Rough estimation: ~4 chars per token, plus conversation history
            let estimated_tokens =
                self.estimate_request_tokens(&conversation_history, &current_user_content);

            if self.verbose {
                println!("  [DEBUG] Estimated input tokens: {}", estimated_tokens);
                let (used, remaining, reset_in) = self.rate_limiter.get_stats();
                println!(
                    "  [DEBUG] Rate limit - Used: {}, Remaining: {}, Reset in: {}s",
                    used, remaining, reset_in
                );
            }

            // Check rate limit and wait if necessary
            if let Err(wait_duration) = self.rate_limiter.check_and_wait(estimated_tokens) {
                let wait_secs = wait_duration.as_secs();
                println!(
                    "\n⏸️  Rate limit approaching. Waiting {} seconds before next request...",
                    wait_secs
                );

                // Animated countdown
                for remaining in (1..=wait_secs).rev() {
                    print!(
                        "\r⏳ Waiting: {} second{}...   ",
                        remaining,
                        if remaining == 1 { "" } else { "s" }
                    );
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                print!("\r✓ Rate limit window reset - continuing...                    \n");
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }

            // Build LLMRequest
            let llm_request = crate::llm::LLMRequest {
                system_prompt: None,
                messages,
                tools: tool_definitions,
                max_tokens: Some(1024),
                temperature: Some(0.7),
                stream: false,
            };

            // Call provider
            let llm_response = self.provider.complete(llm_request).await.map_err(|e| {
                println!("✗ Provider Error: {}", e);
                PipelineError::AnthropicApiError(format!("Provider error: {}", e))
            })?;

            // Convert response back to anthropic format for compatibility with rest of pipeline
            let response =
                Self::llm_response_to_anthropic_message(llm_response, &self.provider_config.model);

            // Record actual token usage from the API response
            let actual_input_tokens = response.usage.input_tokens as usize;
            self.rate_limiter.record_usage(actual_input_tokens);

            if self.verbose {
                println!(
                    "  [DEBUG] Actual input tokens used: {}",
                    actual_input_tokens
                );
                println!(
                    "  [DEBUG] Estimated was: {}, difference: {}",
                    estimated_tokens,
                    (actual_input_tokens as i64 - estimated_tokens as i64).abs()
                );
            }

            // Check stop reason
            let has_tool_use = response
                .content
                .iter()
                .any(|c| matches!(c, ContentBlock::ToolUse { .. }));

            // Print text responses and check for give-up message
            let mut gave_up = false;
            for content in &response.content {
                if let ContentBlock::Text { text } = content {
                    println!("\n💭 Claude says:\n{}\n", text);

                    // Check if Claude is giving up
                    if text.contains("GIVING UP:") {
                        gave_up = true;
                        self.handle_give_up(text);
                    }
                }
            }

            if gave_up || !has_tool_use {
                if !gave_up {
                    println!("\n✓ autofix finished!");
                }
                return Ok(());
            }

            // Execute tool calls
            let mut tool_results = Vec::new();
            test_failed_in_last_iteration = false; // Reset for this iteration

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

                            if self.verbose {
                                println!("   [DEBUG] Operation: {}", tool_input.operation);
                                println!("   [DEBUG] Path: {}", tool_input.path);
                            }

                            let result = dir_tool.execute(tool_input, &self.workspace_path);

                            if self.verbose {
                                println!(
                                    "   [DEBUG] Result: {}",
                                    serde_json::to_string_pretty(&result).unwrap_or_default()
                                );
                            }

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

                            if self.verbose {
                                println!("   [DEBUG] File path: {}", tool_input.file_path);
                                println!(
                                    "   [DEBUG] Old content length: {} chars",
                                    tool_input.old_content.len()
                                );
                                println!(
                                    "   [DEBUG] New content length: {} chars",
                                    tool_input.new_content.len()
                                );
                            }

                            let result = code_tool.execute(tool_input, &self.workspace_path);
                            println!("   ✏️ Edit result: {}", result.message);

                            if self.verbose && result.success {
                                println!("   [DEBUG] Edit successful");
                            }

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

                            if self.verbose {
                                println!("   [DEBUG] Operation: {}", tool_input.operation);
                                println!(
                                    "   [DEBUG] Test identifier: {}",
                                    tool_input.test_identifier
                                );
                            }

                            let result = test_tool.execute(tool_input, &self.workspace_path);
                            println!(
                                "   🧪 Test result: {} (exit code: {})",
                                result.message, result.exit_code
                            );
                            if result.success {
                                println!("   ✅ SUCCESS!");
                            } else {
                                test_failed_in_last_iteration = true;

                                if let Some(ref test_detail) = result.test_detail {
                                    println!("   ❌ Test failed: {}", test_detail.test_name);
                                    println!("   📊 Result: {}", test_detail.test_result);
                                    println!(
                                        "   📸 New snapshot available at: {:?}",
                                        result.xcresult_path
                                    );

                                    // Store xcresult path for extracting new snapshot in next iteration
                                    if let Some(ref xcresult_path) = result.xcresult_path {
                                        if self.verbose {
                                            println!(
                                                "   [DEBUG] Saving xcresult path for next iteration"
                                            );
                                        }
                                        // Extract and save the new snapshot
                                        self.extract_latest_snapshot_from_xcresult(
                                            xcresult_path,
                                            &detail.test_identifier_url,
                                        )?;
                                    }
                                }
                            }

                            if self.verbose {
                                println!("   [DEBUG] stdout length: {} bytes", result.stdout.len());
                                println!("   [DEBUG] stderr length: {} bytes", result.stderr.len());
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

                // If test failed in last iteration, inject updated context for next iteration
                if test_failed_in_last_iteration {
                    if self.verbose {
                        println!(
                            "\n  [DEBUG] Test failed - preparing updated context for next iteration"
                        );
                    }

                    // Re-read the test file (it may have been edited)
                    if let Ok(updated_test_content) = fs::read_to_string(test_file_path) {
                        // Find the latest snapshot
                        if let Some(snapshot_path) = self.find_latest_snapshot() {
                            println!("\n📋 Providing updated context for next iteration:");
                            println!("   • Updated test file content");
                            println!("   • Latest failure snapshot");

                            // Add updated test file content as a text message
                            let context_message = format!(
                                "UPDATED CONTEXT after test failure:\n\n\
                                The test file may have been modified. Here's the current content:\n\n\
                                ```swift\n{}\n```\n\n\
                                A new snapshot from the failed test run is attached below showing the current UI state.",
                                updated_test_content
                            );
                            current_user_content.push(ContentBlockParam::text(&context_message));

                            // Add the new snapshot image
                            if let Ok(image_data) = fs::read(&snapshot_path) {
                                let base64_image =
                                    base64::engine::general_purpose::STANDARD.encode(&image_data);
                                current_user_content.push(ContentBlockParam::image_base64(
                                    "image/jpeg",
                                    &base64_image,
                                ));
                            }
                        }
                    }
                }
            } else {
                // No tool results but Claude didn't finish - shouldn't happen but handle it
                break;
            }
        }

        println!("\n⚠️ Maximum iterations reached");
        Ok(())
    }

    /// Extract the latest snapshot from an xcresult bundle
    fn extract_latest_snapshot_from_xcresult(
        &self,
        xcresult_path: &Path,
        test_id: &str,
    ) -> Result<(), PipelineError> {
        let attachment_handler = XCTestResultAttachmentHandler::new();

        if self.verbose {
            println!(
                "  [DEBUG] Extracting attachments from: {}",
                xcresult_path.display()
            );
        }

        match attachment_handler.fetch_attachments(test_id, xcresult_path, &self.temp_dir) {
            Ok(attachments_dir) => {
                if self.verbose {
                    println!(
                        "  [DEBUG] Attachments extracted to: {}",
                        attachments_dir.display()
                    );
                }
                Ok(())
            }
            Err(e) => {
                if self.verbose {
                    println!("  [DEBUG] Failed to extract attachments: {}", e);
                }
                // Don't fail the entire pipeline if we can't extract attachments
                Ok(())
            }
        }
    }

    /// Handle Claude giving up by parsing the message and opening Xcode
    fn handle_give_up(&self, text: &str) {
        println!("\n❌ Claude has given up after multiple attempts\n");

        // Try to parse the file path and line number from the message
        // Expected format:
        // File: /absolute/path/to/File.swift
        // Line: 42

        let mut file_path: Option<String> = None;
        let mut line_number: Option<u32> = None;

        for line in text.lines() {
            let line = line.trim();

            if line.starts_with("File:") {
                file_path = Some(line.trim_start_matches("File:").trim().to_string());
            } else if line.starts_with("Line:")
                && let Ok(num) = line.trim_start_matches("Line:").trim().parse::<u32>() {
                    line_number = Some(num);
                }
        }

        // Generate Xcode deep link if we have both file and line
        if let (Some(file), Some(line)) = (file_path, line_number) {
            let xcode_url = format!("xed://open?file={}&line={}", file, line);

            println!("┌─────────────────────────────────────────────────────────────");
            println!("│ 🚀 Opening Xcode at the failing assertion...");
            println!("│");
            println!("│ File: {}", file);
            println!("│ Line: {}", line);
            println!("└─────────────────────────────────────────────────────────────\n");

            // Try to open Xcode using the 'open' command on macOS
            if cfg!(target_os = "macos") {
                match std::process::Command::new("open").arg(&xcode_url).output() {
                    Ok(_) => {
                        println!("✓ Xcode should now be opening at the failing line\n");
                    }
                    Err(e) => {
                        println!("⚠️  Could not automatically open Xcode: {}", e);
                        println!("   Copy and paste this URL to open manually:");
                        println!("   {}\n", xcode_url);
                    }
                }
            } else {
                println!("ℹ️  Xcode deep link (macOS only):");
                println!("   {}\n", xcode_url);
            }
        } else {
            println!("⚠️  Could not parse file location from give-up message\n");
        }
    }

    /// Estimate the number of tokens in a request
    /// Uses a simple heuristic: ~4 characters per token
    fn estimate_request_tokens(
        &self,
        conversation_history: &[(Vec<ContentBlockParam>, Vec<ContentBlock>)],
        current_content: &[ContentBlockParam],
    ) -> usize {
        let mut char_count = 0;

        // Count characters in conversation history
        for (user_blocks, assistant_blocks) in conversation_history {
            char_count += self.estimate_content_param_chars(user_blocks);
            char_count += self.estimate_content_block_chars(assistant_blocks);
        }

        // Count characters in current content
        char_count += self.estimate_content_param_chars(current_content);

        // Convert to token estimate (rough: 1 token ≈ 4 chars)
        // Add 20% buffer for safety
        

        (char_count / 4) * 12 / 10
    }

    fn estimate_content_param_chars(&self, blocks: &[ContentBlockParam]) -> usize {
        blocks
            .iter()
            .map(|block| match block {
                ContentBlockParam::Text { text } => text.len(),
                ContentBlockParam::ToolResult { content, .. } => {
                    content.as_ref().map(|s| s.len()).unwrap_or(0)
                }
                _ => 100, // Rough estimate for other types
            })
            .sum()
    }

    fn estimate_content_block_chars(&self, blocks: &[ContentBlock]) -> usize {
        blocks
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => text.len(),
                _ => 100, // Rough estimate for other types
            })
            .sum()
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
        let config = ProviderConfig::default();
        let pipeline = AutofixPipeline::new(
            "tests/fixtures/sample.xcresult",
            "path/to/workspace",
            false,
            false,
            config,
        );

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
        let config = ProviderConfig::default();
        let pipeline = AutofixPipeline::new(
            "tests/fixtures/sample.xcresult",
            "path/to/workspace",
            false,
            false,
            config,
        )
        .unwrap();

        let dir_name = pipeline.temp_dir.file_name().unwrap().to_string_lossy();

        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&dir_name).is_ok());

        // Cleanup
        pipeline.cleanup().unwrap();
    }
}
