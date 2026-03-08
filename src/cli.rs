use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tmux-babysitter")]
#[command(about = "Monitor a tmux session and automatically respond to questions using LLM", long_about = None)]
pub struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    pub config: PathBuf,

    /// Dry run mode - only log what would be done
    #[arg(long)]
    pub dry_run: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}
