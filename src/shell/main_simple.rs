//! Soliloquy Shell - Simplified standalone version
//! This version compiles without desktop shell integration for quick testing

use std::env;
use std::process::ExitCode;

use soliloquy_shell::optimizations::{init_optimizations, OptimizationSettings};
use soliloquy_shell::ServoEmbedder;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    init_optimizations(&OptimizationSettings::embedded());

    let mut embedder = match ServoEmbedder::new() {
        Ok(embedder) => embedder,
        Err(error) => {
            eprintln!("failed to initialize Soliloquy shell: {error}");
            return ExitCode::FAILURE;
        }
    };

    if let Some(start_url) = env::var("SOLILOQUY_START_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        if let Err(error) = embedder.load_url(&start_url) {
            eprintln!("failed to load {start_url}: {error}");
            return ExitCode::FAILURE;
        }
    }

    if let Err(error) = embedder.run_maintenance() {
        eprintln!("maintenance failed: {error}");
        return ExitCode::FAILURE;
    }

    match embedder.get_memory_stats() {
        Ok(stats) => println!("{stats}"),
        Err(error) => {
            eprintln!("failed to read memory stats: {error}");
            return ExitCode::FAILURE;
        }
    }

    println!("Soliloquy shell state: {:?}", embedder.get_state());
    ExitCode::SUCCESS
}
