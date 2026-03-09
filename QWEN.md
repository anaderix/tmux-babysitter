## Qwen Added Memories
### Project Overview
tmux-babysitter: Rust application that monitors tmux sessions and automatically responds to yes/no questions using LLM (Ollama/OpenAI). Features configurable TOML-based guard rails for safety, protects against destructive operations with 60+ safety rules, includes dry-run mode, verbose logging, Makefile for build automation, cross-platform (Linux/macOS).

### Requirements & Design Choices

#### Why Rust?
- Performance: Efficient for continuous monitoring loops
- Safety: Memory safety, no garbage collection pauses
- Concurrency: tokio async runtime for I/O operations
- Cross-platform: Easy binary distribution (Linux/macOS)

#### Why LLM instead of Regex?
- Intelligent: Understands context and intent
- Flexible: Adapts to different prompt styles
- Guard rails: Declarative rules via TOML configuration
- Safety: Can analyze complex questions, not just patterns

#### Why TOML for Configuration?
- Human-readable: Easy to edit manually
- Type-safe: Maps directly to Rust structs
- Comments: Allows inline documentation
- Hierarchical: Good for nested configs

#### Architecture Decisions
- **LLM Client**: OpenAI-compatible API (works with Ollama, OpenAI, local models)
- **Tmux Integration**: System commands (capture-pane, send-keys) - no tmux protocol complexity
- **Monitoring Loop**: Configurable interval (500-1000ms) for balance of speed vs. resource usage
- **Default Response**: Always "no" for safety (defense-in-depth)
- **Dry-run Mode**: Essential for testing without risk

#### Guard Rail Strategy
- Explicit allow-list for safe operations
- Comprehensive block-list for destructive operations
- Rule matching via LLM classification
- 60+ predefined rules covering: filesystem, data, git, packages, credentials, containers, cloud

#### Safety Features
- Dry-run mode for testing
- Verbose logging of all decisions
- Default to "no" when uncertain
- Fast monitoring (500ms) for quick response
- tmux-babysitter now has QWEN.md file documenting project requirements and design choices (Rust rationale, LLM vs regex, TOML config, architecture, guard rail strategy, safety features)

### Numbered Menu Confirmation Logic (March 2026)

**Problem:** AI assistants like Claude use numbered menus instead of text prompts:
```
 ❯ 1. Yes
   2. No
```
Or 3-option menus:
```
 ❯ 1. Yes
   2. Yes, always
   3. No
```

**Solution:** Position-aware response system

**LLM Response Format:**
- Text prompts: `file_delete` → uses config `response` value
- Numbered menus: `file_delete:3` → sends position `3` directly

**Response Flow:**
1. LLM analyzes terminal output for numbered menu patterns
2. If menu detected, LLM returns `rule_name:position` (e.g., `file_delete:3`)
3. `main.rs` parses the colon format:
   - If position present: use it directly
   - If no position: fallback to backward compatibility (`yes`→`1`, `no`→`2`)
4. Send single keypress (number) to tmux - no Enter needed

**Backward Compatibility:**
- Existing configs with `response = "yes"` still work (converts to `1`)
- Existing configs with `response = "no"` still work (converts to `2`)
- LLM intelligently detects when "No" is at position 3 in 3-option menus

**Key Files Modified:**
- `src/llm.rs`: System prompt with position detection instructions and examples
- `src/main.rs`: Parse `rule:position` format, yes/no fallback mapping
- `src/tmux.rs`: `send_keys_no_enter` - sends number without Enter
- `config.safeguard.toml`: Added `menu_select_*` rules for explicit handling

**Example Scenarios:**
| Terminal Output | LLM Returns | Sends |
|-----------------|-------------|-------|
| `Delete? (yes/no)` | `file_delete` | `2` |
| `1. Yes  2. No` | `file_delete:2` | `2` |
| `1. Yes  2. Always  3. No` | `file_delete:3` | `3` |
| `1. Continue  2. Cancel` | `continue_confirmation:1` | `1` |

### Session Termination Detection & Log Noise Reduction (March 2026)

**Problem 1:** When tmux session stops, babysitter endlessly logs errors:
```
ERROR tmux_babysitter: Error during monitoring cycle: Failed to capture tmux pane
ERROR tmux_babysitter::tmux: tmux capture-pane failed: can't find window: 4
```

