mod autofix_command;
mod xcresultparser;
mod xctestresultdetailparser;

use autofix_command::AutofixCommand;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A tool to automatically fix failing UI tests
#[derive(Parser, Debug)]
#[command(name = "autofix")]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run autofix on test results
    Autofix {
        /// Run autofix for iOS tests
        #[arg(short = 'i', long, conflicts_with = "android")]
        ios: bool,

        /// Run autofix for Android tests (not yet implemented)
        #[arg(short = 'a', long, conflicts_with = "ios")]
        android: bool,

        /// Path to the test result file (xcresult for iOS)
        #[arg(long, required_if_eq("ios", "true"))]
        test_result: Option<PathBuf>,

        /// Path to the workspace/project
        #[arg(long, required_if_eq("ios", "true"))]
        workspace: Option<PathBuf>,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Autofix {
            ios,
            android,
            test_result,
            workspace,
        } => {
            if ios {
                // iOS autofix
                let test_result_path = test_result.expect("--test-result is required for iOS");
                let workspace_path = workspace.expect("--workspace is required for iOS");

                let cmd = AutofixCommand::new(test_result_path, workspace_path);

                if let Err(e) = cmd.execute_ios() {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            } else if android {
                // Android autofix
                let cmd = AutofixCommand::new(
                    test_result.unwrap_or_default(),
                    workspace.unwrap_or_default(),
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
