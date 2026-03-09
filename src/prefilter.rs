use tracing::debug;

/// Cheap regex-free check to determine if terminal output likely contains
/// a question or menu that needs LLM analysis. Returns true if the output
/// looks like it has a prompt worth analyzing.
pub fn has_question(output: &str) -> bool {
    // Only look at the last 20 lines (same window the LLM sees)
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(20);
    let tail = &lines[start..];

    for line in tail {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();

        // Yes/no prompt patterns
        if lower.contains("(y/n)")
            || lower.contains("(yes/no)")
            || lower.contains("[y/n]")
            || lower.contains("[yes/no]")
            || lower.contains("(y/n/a)")
            || lower.contains("[y/n/a]")
        {
            debug!("Prefilter matched yes/no pattern: {}", trimmed);
            return true;
        }

        // Numbered menu patterns: "1.", "[1]", "❯ 1."
        if is_numbered_menu_line(trimmed) {
            debug!("Prefilter matched numbered menu: {}", trimmed);
            return true;
        }

        // Question mark at end of a line (with possible trailing whitespace/punctuation)
        if trimmed.ends_with('?') || trimmed.ends_with("? ") {
            debug!("Prefilter matched question mark: {}", trimmed);
            return true;
        }

        // Common prompt keywords followed by question-like context
        if (lower.contains("confirm")
            || lower.contains("proceed")
            || lower.contains("continue")
            || lower.contains("overwrite")
            || lower.contains("delete")
            || lower.contains("remove")
            || lower.contains("allow")
            || lower.contains("accept"))
            && (lower.contains('?') || lower.contains("(") || lower.contains("["))
        {
            debug!("Prefilter matched keyword pattern: {}", trimmed);
            return true;
        }
    }

    false
}

/// Check if a line looks like a numbered menu option.
/// Matches patterns like: "1. Yes", "  2. No", "[1] Yes", "❯ 1. Yes", "> 1. Yes"
fn is_numbered_menu_line(line: &str) -> bool {
    let trimmed = line
        .trim_start_matches(|c: char| c.is_whitespace() || c == '❯' || c == '>');
    let trimmed = trimmed.trim_start();

    // "1. Something" or "1) Something"
    if let Some(first) = trimmed.chars().next() {
        if first.is_ascii_digit() {
            let rest = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
            if rest.starts_with(". ") || rest.starts_with(") ") || rest.starts_with("] ") {
                return true;
            }
        }
    }

    // "[1] Something"
    if trimmed.starts_with('[') {
        let rest = trimmed.trim_start_matches('[');
        let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        if rest.starts_with(']') {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_yes_no_prompts() {
        assert!(has_question("Delete file? (y/n)"));
        assert!(has_question("Overwrite? (yes/no)"));
        assert!(has_question("Continue? [Y/n]"));
        assert!(has_question("Proceed? [yes/no]"));
    }

    #[test]
    fn detects_numbered_menus() {
        let menu = "Do you want to proceed?\n❯ 1. Yes\n  2. No";
        assert!(has_question(menu));

        let menu2 = "Select option:\n[1] Continue\n[2] Cancel";
        assert!(has_question(menu2));

        let menu3 = "Choose:\n1) Accept\n2) Reject";
        assert!(has_question(menu3));
    }

    #[test]
    fn detects_question_marks() {
        assert!(has_question("Are you sure?"));
        assert!(has_question("Do you want to continue?"));
    }

    #[test]
    fn detects_keyword_patterns() {
        assert!(has_question("Confirm deletion? [yes/no]"));
        assert!(has_question("Proceed with overwrite? (y/n)"));
    }

    #[test]
    fn ignores_normal_output() {
        assert!(!has_question("Building project..."));
        assert!(!has_question("Compiling src/main.rs"));
        assert!(!has_question(""));
        assert!(!has_question("   \n   \n   "));
        assert!(!has_question("Downloaded 50 crates in 2.3s"));
    }

    #[test]
    fn ignores_question_marks_in_urls_or_code() {
        // Standalone question mark at end of line should match
        assert!(has_question("Ready to deploy?"));
        // But normal log lines without question patterns should not
        assert!(!has_question("Fetching https://example.com/api"));
        assert!(!has_question("Processing 100 items"));
    }
}
