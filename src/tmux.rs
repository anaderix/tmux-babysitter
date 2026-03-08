use crate::config::TmuxConfig;
use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{debug, error, info};

pub struct TmuxClient {
    config: TmuxConfig,
}

impl TmuxClient {
    pub fn new(config: TmuxConfig) -> Self {
        Self { config }
    }

    fn build_target(&self) -> String {
        let mut target = format!("{}", self.config.session);
        if let Some(window) = &self.config.window {
            target.push_str(&format!(":{}", window));
        }
        if let Some(pane) = &self.config.pane {
            target.push_str(&format!(".{}", pane));
        }
        target
    }

    pub async fn capture_pane(&self) -> Result<String> {
        let target = self.build_target();
        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", &target])
            .output()
            .await
            .context("Failed to execute tmux capture-pane")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("tmux capture-pane failed: {}", stderr);
            anyhow::bail!("tmux capture-pane failed: {}", stderr);
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("Captured pane content ({} bytes)", content.len());
        Ok(content)
    }

    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        let target = self.build_target();
        info!("Sending keys to {}: '{}'", target, keys);

        let output = Command::new("tmux")
            .args(["send-keys", "-t", &target, keys, "C-m"])
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
