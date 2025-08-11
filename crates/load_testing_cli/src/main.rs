use clap::Parser;
use load_testing_core::{TestConfig, run_test};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// A high-performance, Rust-based API load testing tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Rune script file to execute.
    #[arg(required = true)]
    script_path: PathBuf,

    /// Number of concurrent virtual users (VUs) to simulate.
    #[arg(short = 'u', long, default_value_t = 1)]
    vus: u32,

    /// Total duration for the test run (e.g., "10s", "1m", "2h").
    #[arg(short = 'd', long, default_value = "10s", value_parser = humantime::parse_duration)]
    duration: Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Parse command-line arguments.
    let args = Args::parse();

    // 2. Read the script file from disk.
    let script_content = fs::read_to_string(&args.script_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read script file at '{}': {}",
            args.script_path.display(),
            e
        )
    })?;

    // 3. Create the configuration for the core engine.
    let config = TestConfig {
        script: script_content,
        vus: args.vus,
        duration: args.duration,
    };

    // 4. Call the core engine to run the test.
    println!(
        "Starting test with {} VUs for {:?}",
        config.vus, config.duration
    );
    run_test(config).await?;

    Ok(())
}
