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
use tmux::{SessionNotFoundError, TmuxClient};
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
                // Check if the session has stopped
                if e.downcast_ref::<SessionNotFoundError>().is_some() {
                    info!("Tmux session '{}' has stopped. Exiting babysitter.", config.tmux.session);
                    return Ok(());
                }

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

    let llm_result = llm_client
        .analyze_output(&output)
        .await
        .context("Failed to analyze output with LLM")?;

    if llm_result != "NONE" {
        // Parse rule_name:position format (e.g., "file_delete:3")
        let (rule_name, position) = if let Some(colon_pos) = llm_result.find(':') {
            let rule = &llm_result[..colon_pos];
            let pos = &llm_result[colon_pos + 1..];
            (rule, Some(pos.to_string()))
        } else {
            (llm_result.as_str(), None)
        };

        let response = guard_engine.determine_response(rule_name);

        // Determine final response to send:
        // 1. If LLM provided explicit position, use it
        // 2. Otherwise, convert yes/no to positions for backward compatibility
        let final_response = if let Some(pos) = position {
            pos
        } else {
            // Convert yes/no to numbered menu selections for backward compatibility
            // "yes" -> "1" (first option), "no" -> "2" (second option)
            match response.to_lowercase().as_str() {
                "yes" => "1".to_string(),
                "no" => "2".to_string(),
                _ => response,
            }
        };

        if dry_run {
            info!(
                "[DRY RUN] Would send response: '{}' for rule: '{}'",
                final_response, rule_name
            );
        } else {
            // Send the number (no Enter needed for tmux menu selection)
            tmux_client
                .send_keys_no_enter(&final_response)
                .await
                .context("Failed to send keys to tmux")?;

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    Ok(())
}
