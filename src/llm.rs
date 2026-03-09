use crate::config::{GuardRule, LlmConfig};
use crate::debuglog::DebugLog;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
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
    system_prompt: String,
}

impl LlmClient {
    pub fn new(config: LlmConfig, rules: &[GuardRule], default_response: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let system_prompt = build_system_prompt(rules, default_response);

        Ok(Self {
            client,
            config,
            system_prompt,
        })
    }

    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/models", self.config.base_url);
        let mut request_builder = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        request_builder
            .send()
            .await
            .context(format!("LLM endpoint unreachable at {}", self.config.base_url))?;

        Ok(())
    }

    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    pub async fn analyze_output(&self, output: &str, debug_log: Option<&DebugLog>) -> Result<String> {
        let lines: Vec<&str> = output.lines().collect();
        let tail = &lines[lines.len().saturating_sub(20)..];
        let user_prompt = format!("Analyze this terminal output:\n\n{}", tail.join("\n"));

        if let Some(dl) = debug_log {
            dl.log_llm_request(&self.system_prompt, &user_prompt);
        }

        debug!("Sending request to LLM");

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.system_prompt.clone(),
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

        if let Some(dl) = debug_log {
            dl.log_llm_response(&content);
        }

        debug!("LLM analysis result: {}", content);
        Ok(content)
    }
}

fn build_system_prompt(rules: &[GuardRule], default_response: &str) -> String {
    let mut destructive_rules = Vec::new();
    let mut safe_rules = Vec::new();
    let mut menu_rules = Vec::new();

    for rule in rules {
        match rule.response.to_lowercase().as_str() {
            "no" => destructive_rules.push(rule),
            "yes" => safe_rules.push(rule),
            _ => menu_rules.push(rule),
        }
    }

    let mut prompt = String::from(
        r#"You are a monitoring assistant. Analyze the terminal output and determine if there's a question that requires a response.

IMPORTANT: If you detect a numbered menu (options like "1. Yes  2. No" or "[1] Yes [2] No"), you MUST include the position number of the correct answer.

Response format:
- For text-based yes/no questions: respond with just the rule name (e.g., "file_delete")
- For numbered menus: respond with "rule_name:position" where position is the number to select (e.g., "file_delete:3" if "No" is option 3)

CRITICAL: Generic numbered menus (like "Do you want to proceed?", "Confirm action?", etc.) where the context is unclear:
- If the menu shows "1. Yes  2. No" and there's NO indication of destructive action, respond: "generic_proceed:1" (safe default)
- Only use destructive rules (file_delete, etc.) when the terminal output CLEARLY indicates a destructive action

Available guard rules:
"#,
    );

    if !destructive_rules.is_empty() {
        prompt.push_str("Destructive operations (answer NO - find which position is \"No\"):\n");
        for rule in &destructive_rules {
            prompt.push_str(&format!("- {}: {}\n", rule.name, rule.description));
        }
        prompt.push('\n');
    }

    if !safe_rules.is_empty() {
        prompt.push_str("Safe operations (answer YES - usually position 1):\n");
        for rule in &safe_rules {
            prompt.push_str(&format!("- {}: {}\n", rule.name, rule.description));
        }
        prompt.push('\n');
    }

    if !menu_rules.is_empty() {
        prompt.push_str("Menu selection rules (use the configured position):\n");
        for rule in &menu_rules {
            prompt.push_str(&format!(
                "- {}: {} (select position {})\n",
                rule.name, rule.description, rule.response
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str(&format!(
        "Default response when no rule matches: \"{}\"\n\n",
        default_response
    ));

    prompt.push_str(
        r#"Examples:
- Terminal: "Delete file? (yes/no)" → Response: "file_delete"
- Terminal: "1. Yes  2. No" + destructive op → Response: "file_delete:2"
- Terminal: "1. Yes  2. Always  3. No" + destructive op → Response: "file_delete:3"
- Terminal: "1. Yes  2. Always  3. No" + safe op (no destruction) → Response: "generic_proceed:1"
- Terminal: "1. Continue  2. Cancel" + safe op → Response: "continue_confirmation:1"

Respond ONLY with the rule name or "NONE", optionally followed by ":position" for numbered menus."#,
    );

    prompt
}
