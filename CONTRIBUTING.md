# Contributing to Throttler

Thank you for your interest in contributing to Throttler! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Coding Standards](#coding-standards)
- [Pull Request Process](#pull-request-process)
- [Issue Templates](#issue-templates)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Please be kind and constructive in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/throttler.git
   cd throttler
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/psenger/throttler.git
   ```

## How to Contribute

### Reporting Bugs

If you find a bug, please [open a bug report](https://github.com/psenger/throttler/issues/new?template=bug_report.md) with:

- A clear, descriptive title
- Steps to reproduce the issue
- Expected vs actual behavior
- Your environment details (OS, Rust version, Redis version)

### Suggesting Features

Have an idea for a new feature? [Open a feature request](https://github.com/psenger/throttler/issues/new?template=feature_request.md) with:

- A clear description of the feature
- The problem it solves or use case it addresses
- Any implementation ideas you have

### Submitting Code

1. Create a feature branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Make your changes
3. Write or update tests as needed
4. Ensure all tests pass
5. Submit a pull request

## Development Setup

### Prerequisites

- **Rust 1.70+** — [Install Rust](https://rustup.rs/)
- **Docker & Docker Compose** — For Redis

### Setup Steps

```bash
# Clone the repository
git clone https://github.com/psenger/throttler.git
cd throttler

# Copy environment configuration
cp .env.example .env

# Start Redis
docker compose up -d

# Build the project
cargo build

# Run tests
cargo test

# Run the service
cargo run
```

### Useful Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build the project |
| `cargo test` | Run all tests |
| `cargo test -- --nocapture` | Run tests with output |
| `cargo fmt` | Format code |
| `cargo clippy` | Run linter |
| `cargo check` | Check code without building |
| `RUST_LOG=debug cargo run` | Run with debug logging |

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Write documentation for public APIs

### Code Quality

- Write meaningful commit messages
- Keep commits focused and atomic
- Add tests for new functionality
- Update documentation as needed

### Testing

- All new features must include tests
- Bug fixes should include regression tests
- Run the full test suite before submitting:
  ```bash
  cargo test
  ```

## Pull Request Process

1. **Update your fork** with the latest upstream changes:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Ensure quality checks pass**:
   ```bash
   cargo fmt --check
   cargo clippy
   cargo test
   ```

3. **Create a pull request** with:
   - A clear title describing the change
   - A description of what changed and why
   - Reference to any related issues (e.g., "Fixes #123")

4. **Address review feedback** promptly

5. **Squash commits** if requested by maintainers

### PR Checklist

- [ ] Code follows the project's style guidelines
- [ ] Tests pass locally
- [ ] New functionality includes tests
- [ ] Documentation is updated if needed
- [ ] Commit messages are clear and descriptive

## Issue Templates

This project uses GitHub issue templates to help you provide the right information:

- **[Bug Report](https://github.com/psenger/throttler/issues/new?template=bug_report.md)** — Report a bug or unexpected behavior
- **[Feature Request](https://github.com/psenger/throttler/issues/new?template=feature_request.md)** — Suggest a new feature or enhancement

Using these templates helps maintainers understand and address your issue faster.

## Questions?

If you have questions about contributing, feel free to open a discussion or reach out through GitHub issues.

Thank you for contributing to Throttler!
