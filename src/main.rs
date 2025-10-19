mod autofix_command;
mod pipeline;
mod rate_limiter;
mod test_command;
mod tools;
mod xc_test_result_attachment_handler;
mod xc_workspace_file_locator;
mod xcresultparser;
mod xctestresultdetailparser;

use autofix_command::AutofixCommand;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use test_command::TestCommand;

/// A tool to automatically fix failing UI tests
#[derive(Parser, Debug)]
#[command(name = "autofix")]
#[command(version, about, long_about = None)]
struct Args {
    /// Run autofix for iOS tests
    #[arg(short = 'i', long, conflicts_with = "android", global = true)]
    ios: bool,

    /// Run autofix for Android tests (not yet implemented)
    #[arg(short = 'a', long, conflicts_with = "ios", global = true)]
    android: bool,

    /// Path to the test result file (xcresult for iOS)
    #[arg(long, required_if_eq("ios", "true"), global = true)]
    test_result: Option<PathBuf>,

    /// Path to the workspace/project
    #[arg(long, required_if_eq("ios", "true"), global = true)]
    workspace: Option<PathBuf>,

    /// Enable Knight Rider mode: AI agent with tools to automatically fix code
    #[arg(long, global = true)]
    knightrider: bool,

    /// Enable verbose mode: print detailed debug information
    #[arg(short = 'v', long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Get details for a specific test
    Test {
        /// Test ID to fetch details for
        #[arg(short = 't', long)]
        test_id: String,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        // Handle "autofix test --test-id ..." subcommand
        Some(Commands::Test { test_id }) => {
            if args.ios {
                // iOS test details
                let test_result_path = args.test_result.expect("--test-result is required for iOS");
                let workspace_path = args.workspace.expect("--workspace is required for iOS");

                let cmd = TestCommand::new(
                    test_result_path,
                    workspace_path,
                    test_id,
                    args.knightrider,
                    args.verbose,
                );

                if let Err(e) = cmd.execute_ios().await {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else if args.android {
                // Android test details
                let cmd = TestCommand::new(
                    args.test_result.unwrap_or_default(),
                    args.workspace.unwrap_or_default(),
                    test_id,
                    args.knightrider,
                    args.verbose,
                );

                if let Err(e) = cmd.execute_android() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Error: Either --ios or --android must be specified");
                std::process::exit(1);
            }
        }
        // Handle "autofix --ios ..." (no subcommand - process all tests)
        None => {
            if args.ios {
                // iOS autofix - process all failed tests
                let test_result_path = args.test_result.expect("--test-result is required for iOS");
                let workspace_path = args.workspace.expect("--workspace is required for iOS");

                let cmd = AutofixCommand::new(
                    test_result_path,
                    workspace_path,
                    args.knightrider,
                    args.verbose,
                );

                if let Err(e) = cmd.execute_ios().await {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else if args.android {
                // Android autofix
                let cmd = AutofixCommand::new(
                    args.test_result.unwrap_or_default(),
                    args.workspace.unwrap_or_default(),
                    args.knightrider,
                    args.verbose,
                );

                if let Err(e) = cmd.execute_android() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Error: Either --ios or --android must be specified");
                std::process::exit(1);
            }
        }
    }
}
