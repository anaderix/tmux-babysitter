# Tmux Babysitter Safeguard Configuration Guide

This document explains the safeguard configuration (`config.safeguard.toml`) designed to protect your system from destructive operations that might be initiated by AI assistants like Claude or other automated processes.

## Philosophy

The safeguard configuration follows a **defense-in-depth** approach:

1. **Default to "NO"**: If uncertain about an operation, refuse it
2. **Explicit allow-list**: Only safe, non-destructive operations get "yes"
3. **Comprehensive blocking**: Cover all common destructive operations
4. **Fast monitoring**: Checks every 500ms for rapid response

## Categories of Protected Operations

### 1. File System Destruction

Protects against:
- Deleting files and directories (`rm`, `delete`, `erase`)
- Recursive deletes (`rm -rf`, `del /s`, `rm -r`)
- Overwriting existing files
- Formatting or wiping disks
- Deleting disk partitions

### 2. Data Loss

Protects against:
- Dropping or truncating databases
- Wiping database tables
- Deleting backups and archives
- Deleting log files (which may contain audit trails)
- Clearing caches that might contain uncommitted work

### 3. Version Control Destruction

Protects against:
- Force-deleting git branches
- `git reset --hard` (discards uncommitted changes)
- Force pushing (overwrites remote history)
- Amending already-pushed commits
- `git clean -fdx` (removes untracked files)

### 4. Package & Dependency Destruction

Protects against:
- Uninstalling or removing packages
- Deleting dependency directories (`node_modules`, `vendor`)
- Full system upgrades (can break environments)

### 5. Credential & Security Loss

Protects against:
- Deleting SSH keys
- Removing API keys or tokens
- Deleting SSL/TLS certificates
- Resetting keychains or password stores

### 6. Process & System Control

Protects against:
- Force-killing processes (`kill -9`, SIGKILL)
- Shutting down, rebooting, or halting the system
- Stopping critical system services

### 7. Configuration Destruction

Protects against:
- Deleting configuration files
- Overwriting existing configurations
- Resetting application settings

### 8. Data Exfiltration & Remote Code

Protects against:
- Uploading sensitive data to external servers
- Sending files to external recipients
- Downloading and executing code from the internet

### 9. Container & Infrastructure Destruction

Protects against:
- Deleting Docker containers with data
- Deleting Docker volumes
- Deleting virtual machines
- Resetting or wiping development sandboxes
- Deleting cloud resources (EC2, S3, etc.)
- Terminating cloud instances
- Wiping cloud storage

### 10. Development Environment Protection

Protects against:
- Clearing command history
- Truncating or zeroing files
- General destructive development operations

## Allowed Safe Operations

The following operations are **automatically approved**:

### Development Workflow
- Continuing with non-destructive processes
- Building and compiling code
- Running tests
- Installing packages and dependencies
- Creating git commits
- Pushing/pulling (non-force) from git
- Checking out and merging branches

### Container Operations
- Pulling Docker images
- Running Docker containers
- Building Docker images

### Database Operations
- Running migrations (non-destructive)

### Deployment
- Deploying to non-production environments

## Using the Safeguard Configuration

### 1. Copy and Customize

```bash
cp config.safeguard.toml config.toml
```

Edit the `[tmux]` section to match your session:
```toml
[tmux]
session = "your-session-name"
window = "your-window-name"  # Optional
pane = "0"                    # Optional
```

### 2. Configure LLM

The configuration uses Ollama by default:
```toml
[llm]
base_url = "http://localhost:11434/v1"
model = "llama3.2"
api_key = "ollama"
```

To use a different LLM (e.g., OpenAI):
```toml
[llm]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
api_key = "your-api-key-here"
```

### 3. Test in Dry Run Mode

Always test first:
```bash
./target/release/tmux-babysitter -c config.toml --dry-run -v
```

Watch the logs to see how questions are being classified.

### 4. Run the Babysitter

Once satisfied with the dry-run:
```bash
./target/release/tmux-babysitter -c config.toml
```

## Customizing Guardrails

### Adding New Rules

