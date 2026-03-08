# Makefile for tmux-babysitter

.PHONY: all build release test clean run help install fmt clippy check

# Variables
CARGO = cargo
TARGET_DIR = target
BINARY_NAME = tmux-babysitter
RELEASE_BINARY = $(TARGET_DIR)/release/$(BINARY_NAME)
DEBUG_BINARY = $(TARGET_DIR)/debug/$(BINARY_NAME)

# Default target
all: build

# Build debug version
build:
	$(CARGO) build

# Build release version (optimized)
release:
	$(CARGO) build --release

# Run tests
test:
	$(CARGO) test

# Run tests with output
test-verbose:
	$(CARGO) test -- --nocapture

# Run tests in release mode
test-release:
	$(CARGO) test --release

# Clean build artifacts
clean:
	$(CARGO) clean
	rm -f $(RELEASE_BINARY) $(DEBUG_BINARY)

# Format code
fmt:
	$(CARGO) fmt

# Check code (format + clippy)
check: fmt clippy

# Run clippy for linting
clippy:
	$(CARGO) clippy -- -D warnings

# Run the debug version
run: build
	$(DEBUG_BINARY) $(ARGS)

# Run the release version
run-release: release
	$(RELEASE_BINARY) $(ARGS)

# Run with example config and dry-run
test-babysitter: release
	$(RELEASE_BINARY) -c config.example.toml --dry-run -v

# Run with safeguard config and dry-run
test-safeguard: release
	$(RELEASE_BINARY) -c config.safeguard.toml --dry-run -v

# Install to /usr/local/bin (requires sudo)
install: release
	install -m 755 $(RELEASE_BINARY) /usr/local/bin/$(BINARY_NAME)

# Install to ~/.local/bin (no sudo needed)
install-user: release
	install -d $(HOME)/.local/bin
	install -m 755 $(RELEASE_BINARY) $(HOME)/.local/bin/$(BINARY_NAME)

# Uninstall from /usr/local/bin (requires sudo)
uninstall:
	rm -f /usr/local/bin/$(BINARY_NAME)

# Uninstall from ~/.local/bin
uninstall-user:
	rm -f $(HOME)/.local/bin/$(BINARY_NAME)

# Show help
help:
	@echo "tmux-babysitter Makefile"
	@echo ""
	@echo "Usage:"
	@echo "  make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  all              Build debug version (default)"
	@echo "  build            Build debug version"
	@echo "  release          Build release version (optimized)"
	@echo "  test             Run tests"
	@echo "  test-verbose     Run tests with output"
	@echo "  test-release     Run tests in release mode"
	@echo "  clean            Clean build artifacts"
	@echo "  fmt              Format code"
	@echo "  clippy           Run clippy linter"
	@echo "  check            Format and check code (fmt + clippy)"
	@echo "  run              Run debug version (use ARGS=... for arguments)"
	@echo "  run-release      Run release version (use ARGS=... for arguments)"
	@echo "  test-babysitter  Test with example config in dry-run mode"
	@echo "  test-safeguard    Test with safeguard config in dry-run mode"
	@echo "  install          Install to /usr/local/bin (requires sudo)"
	@echo "  install-user     Install to ~/.local/bin"
	@echo "  uninstall        Uninstall from /usr/local/bin (requires sudo)"
	@echo "  uninstall-user   Uninstall from ~/.local/bin"
	@echo "  help             Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make run ARGS='-c config.toml --dry-run'"
	@echo "  make run-release ARGS='-c config.toml -v'"
	@echo "  make test-babysitter"
