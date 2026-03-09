use crate::config::TmuxConfig;
use anyhow::{bail, Context, Result};
use tokio::process::Command;
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct SessionNotFoundError {
    pub target: String,
}

impl std::fmt::Display for SessionNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Session not found: {}", self.target)
    }
}

impl std::error::Error for SessionNotFoundError {}

pub struct TmuxClient {
    config: TmuxConfig,
}

impl TmuxClient {
    pub fn new(config: TmuxConfig) -> Self {
        Self { config }
    }

    fn build_target(&self) -> String {
        let mut target = self.config.session.to_string();
        if let Some(window) = &self.config.window {
            target.push_str(&format!(":{}", window));
        }
        if let Some(pane) = &self.config.pane {
            target.push_str(&format!(".{}", pane));
        }
        target
    }

    pub async fn check_session(&self) -> Result<()> {
        let target = self.build_target();
        let output = Command::new("tmux")
            .args(["has-session", "-t", &self.config.session])
            .output()
            .await
            .context("Failed to execute tmux has-session. Is tmux installed?")?;

        if !output.status.success() {
            bail!(SessionNotFoundError { target });
        }

        Ok(())
    }

    pub async fn capture_pane(&self) -> Result<String> {
        let target = self.build_target();
        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", &target])
            .output()
            .await
            .context("Failed to execute tmux capture-pane")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            error!("tmux capture-pane failed: {}", stderr);

            // Check if the session/window is gone
            if stderr.contains("can't find") || stderr.contains("no such session") {
                bail!(SessionNotFoundError { target });
            }

            bail!("tmux capture-pane failed: {}", stderr);
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("Captured pane content ({} bytes)", content.len());
        Ok(content)
    }

    pub async fn send_keys_no_enter(&self, keys: &str) -> Result<()> {
        let target = self.build_target();
        info!("Sending keys to {}: '{}'", target, keys);

        let output = Command::new("tmux")
            .args(["send-keys", "-t", &target, keys])
            .output()
            .await
            .context("Failed to execute tmux send-keys")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("tmux send-keys failed: {}", stderr);
            anyhow::bail!("tmux send-keys failed: {}", stderr);
        }

        Ok(())
    }
}
