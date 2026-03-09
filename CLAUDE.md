# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

tmux-babysitter is a Rust application that monitors a tmux session and automatically answers yes/no questions (and numbered menus) using a cheap LLM. It's designed to keep unattended processes (like cloud code tools) running by responding to confirmation prompts.

## Build & Development Commands

```bash
make build          # Debug build
make release        # Release build
make test           # Run tests
make test-verbose   # Run tests with output (--nocapture)
make fmt            # Format code (cargo fmt)
make clippy         # Lint (cargo clippy -- -D warnings)
make check          # Format + clippy
make run ARGS='-c config.toml --dry-run'   # Run debug build with args
make test-babysitter  # Test with example config in dry-run mode
```

## Architecture

The codebase is a single-binary async Rust application (tokio runtime) with 5 modules:

- **`main.rs`** — Entry point and main monitoring loop. Calls `monitor_once()` each cycle, which captures tmux output → sends to LLM → matches guard rule → sends response. After a successful response, enters a "rapid response" burst loop to catch chained confirmations.
- **`config.rs`** — TOML config deserialization structs: `Config`, `TmuxConfig`, `LlmConfig`, `GuardRails`, `GuardRule`, `RapidResponse`.
- **`llm.rs`** — `LlmClient` sends captured terminal output (last 20 lines, reversed) to an OpenAI-compatible chat completions API. The system prompt contains a hardcoded list of safe/destructive rule names. Returns either `"NONE"`, a rule name, or `"rule_name:position"` for numbered menus.
- **`guard.rs`** — `GuardRailsEngine` looks up the LLM-returned rule name against configured rules and returns the configured response string. Falls back to `default_response` (typically "no") for unknown rules.
- **`tmux.rs`** — `TmuxClient` wraps `tmux capture-pane` and `tmux send-keys`. Detects session termination via `SessionNotFoundError` to allow graceful exit.
- **`cli.rs`** — CLI argument parsing with clap (config path, dry-run, verbose flags).

## Key Design Decisions

- **LLM response format**: The LLM returns either a plain rule name (for yes/no prompts) or `rule_name:position` (for numbered menus). When no position is given, `main.rs` converts yes→"1", no→"2" as a backward-compatible default.
- **Responses are sent without Enter** (`send-keys` without trailing `Enter`) since tmux menu selections don't need it.
- **The system prompt in `llm.rs` contains a hardcoded comprehensive list of guard rules** (destructive vs safe). When adding new rule categories, both the system prompt in `llm.rs` and the config file need updating.
- **Rapid response mode**: After sending a response, the app does a configurable burst of fast re-checks (default 5 checks at 200ms intervals) to handle rapid follow-up prompts.

## Configuration

TOML-based config. See `config.example.toml` for the full template. Key sections: `[tmux]`, `[llm]`, `[guard_rails]` with `[[guard_rails.rules]]`, `[rapid_response]`, and top-level `monitoring_interval_ms`.
