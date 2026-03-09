use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// Optional file-based debug logger for LLM interactions.
/// Logs captured pane content, the prompt sent to LLM, and LLM responses.
pub struct DebugLog {
    file: Mutex<File>,
}

impl DebugLog {
    pub fn new(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub fn log_capture(&self, pane_output: &str) {
        self.write_section("PANE CAPTURE", pane_output);
    }

    pub fn log_llm_request(&self, system_prompt: &str, user_prompt: &str) {
        self.write_section("SYSTEM PROMPT", system_prompt);
        self.write_section("USER PROMPT", user_prompt);
    }

    pub fn log_llm_response(&self, result: &str) {
        self.write_section("LLM RESPONSE", result);
    }

    pub fn log_action(&self, rule: &str, response: &str, dry_run: bool) {
        let prefix = if dry_run { "[DRY RUN] " } else { "" };
        self.write_section("ACTION", &format!("{}rule='{}' response='{}'", prefix, rule, response));
    }

    pub fn log_skip(&self, reason: &str) {
        self.write_section("SKIP", reason);
    }

    fn write_section(&self, label: &str, content: &str) {
        if let Ok(mut f) = self.file.lock() {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(f, "=== [{ts}] {label} ===");
            let _ = writeln!(f, "{content}");
            let _ = writeln!(f);
        }
    }
}