**Problem 2:** Normal operation logs too much noise:
```
INFO tmux_babysitter::llm: LLM analysis result: NONE
```
(repeats every 500ms when no prompt is present)

**Solution 1: Graceful Session Termination**
- Added `SessionNotFoundError` custom error type in `src/tmux.rs`
- `capture_pane()` detects "can't find" or "no such session" errors
- Main loop catches this error, logs shutdown message, and exits cleanly

**Solution 2: Log Noise Suppression**
- Changed "LLM analysis result: NONE" from `info!` to `debug!`
- Only appears with `--verbose` flag
- Normal operation is now silent unless taking action

**Key Files Modified:**
- `src/tmux.rs`: Added `SessionNotFoundError`, updated `capture_pane()` to detect missing sessions, fixed clippy warning (`format!` → `.to_string()`)
- `src/main.rs`: Catch `SessionNotFoundError` and exit with info message
- `src/llm.rs`: Changed `info!` to `debug!` for NONE results, removed unused import

### Rapid Response for Chained Confirmations (March 2026)

**Problem:** Some tools show multiple yes/no confirmations in rapid succession (e.g., "Delete file? [y/n]" immediately followed by "Remove directory? [y/n]"). The normal monitoring loop has a 500-1000ms gap between checks after sending a response, which can miss the second prompt.

**Solution:** After sending a response, enter a short "burst mode" of rapid checks to catch chained confirmations.

**Behavior:**
- Normal monitoring: checks every `monitoring_interval_ms` (500-1000ms)
- After sending a response: enter rapid response mode for a burst of checks
- Rapid mode performs `count` checks, each separated by `interval_ms`
- Then returns to normal monitoring interval

**Default Configuration:**
- `enabled: true` - rapid response is on by default
- `interval_ms: 200` - check every 200ms during burst mode
- `count: 5` - perform 5 rapid checks (total window: ~800ms)

**Example Timeline:**
1. Prompt detected → Response sent (e.g., "2" for "No")
2. Wait 200ms → Check pane → No new prompt
3. Wait 200ms → Check pane → Prompt found! → Send response
4. Wait 200ms → Check pane → No new prompt
5. Wait 200ms → Check pane → No new prompt
6. Wait 200ms → Check pane → No new prompt
7. Exit burst mode, return to normal monitoring

**Key Files Modified:**
- `src/config.rs`: Added `RapidResponse` struct with `enabled`, `interval_ms`, `count` fields; implemented `Default` trait
- `config.example.toml`: Added `[rapid_response]` section with defaults and documentation
- `src/main.rs`:
  - `monitor_once()` now returns `Result<bool>` indicating if a response was sent
  - Added `rapid_response_loop()` function for burst checking
  - Main loop triggers rapid response after successful response (when not in dry-run)
  - Added `debug` import for detailed logging

### Generic Menu Handling (March 2026)

**Problem:** Generic numbered menus like "Do you want to proceed?" with options "1. Yes  2. No" don't match any specific rule, causing the LLM to return "NONE" and the babysitter to ignore the prompt.

**Solution:** Added `generic_proceed` rule and updated LLM prompt to handle generic menus intelligently.

**Behavior:**
- When LLM detects a generic numbered menu with unclear context:
  - If NO destructive action is indicated in terminal output → responds with `generic_proceed:1` (safely say Yes)
  - If destructive action IS clearly indicated → uses specific destructive rule (e.g., `file_delete:2`)
- The `generic_proceed` rule in config maps to response "1" (first option, typically "Yes")

**Example:**
```
Terminal: "Do you want to proceed?"
          " ❯ 1. Yes"
          "   2. No"
```
- LLM sees generic menu, no destructive context → returns "generic_proceed:1"
- Guard rails maps to "1" → sends "1" to tmux (selects "Yes")

**Key Files Modified:**
- `src/llm.rs`: Added CRITICAL instruction about generic menus in system prompt; added `generic_proceed` to available rules list; added example for safe 3-option menus
- `config.safeguard.toml`: Added `generic_proceed` rule with response "1"

**Important Note:** The LLM prompt now includes examples for:
- 2-option destructive menus (e.g., "Yes  2. No" → respond with No position)
- 3-option destructive menus (e.g., "Yes  2. Always  3. No" → respond with No position)
- 3-option SAFE menus (e.g., "Yes  2. Always  3. No" for syntax check → respond with Yes position via `generic_proceed:1`)
