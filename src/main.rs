mod cli;
mod config;
mod debuglog;
mod guard;
mod llm;
mod prefilter;
mod tmux;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use config::Config;
use debuglog::DebugLog;
use guard::GuardRailsEngine;
use llm::LlmClient;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tmux::{SessionNotFoundError, TmuxClient};
use tokio::time;
use tracing::{debug, error, info, warn};

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
    let llm_client = LlmClient::new(
        config.llm.clone(),
        &config.guard_rails.rules,
        &config.guard_rails.default_response,
    )?;
    let guard_engine = GuardRailsEngine::new(config.guard_rails.clone());

    if config.guard_rails.rules.is_empty() {
        warn!("No guard rules configured — all questions will use default response '{}'", config.guard_rails.default_response);
    }

    // Validate tmux session and LLM endpoint before entering the loop
    tmux_client
        .check_session()
        .await
        .context("Startup check failed: tmux session not found")?;
    info!("Tmux session '{}' is active", config.tmux.session);

    llm_client
        .health_check()
        .await
        .context("Startup check failed: cannot reach LLM endpoint")?;
    info!("LLM endpoint at '{}' is reachable", config.llm.base_url);

    let debug_log: Option<Arc<DebugLog>> = if let Some(ref path) = args.debug_log {
        let dl = DebugLog::new(path).context("Failed to open debug log file")?;
        info!("Debug logging to: {}", path.display());
        // Log the system prompt once at startup
        dl.log_llm_request(llm_client.system_prompt(), "(startup — system prompt logged)");
        Some(Arc::new(dl))
    } else {
        None
    };

    if args.dry_run {
        warn!("DRY RUN MODE: No actual responses will be sent");
    }

    info!(
        "Starting to monitor tmux session every {}ms",
        config.monitoring_interval_ms
    );

    let mut interval = time::interval(Duration::from_millis(config.monitoring_interval_ms));
    let mut state = MonitorState::new();

    loop {
        interval.tick().await;

        match monitor_once(&tmux_client, &llm_client, &guard_engine, args.dry_run, &mut state, debug_log.as_deref()).await {
            Ok(response_sent) => {
                if response_sent && !args.dry_run {
                    // Enter rapid response mode to catch chained confirmations
                    if let Err(e) = rapid_response_loop(
                        &tmux_client,
                        &llm_client,
                        &guard_engine,
                        args.dry_run,
                        &config.rapid_response,
                        &mut state,
                        debug_log.as_deref(),
                    )
                    .await
                    {
                        if e.downcast_ref::<SessionNotFoundError>().is_some() {
                            info!("Tmux session '{}' has stopped. Exiting babysitter.", config.tmux.session);
                            return Ok(());
                        }
                        error!("Error during rapid response: {}", e);
                    }
                }
            }
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

/// Tracks monitoring state across cycles to avoid redundant LLM calls.
struct MonitorState {
    /// Hash of the full pane capture, for exact-match dedup.
    last_capture_hash: Option<u64>,
    /// The last LLM result we acted on, to suppress duplicate responses.
    last_response_llm_result: Option<String>,
    /// Hash of the last 20 lines sent to LLM, to skip when only
    /// non-visible parts of the pane changed.
    last_llm_input_hash: Option<u64>,
}

impl MonitorState {
    fn new() -> Self {
        Self {
            last_capture_hash: None,
            last_response_llm_result: None,
            last_llm_input_hash: None,
        }
    }
}

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

async fn monitor_once(
    tmux_client: &TmuxClient,
    llm_client: &LlmClient,
    guard_engine: &GuardRailsEngine,
    dry_run: bool,
    state: &mut MonitorState,
    debug_log: Option<&DebugLog>,
) -> Result<bool> {
    let output = tmux_client
        .capture_pane()
        .await
        .context("Failed to capture tmux pane")?;

    // Strip trailing blank lines — tmux capture-pane includes the full
    // visible pane height, padding with empty lines below the actual content.
    // Without trimming, the "last 20 lines" window can be entirely blank.
    let output = output.trim_end().to_string();

    if output.is_empty() {
        return Ok(false);
    }

    let current_hash = hash_string(&output);
    if state.last_capture_hash == Some(current_hash) {
        debug!("Pane content unchanged, skipping");
        return Ok(false);
    }
    state.last_capture_hash = Some(current_hash);

    if let Some(dl) = debug_log {
        dl.log_capture(&output);
    }

    if !prefilter::has_question(&output) {
        debug!("Prefilter: no question detected, skipping LLM call");
        // No question on screen — clear the trackers so the next
        // real question (even if it matches a previous one) gets answered.
        state.last_response_llm_result = None;
        state.last_llm_input_hash = None;
        return Ok(false);
    }

    // Hash the last 20 lines (the portion we send to the LLM) to avoid
    // re-querying when only non-visible parts of the pane changed.
    let lines: Vec<&str> = output.lines().collect();
    let tail = &lines[lines.len().saturating_sub(20)..];
    let tail_str: String = tail.join("\n");
    let tail_hash = hash_string(&tail_str);

    if state.last_llm_input_hash == Some(tail_hash) {
        debug!("LLM input (last 20 lines) unchanged, skipping LLM call");
        return Ok(false);
    }
    // LLM input changed — clear the duplicate response tracker so that
    // a new question producing the same LLM result (e.g., two consecutive
    // "generic_proceed:1" for different commands) still gets answered.
    state.last_llm_input_hash = Some(tail_hash);
    state.last_response_llm_result = None;

    debug!("Prefilter: possible question detected, calling LLM");

    let llm_result = llm_client
        .analyze_output(&output, debug_log)
        .await
        .context("Failed to analyze output with LLM")?;

    if llm_result == "NONE" {
        if let Some(dl) = debug_log {
            dl.log_skip("LLM returned NONE");
        }
        return Ok(false);
    }

    // Check if we already responded to this exact LLM result
    if state.last_response_llm_result.as_deref() == Some(&llm_result) {
        debug!(
            "Duplicate response suppressed: already answered '{}'",
            llm_result
        );
        if let Some(dl) = debug_log {
            dl.log_skip(&format!("Duplicate suppressed: already answered '{}'", llm_result));
        }
        return Ok(false);
    }

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

    if let Some(dl) = debug_log {
        dl.log_action(rule_name, &final_response, dry_run);
    }

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
    }

    // Record what we responded to, so we don't send it again
    state.last_response_llm_result = Some(llm_result);

    Ok(true)
}

async fn rapid_response_loop(
    tmux_client: &TmuxClient,
    llm_client: &LlmClient,
    guard_engine: &GuardRailsEngine,
    dry_run: bool,
    rapid_config: &config::RapidResponse,
    state: &mut MonitorState,
    debug_log: Option<&DebugLog>,
) -> Result<()> {
    if !rapid_config.enabled {
        return Ok(());
    }

    debug!(
        "Entering rapid response mode: {} checks every {}ms",
        rapid_config.count, rapid_config.interval_ms
    );

    for i in 0..rapid_config.count {
        tokio::time::sleep(Duration::from_millis(rapid_config.interval_ms)).await;

        match monitor_once(tmux_client, llm_client, guard_engine, dry_run, state, debug_log).await {
            Ok(response_sent) => {
                if response_sent {
                    debug!("Rapid response {}: action taken", i + 1);
                }
            }
            Err(e) => {
                // Check if the session has stopped
                if e.downcast_ref::<SessionNotFoundError>().is_some() {
                    return Err(e);
                }
                // Log other errors but continue rapid response mode
                debug!("Rapid response {}: error occurred - {}", i + 1, e);
            }
        }
    }

    debug!("Exiting rapid response mode");
    Ok(())
}