Add to the appropriate section in `config.toml`:

```toml
[[guard_rails.rules]]
name = "my_custom_rule"
description = "Description of what this rule detects"
response = "no"  # or "yes"
```

### Modifying Existing Rules

Change the `response` field to allow an operation (use caution!):

```toml
[[guard_rails.rules]]
name = "package_remove"
description = "Asks to remove, uninstall, or purge packages"
response = "yes"  # DANGER: Now allows package removal!
```

### Removing Rules

Simply remove the entire `[[guard_rails.rules]]` block.

## Security Best Practices

### 1. Always Test with Dry-Run

Never run the babysitter in production without first testing with `--dry-run`.

### 2. Monitor Logs

Keep an eye on the logs, especially in the beginning:
```bash
./target/release/tmux-babysitter -c config.toml -v | tee babysitter.log
```

### 3. Review LLM Decisions

The LLM might misclassify questions. If you see incorrect classifications:

- **Too restrictive**: Consider adding a more specific rule
- **Too permissive**: Review the LLM prompt in `src/llm.rs` or add specific blocking rules

### 4. Keep Default as "No"

Never change `default_response` to "yes". If the LLM is uncertain, it should refuse.

### 5. Regular Audits

Periodically review:
- The guardrail rules
- Recent logs
- Any modifications to the config

### 6. Backup Critical Data

While the babysitter helps, it's not perfect. Maintain regular backups of important data.

### 7. Understand Your LLM

Different LLMs may have different accuracies:
- **Llama 3.2 3B**: Fast, inexpensive, good for simple pattern matching
- **GPT-4o-mini**: More accurate, more expensive
- **Ollama models**: Run locally, no data leaves your system

## Common Scenarios

### Scenario: Claude tries to delete files

**Terminal output:**
```
rm -rf /important/data/
Are you sure you want to delete this directory? [y/N]
```

**Babysitter response:**
- LLM identifies: `file_delete` or `recursive_delete`
- Guard rule response: `no`
- Result: Command is aborted, data protected

### Scenario: Installing a package

**Terminal output:**
```
npm install package-name
Proceed with installation? [y/N]
```

**Babysitter response:**
- LLM identifies: `package_install`
- Guard rule response: `yes`
- Result: Installation proceeds

### Scenario: Git force push attempt

**Terminal output:**
```
git push --force
This will overwrite remote history. Continue? [y/N]
```

**Babysitter response:**
- LLM identifies: `git_force_push`
- Guard rule response: `no`
- Result: Force push blocked, remote history protected

## Limitations

1. **LLM Accuracy**: The system depends on the LLM correctly identifying questions
2. **Non-Prompted Operations**: Actions that don't ask for confirmation won't be blocked
3. **New Operations**: Novel destructive operations not in the rules list may be misclassified
4. **Speed**: 500ms interval means some fast-acting commands might slip through
5. **False Positives**: Some safe operations might be incorrectly blocked

## Troubleshooting

### "Operations are being blocked incorrectly"

1. Enable verbose logging: `-v`
2. Check what rule is being matched
3. If needed, add a more specific rule or modify the description

### "LLM isn't detecting questions"

1. Verify the LLM is accessible
2. Check the LLM model name
3. Review the output to see if it matches expected patterns
4. Consider using a more capable model temporarily for debugging

### "Too many false negatives (dangerous operations not blocked)"

1. Review the rules list for missing operation types
2. Add specific rules for new destructive operations
3. Check if the LLM is correctly identifying the operation type

## Contributing

To add new guard rules:

1. Identify the destructive operation type
2. Create a descriptive name and description
3. Set response to "no" for destructive operations
4. Add the corresponding entry to the LLM prompt in `src/llm.rs`
5. Test with `--dry-run`

## Support

If you encounter issues or have suggestions:
1. Check the logs with `-v`
2. Review this guide
3. Test operations in a safe environment first
4. Consider the specific LLM you're using and its capabilities

---

**Remember**: The safeguard configuration is a safety net, not a replacement for proper security practices. Always maintain backups and follow security best practices.
