use clap::Parser;

/// A simple autofix tool
#[derive(Parser, Debug)]
#[command(name = "autofix")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the file or directory to process
    #[arg(value_name = "PATH")]
    path: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Dry run mode (don't apply changes)
    #[arg(short = 'n', long)]
    dry_run: bool,
}

fn main() {
    let args = Args::parse();

    if args.verbose {
        println!("Running autofix with:");
        println!("  Path: {}", args.path);
        println!("  Dry run: {}", args.dry_run);
    }

    println!("Processing: {}", args.path);

    if args.dry_run {
        println!("Dry run mode - no changes will be applied");
    }
}
