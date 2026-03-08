mod cli;
mod config;
mod guard;
mod llm;
mod tmux;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use config::Config;
use guard::GuardRailsEngine;
use llm::LlmClient;
use std::str::FromStr;
use std::time::Duration;
use tmux::TmuxClient;
use tokio::time;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::from_str(log_level)?)
        .init();

    info!("Starting tmux-babysitter");

    let config = load_config(&args.config)?;

    info!("Config loaded for tmux session: {}", config.tmux.session);

    let tmux_client = TmuxClient::new(config.tmux.clone());
    let llm_client = LlmClient::new(config.llm.clone())?;
    let guard_engine = GuardRailsEngine::new(config.guard_rails.clone());

    if args.dry_run {
        warn!("DRY RUN MODE: No actual responses will be sent");
    }

    info!(
        "Starting to monitor tmux session every {}ms",
        config.monitoring_interval_ms
    );

    let mut interval = time::interval(Duration::from_millis(config.monitoring_interval_ms));

    loop {
        interval.tick().await;

        match monitor_once(&tmux_client, &llm_client, &guard_engine, args.dry_run).await {
            Ok(_) => {}
            Err(e) => {
                error!("Error during monitoring cycle: {}", e);
            }
        }
    }
}

fn load_config(path: &std::path::Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).context("Failed to read config file")?;
    let config: Config = toml::from_str(&content).context("Failed to parse config file")?;
    Ok(config)
}

async fn monitor_once(
    tmux_client: &TmuxClient,
    llm_client: &LlmClient,
    guard_engine: &GuardRailsEngine,
    dry_run: bool,
) -> Result<()> {
    let output = tmux_client
        .capture_pane()
        .await
        .context("Failed to capture tmux pane")?;

    if output.is_empty() {
        return Ok(());
    }

    let rule_name = llm_client
        .analyze_output(&output)
        .await
        .context("Failed to analyze output with LLM")?;

    if rule_name != "NONE" {
        let response = guard_engine.determine_response(&rule_name);

        if dry_run {
            info!(
                "[DRY RUN] Would send response: '{}' for rule: '{}'",
                response, rule_name
            );
        } else {
            tmux_client
                .send_keys(&response)
                .await
                .context("Failed to send keys to tmux")?;

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    Ok(())
}
