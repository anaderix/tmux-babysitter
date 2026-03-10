use tracing::debug;

/// Cheap regex-free check to determine if terminal output likely contains
/// an active question or menu that needs LLM analysis. Returns true if the
/// output looks like it has a prompt waiting for input.
///
/// Key distinction: an active prompt has the question/menu at the very bottom
/// of the visible content. Stale questions (e.g., a question Claude asked the
/// user earlier, now scrolled up with new content below) should not match.
pub fn has_question(output: &str) -> bool {
    let lines: Vec<&str> = output.lines().collect();

    // Strong indicators (y/n brackets, numbered menus) — check last 20 lines.
    // These are specific enough to avoid false positives from normal output.
    let start_wide = lines.len().saturating_sub(20);
    let tail_wide = &lines[start_wide..];

    for line in tail_wide {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();

        // Yes/no prompt patterns — very specific, safe to check broadly
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

        // Numbered menu patterns: "❯ 1. Yes", "2. No", "[1] Continue"
        if is_numbered_menu_line(trimmed) {
            debug!("Prefilter matched numbered menu: {}", trimmed);
            return true;
        }
    }

    // Weak indicators (trailing ?, prompt keywords) — only check the last 5
    // non-empty lines. A trailing ? is too common in normal output (e.g., LLM
    // asking a conversational question that's already been answered). Only
    // match if it's near the very bottom, where active prompts appear.
    let non_empty: Vec<&str> = lines
        .iter()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .take(5)
        .copied()
        .collect();

    for line in &non_empty {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();

        if trimmed.ends_with('?') || trimmed.ends_with("? ") {
            debug!("Prefilter matched question mark (bottom): {}", trimmed);
            return true;
        }

        if has_prompt_keyword(&lower)
            && (lower.contains('?')
                || lower.contains("(y")
                || lower.contains("[y")
                || lower.contains("(n")
                || lower.contains("[n"))
        {
            debug!("Prefilter matched keyword pattern (bottom): {}", trimmed);
            return true;
        }
    }

    false
}

/// Check if a lowercase string contains a prompt keyword as a whole word.
/// "confirm" matches "confirm deletion" but NOT "confirmed" or "unconfirmed".
fn has_prompt_keyword(lower: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "confirm", "proceed", "continue", "overwrite", "delete", "remove", "allow", "accept",
    ];
    for keyword in KEYWORDS {
        if let Some(pos) = lower.find(keyword) {
            let before_ok = pos == 0
                || !lower.as_bytes()[pos - 1].is_ascii_alphabetic();
            let end = pos + keyword.len();
            let after_ok = end >= lower.len()
                || !lower.as_bytes()[end].is_ascii_alphabetic();
            if before_ok && after_ok {
                return true;
            }
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
        assert!(has_question("Do you want to continue? (y/n)"));
        assert!(has_question("Allow access? [y/n]"));
    }

    #[test]
    fn ignores_keyword_substrings_and_status_messages() {
        // "confirmed" contains "confirm" but is not a prompt keyword
        assert!(!has_question("SH3 is not confirmed as-is — SLoD-routed (F1=0.207)"));
        // "continued" contains "continue" but is not a prompt
        assert!(!has_question("Processing continued (batch 5 of 10)"));
        // "removed" contains "remove"
        assert!(!has_question("File was removed successfully (3 items)"));
        // "allowed" contains "allow"
        assert!(!has_question("Connection allowed [TLS 1.3]"));
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

    #[test]
    fn ignores_stale_questions_with_new_content_below() {
        // A question from earlier in the session, now scrolled up with
        // new content below it — not an active prompt.
        let stale = "\
Want me to try them?\n\
\n\
❯ yes, run 1-3\n\
\n\
● Agent(Engineer: threshold sweep)\n\
  ⎿  Running...\n\
\n\
* Proofing… (53s)\n\
\n\
❯ \n\
───────────────\n\
  ⏵⏵ accept edits on (shift+tab to cycle)";
        assert!(!has_question(stale));
    }

    #[test]
    fn detects_active_question_at_bottom() {
        // An active question with menu at the bottom — should match
        let active = "\
● Agent(Runner)\n\
  ⎿  Done (35 tool uses)\n\
\n\
─────────────────\n\
 Bash command\n\
\n\
   cd /home/user/project && git status\n\
\n\
 Do you want to proceed?\n\
 ❯ 1. Yes\n\
   2. No\n\
\n\
 Esc to cancel";
        assert!(has_question(active));
    }
}
