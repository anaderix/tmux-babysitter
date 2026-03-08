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
