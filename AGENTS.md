# AGENTS.md - taskspace Development Guide

This file provides development guidelines for agents operating in the taskspace repository.

## Project Overview

taskspace is a Rust workspace project for session-oriented workspace management for AI coding.
- **Workspace crates**: taskspace-core, taskspace-app, taskspace-infra-fs, taskspace-cli
- **Edition**: 2024
- **Minimum line coverage**: 70%

---

## Build, Lint, and Test Commands

### Standard Workflow Commands

```bash
# Build the entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Format check (Rustfmt)
cargo fmt --all -- --check

# Lint with Clippy (warnings are errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Security audit
cargo audit

# License and dependency policy check
cargo deny check

# Line coverage gate (>=70%)
cargo llvm-cov --workspace --all-features --fail-under-lines 70 -- --test-threads=1
```

### Single Test Commands

```bash
# Run a single test by name (exact match)
cargo test test_session_name_accepts_simple_name

# Run tests matching a pattern
cargo test session_name

# Run tests in a specific crate
cargo test -p taskspace-core
cargo test -p taskspace-app
cargo test -p taskspace-cli

# Run tests in a specific crate matching a pattern
cargo test -p taskspace-cli binary_new_and_list_work

# Run only library tests (not integration tests)
cargo test --lib

# Run only integration tests
cargo test --test '*'
```

### Using mise (recommended)

```bash
mise run build     # Build
mise run test      # Run tests
mise run fmt       # Check formatting
mise run lint      # Run clippy
mise run audit     # Security audit
mise run deny      # Dependency policy
mise run coverage  # Line coverage
mise run check     # All quality checks
```

---

## Code Style Guidelines

### General Rules

1. **No warnings allowed**: Clippy runs with `-D warnings`; code must compile cleanly.
2. **Formatting**: Use `cargo fmt` before committing.
3. **Line coverage**: New code must maintain >=70% line coverage.

### Imports

- Group imports in order: `std` → external crates → local crates
- Use `use` statements with braces for grouped imports
- Prefer bringing specific items into scope rather than `use *`

```rust
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use taskspace_core::TaskspaceError;
```

### Error Handling

**Error enum pattern** (for domain errors):
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskspaceError {
    #[error("error: {0}")]
    Usage(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("internal error: {0}")]
    Internal(String),
}
```

**Result type usage**:
- Use `anyhow::Result<T>` for application-level operations where error context is needed
- Use domain-specific `Result<T, ErrorType>` for library/parsing code
- Use `?` operator for error propagation
- Map infrastructure errors: `map_err(map_infra_error)`

### Struct and Enum Definitions

```rust
// Structs: derive Debug, Clone, (PartialEq, Eq if comparable)
#[derive(Debug, Clone)]
pub struct SessionName(String);

// Enums: derive Debug, Clone, Copy, PartialEq, Eq where appropriate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    #[default]
    Opencode,
    Code,
}

// Request/Response structs for public API
#[derive(Debug, Clone)]
pub struct NewSessionRequest {
    pub name: SessionName,
    pub template_path: Option<PathBuf>,
    pub open_after_create: bool,
    pub editor: EditorKind,
}
```

### Visibility

- Use `pub(crate)` for items intended for internal crate use
- Use `pub` only for truly public API
- Private by default is preferred

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Structs | PascalCase | `SessionName`, `TaskspaceApp` |
| Enums | PascalCase | `EditorKind`, `TaskspaceError` |
| Enum variants | PascalCase | `EditorKind::Opencode` |
| Functions | snake_case | `create_session()`, `map_infra_error()` |
| Struct fields | snake_case | `root_dir`, `session_name` |
| Constants | SCREAMING_SNAKE_CASE | `WORKSPACE_SCHEMA_VERSION` |
| Type aliases | PascalCase | (for newtypes) |

### Newtype Pattern

Use newtypes for validated data:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionName(String);

impl SessionName {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        // validation logic
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

### Module Organization

- One module per file, filename matches module name
- Module declaration at top of file: `mod module_name;`
- Use `#[cfg(test)]` for test-only helper functions

```rust
mod doctor;
mod paths;
mod repo_import;
mod spec;
mod template;
mod validation;
```

### Test Patterns

**Unit tests** (in `#[cfg(test)]` mod within source file):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_name_accepts_simple_name() {
        let name = SessionName::parse("feature-123").expect("valid name");
        assert_eq!(name.as_str(), "feature-123");
    }

    #[test]
    fn session_name_rejects_invalid_chars() {
        let err = SessionName::parse("feature/123").expect_err("invalid name");
        assert!(matches!(err, TaskspaceError::Usage(_)));
    }
}
```

**Integration tests** (in `tests/` directory):
- Use `assert_cmd::Command::cargo_bin()` for CLI testing
- Use `tempfile::tempdir()` for temporary directories
- Use `predicates` for output assertions

### Commit Messages

Follow [Conventional Commits v1.0.0](https://www.conventionalcommits.org/):
```
feat: add session archiving
fix: handle missing SESSION.md in doctor
refactor: extract validation logic
test: add integration tests for rm command
```

---

## Workspace Structure

```
crates/
├── taskspace-core/     # Domain types, errors, validation
├── taskspace-app/      # Application logic, session management
├── taskspace-infra-fs/ # File system and process operations
└── taskspace-cli/      # CLI entry point, argument parsing
```

**Dependency direction**: CLI → App → Core and Infra-FS

- `taskspace-cli` depends on: taskspace-app, taskspace-core
- `taskspace-app` depends on: taskspace-core, taskspace-infra-fs
- `taskspace-core`: no internal dependencies
- `taskspace-infra-fs`: no internal dependencies

---

## Dependency Policy

Managed via `deny.toml`:
- Allowed licenses: MIT, Apache-2.0, BSD-3-Clause, ISC, Unicode-3.0
- Vulnerable dependencies are blocked
- Multiple dependency versions trigger warnings

---

## Key Conventions

1. **Parse-then-validate pattern**: Parse input into types (returning domain errors), then operate on validated data.
2. **Fail-safe cleanup**: On failure during multi-step operations, attempt rollback.
3. **Helpful error messages**: Include context and hints in user-facing errors.
4. **Test isolation**: Each test creates its own temporary directory.
5. **Descriptive assertions**: Use `.expect("description")` for unwraps, include context in assertion messages.
