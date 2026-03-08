use crate::config::{GuardRails, GuardRule};
use tracing::{debug, info, warn};

pub struct GuardRailsEngine {
    rails: GuardRails,
}

impl GuardRailsEngine {
    pub fn new(rails: GuardRails) -> Self {
        Self { rails }
    }

    pub fn get_response(&self, rule_name: &str) -> Option<&GuardRule> {
        if rule_name == "NONE" {
            return None;
        }

        self.rails
            .rules
            .iter()
            .find(|rule| rule.name == rule_name)
            .or_else(|| {
                warn!("Unknown guard rule '{}', using default response", rule_name);
                None
            })
    }

    pub fn get_default_response(&self) -> &str {
        &self.rails.default_response
    }

    pub fn determine_response(&self, rule_name: &str) -> String {
        if let Some(rule) = self.get_response(rule_name) {
            debug!(
                "Matched rule '{}': {} -> '{}'",
                rule.name, rule.description, rule.response
            );
            info!("Response: {}", rule.response);
            rule.response.clone()
        } else {
            debug!("No rule matched, using default response");
            let response = self.get_default_response().to_string();
            info!("Default response: {}", response);
            response
        }
    }
}
