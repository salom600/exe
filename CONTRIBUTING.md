# Contributing to FlowCut

Thank you for your interest in contributing to FlowCut! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, constructive, and professional in all interactions.

## How to Contribute

### Reporting Bugs
1. Check if the bug has already been reported in [Issues](https://github.com/salom600/exe/issues)
2. Create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - OS, version, and system specs
   - Screenshots or logs if applicable

### Suggesting Features
1. Open a feature request issue
2. Describe the feature and its use case
3. Explain why it would benefit FlowCut users

### Submitting Code Changes

#### Setup
```bash
git clone https://github.com/salom600/exe.git
cd exe
npm install
npm run dev
```

#### Development Workflow
1. Create a feature branch: `git checkout -b feature/your-feature-name`
2. Make your changes
3. Test locally: `npm run dev`
4. Run Rust checks: `cargo fmt && cargo clippy && cargo test`
5. Commit with clear messages
6. Push and create a Pull Request

#### Code Standards

**Rust:**
- Follow `rustfmt` formatting: `cargo fmt`
- Pass Clippy lint: `cargo clippy -- -D warnings`
- Add doc comments for all public items
- Use `thiserror` for error types
- Use `serde` for serialization

**JavaScript:**
- Use ES6+ features
- Add JSDoc comments for classes and methods
- Use the EventBus for cross-module communication
- Handle errors gracefully with user-friendly messages

**CSS:**
- Follow the Catppuccin Mocha color variables
- Use CSS custom properties (variables) for all colors
- Keep styles modular in separate files

#### Pull Request Guidelines
- One feature or fix per PR
- Include tests if applicable
- Update documentation for new features
- Reference related issues
- Keep PRs focused and manageable in size

### Build Verification
All PRs are verified by GitHub Actions:
- Rust format + lint + test
- Frontend file validation
- Cross-platform build (Linux, Windows, macOS)

## Project Structure

See [README.md](README.md) for the full architecture overview.

## Contact

Open an issue for any questions about contributing.
