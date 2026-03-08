# Tmux Babysitter

A Rust application that monitors a tmux session containing a cloud code process and automatically answers yes/no questions using a cheap LLM, keeping the environment safe and progress going.

## Features

- **Intelligent Question Detection**: Uses an LLM (like Ollama) to detect questions in terminal output
- **Guard Rails**: Configurable rules define how to respond to different types of questions
- **Tmux Integration**: Captures pane output and sends keyboard input
- **Safety First**: Defaults to safe responses ("no") for destructive operations
- **Dry Run Mode**: Test the babysitter without actually sending responses
- **Configurable**: TOML-based configuration for easy customization

## Requirements

- Rust 1.70 or later
- tmux (installed and running)
- Access to an LLM API (Ollama, OpenAI, or OpenAI-compatible API)

## Installation

### Using Make (Recommended)

```bash
# Clone or navigate to the repository
cd tmux-babysitter

# Build release version
make release

# View all available make targets
make help
```

### Using Cargo Directly

```bash
# Clone or navigate to the repository
cd tmux-babysitter

# Build the project
cargo build --release

# The binary will be available at target/release/tmux-babysitter
```

### System-wide Installation

```bash
# Install to /usr/local/bin (requires sudo)
make install

# Or install to ~/.local/bin (no sudo needed)
make install-user
```

## Configuration

Create a configuration file (e.g., `config.toml`) based on the example:

```bash
cp config.example.toml config.toml
```

### Configuration Options

```toml
[tmux]
session = "my-session"          # Name of your tmux session
window = "my-window"            # Optional: window name
pane = "0"                       # Optional: pane index

[llm]
base_url = "http://localhost:11434/v1"  # API endpoint
model = "llama3.2"                       # Model name
api_key = "ollama"                       # API key (if required)

[guard_rails]
default_response = "no"  # Default response when no rule matches

[[guard_rails.rules]]
name = "continue_confirmation"
description = "Asks if user wants to continue with a process"
response = "yes"

[[guard_rails.rules]]
name = "file_overwrite"
description = "Asks permission to overwrite files"
response = "no"

[[guard_rails.rules]]
name = "package_installation"
description = "Asks to install dependencies"
response = "yes"

[[guard_rails.rules]]
name = "destructive_operation"
description = "Asks confirmation for destructive operations (delete, remove, etc.)"
response = "no"

monitoring_interval_ms = 1000  # Check every second
```

## Setting up Ollama

To use Ollama locally (recommended for cheap LLM):

```bash
# Install Ollama (macOS)
brew install ollama

# Start Ollama
ollama serve

# Pull a model (e.g., Llama 3.2 3B for speed and low cost)
ollama pull llama3.2:3b

# Your config should look like:
# [llm]
# base_url = "http://localhost:11434/v1"
# model = "llama3.2:3b"
# api_key = "ollama"
```

## Usage

### Using Make (Recommended)

```bash
# Run debug version with arguments
make run ARGS='-c config.toml --dry-run'

# Run release version
make run-release ARGS='-c config.toml -v'

# Test with example config in dry-run mode
make test-babysitter

# Test with safeguard config in dry-run mode
make test-safeguard

# Format code
make fmt

# Run linter
make clippy

# Format and check code
make check

# Clean build artifacts
make clean

# Run tests
make test
```

### Using Cargo Directly

```bash
# Run the babysitter with your config
cargo run -- -c config.toml

# Or use the built binary
./target/release/tmux-babysitter -c config.toml
```

### Dry Run Mode

Test the babysitter without sending any responses:

```bash
cargo run -- -c config.toml --dry-run
```

### Verbose Logging

Enable debug-level logging for more details:

```bash
cargo run -- -c config.toml -v
```

### Command-Line Options

- `-c, --config <PATH>`: Path to configuration file (required)
- `--dry-run`: Only log what would be done, don't send responses
- `-v, --verbose`: Enable verbose logging
- `-h, --help`: Print help information

## How It Works

1. **Capture Output**: The app captures the current tmux pane output at regular intervals
2. **LLM Analysis**: Sends the recent output to the configured LLM to detect if there's a question
3. **Guard Rail Matching**: If a question is detected, matches it to the appropriate guard rule
4. **Automated Response**: Sends the configured response ("yes", "no", or custom) to the tmux session
5. **Logging**: All decisions and responses are logged for transparency

## Safety Considerations

- **Default to "No"**: Always use `default_response = "no"` for safety
- **Test with Dry Run**: Always test your configuration with `--dry-run` first
- **Review Logs**: Monitor the logs to ensure the babysitter is making correct decisions
- **Specific Rules**: Create specific rules for different types of questions to avoid false positives

## Example Workflow

1. Start your cloud code process in a tmux session:
   ```bash
   tmux new -s my-session
   # Run your process that asks questions
   ```

2. Configure `config.toml` with your session name and guard rails

3. Test the configuration:
   ```bash
   cargo run -- -c config.toml --dry-run -v
   ```

4. If the dry-run looks good, run the actual babysitter:
   ```bash
   cargo run -- -c config.toml
   ```

5. The babysitter will now monitor and respond to questions automatically

## Adding Custom Guard Rules

Add new rules to your `config.toml`:

```toml
[[guard_rails.rules]]
name = "database_migration"
description = "Asks confirmation for database migrations"
response = "yes"  # or "no" depending on your preference
```

The LLM will be instructed to look for these rule names and match them to the appropriate response.

## Troubleshooting

### "tmux capture-pane failed"
- Ensure tmux is running
- Verify the session/window/pane names in your config are correct
- Check that you have proper permissions to access the tmux session

### "LLM request failed"
- Verify your LLM API is running (e.g., `ollama serve`)
- Check the `base_url` in your config
- Ensure the model name is correct
- Verify API key if required

### Babysitter not responding
- Enable verbose logging with `-v`
- Check if the LLM is correctly identifying questions
- Adjust the monitoring interval in config
- Review the LLM's prompt in `src/llm.rs` if needed

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
