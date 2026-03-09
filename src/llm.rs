use crate::config::LlmConfig;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

pub struct LlmClient {
    client: Client,
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = Client::builder()
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    pub async fn analyze_output(&self, output: &str) -> Result<String> {
        let system_prompt = r#"You are a monitoring assistant. Analyze the terminal output and determine if there's a question that requires a response.

IMPORTANT: If you detect a numbered menu (options like "1. Yes  2. No" or "[1] Yes [2] No"), you MUST include the position number of the correct answer.

Response format:
- For text-based yes/no questions: respond with just the rule name (e.g., "file_delete")
- For numbered menus: respond with "rule_name:position" where position is the number to select (e.g., "file_delete:3" if "No" is option 3)

Available guard rules:
Destructive operations (answer NO - find which position is "No"):
- file_delete: Asks confirmation to delete, remove, or erase files or directories
- recursive_delete: Asks to recursively delete directories
- disk_format: Asks to format, wipe, or erase disk drives or partitions
- disk_erase: Asks to securely erase or zero out disks
- partition_delete: Asks to delete disk partitions
- database_drop: Asks to drop, delete, or truncate databases or tables
- database_wipe: Asks to clear, empty, or truncate tables with data
- backup_delete: Asks to delete backups or archive files
- log_delete: Asks to delete or rotate log files
- cache_delete: Asks to delete caches that might contain important work
- git_force_delete_branch: Asks to force delete a git branch
- git_reset_hard: Asks to perform git reset --hard
- git_force_push: Asks to force push to remote
- git_amend_public: Asks to amend already-pushed commits
- git_clean_force: Asks to run git clean with force
- package_remove: Asks to remove, uninstall, or purge packages
- package_uninstall: Asks to uninstall dependencies or packages
- dependency_delete: Asks to delete node_modules, vendor, or similar directories
- system_upgrade: Asks to upgrade entire system
- ssh_key_delete: Asks to delete SSH keys
- credential_delete: Asks to delete credentials, tokens, or API keys
- certificate_delete: Asks to delete SSL/TLS certificates
- keychain_delete: Asks to delete or reset keychain/password store
- process_kill_force: Asks to force kill processes (kill -9, SIGKILL)
- system_shutdown: Asks to shutdown, reboot, or halt the system
- service_stop: Asks to stop critical system services
- config_delete: Asks to delete configuration files
- config_overwrite: Asks to overwrite configuration files with new content
- settings_reset: Asks to reset or clear application settings
- data_upload: Asks to upload sensitive data to external servers
- file_send: Asks to send files to external recipients or servers
- execute_remote: Asks to download and execute code from the internet
- docker_delete_container: Asks to delete Docker containers with data
- docker_delete_volume: Asks to delete Docker volumes (which contain data)
- vm_delete: Asks to delete virtual machines
- sandbox_reset: Asks to reset or wipe development sandboxes
- cloud_delete_resource: Asks to delete cloud resources (EC2, S3 buckets, etc.)
- cloud_terminate: Asks to terminate cloud instances or services
- cloud_wipe_data: Asks to wipe or delete data from cloud storage
- overwrite_file: Asks to overwrite an existing file with new content
- truncate_file: Asks to truncate or zero out files
- clear_command_history: Asks to clear command history

Safe operations (answer YES - usually position 1):
- continue_confirmation: Asks to continue with a non-destructive process
- package_install: Asks to install new packages or dependencies
- dependency_install: Asks to install npm, pip, cargo, or other dependencies
- build_confirmation: Asks confirmation to build or compile code
- test_confirmation: Asks to run tests
- git_commit: Asks to create a git commit
- git_push: Asks to push to remote repository (non-force)
- git_pull: Asks to pull from remote repository
- git_checkout: Asks to checkout a branch
- git_merge: Asks to merge changes
- docker_pull: Asks to pull Docker images
- docker_run: Asks to run a Docker container
- docker_build: Asks to build a Docker image
- database_migration: Asks to run database migrations (non-destructive)
- deployment_confirmation: Asks to deploy code to non-production environments

Examples:
- Terminal: "Delete file? (yes/no)" → Response: "file_delete"
- Terminal: "1. Yes  2. No" + destructive op → Response: "file_delete:2"
- Terminal: "1. Yes  2. Always  3. No" + destructive op → Response: "file_delete:3"
- Terminal: "1. Continue  2. Cancel" + safe op → Response: "continue_confirmation:1"

Respond ONLY with the rule name or "NONE", optionally followed by ":position" for numbered menus."#;

        let user_prompt = format!(
            "Analyze this terminal output:\n\n{}",
            output.lines().rev().take(20).collect::<Vec<_>>().join("\n")
        );

        debug!("Sending request to LLM");

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            temperature: 0.1,
        };

        let mut request_builder = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .json(&request);

        if let Some(api_key) = &self.config.api_key {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder
            .send()
            .await
            .context("Failed to send request to LLM")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM request failed with status {}: {}", status, body);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse LLM response")?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_else(|| "NONE".to_string());

        debug!("LLM analysis result: {}", content);
        Ok(content)
    }
}
