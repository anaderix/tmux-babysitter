# Tech Debt Fixes

## ~~1. Regex pre-filter before LLM call~~ DONE
**Priority: High | Impact: High | Completed: 5282b66**

Add a cheap regex check in `monitor_once()` before calling `llm_client.analyze_output()`. Only invoke the LLM when the terminal output looks like it contains a question or menu.

Patterns to detect:
- `(y/n)`, `(yes/no)`, `[Y/n]`, `[y/N]` and variations
- `? ` or `?` at end of a non-empty line
- Numbered menu patterns: `1.`, `[1]`, `❯`, `>` followed by option text
- Common prompt keywords: `confirm`, `proceed`, `continue`, `overwrite`, `delete`

New module: `src/prefilter.rs` with a `has_question(output: &str) -> bool` function.

**Files to change:** `src/prefilter.rs` (new), `src/main.rs` (call prefilter before LLM)

## ~~2. Change detection (skip identical captures)~~ DONE
**Priority: High | Impact: High | Completed: 5282b66**

Store a hash of the last captured pane content. If the new capture matches, skip analysis entirely. This avoids redundant LLM calls when nothing changed on screen.

Add a `last_capture_hash: Option<u64>` state to the monitoring loop. Use a fast hash (e.g., std `DefaultHasher`).

**Files to change:** `src/main.rs` (add hash state, compare before calling monitor_once)

## ~~3. Duplicate response tracking~~ DONE
**Priority: Medium | Impact: Medium | Completed: 5282b66**

After sending a response, record what was answered (e.g., hash of the pane content + rule name). On subsequent cycles, if the same question is still on screen, don't send the response again.

This prevents double-keypresses when the pane hasn't refreshed between cycles.

**Files to change:** `src/main.rs` (add last-response tracking state)

## ~~4. Build LLM prompt from config rules~~ DONE
**Priority: Medium | Impact: Medium**

Instead of a hardcoded rule list in the system prompt (`llm.rs`), generate the prompt dynamically from the loaded `guard_rails.rules` config. This eliminates drift between config and prompt.

Pass the rules to `LlmClient::new()` or to `analyze_output()`, and format them into the system prompt at runtime.

**Files to change:** `src/llm.rs` (dynamic prompt generation), `src/main.rs` (pass rules to LlmClient)

## ~~5. Fix reversed line order~~ DONE
**Priority: Low | Impact: Low**

`llm.rs:137` sends last 20 lines in reverse order. Collect them in correct order instead.

```rust
// Before:
output.lines().rev().take(20).collect::<Vec<_>>().join("\n")
// After:
let lines: Vec<_> = output.lines().collect();
lines[lines.len().saturating_sub(20)..].join("\n")
```

**Files to change:** `src/llm.rs`

## ~~6. Add LLM request timeout~~ DONE
**Priority: Low | Impact: Medium**

Add a timeout (e.g., 10s) to the reqwest client in `LlmClient::new()`:
```rust
Client::builder().timeout(Duration::from_secs(10)).build()
```

**Files to change:** `src/llm.rs`

## ~~7. Startup validation~~ DONE
**Priority: Low | Impact: Low**

Before entering the main loop, verify:
- tmux session exists (`tmux has-session -t <target>`)
- LLM endpoint responds (simple health check or test request)
- Config has at least one guard rule

**Files to change:** `src/main.rs`, `src/tmux.rs` (add `check_session()` method), `src/llm.rs` (add `health_check()` method)
