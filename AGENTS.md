# AGENTS.md - taskspace Development Guide

This repository implements taskspace v0.5.0.

## Project Overview

taskspace is a Rust workspace for task-oriented multi-root workspace management.

- Workspace crates: `taskspace-core`, `taskspace-app`, `taskspace-infra-fs`, `taskspace-cli`
- Edition: 2024
- Minimum line coverage: 70%

## Build, Lint, Test

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Optional tools (if installed):

```bash
cargo audit
cargo deny check
cargo llvm-cov --workspace --all-features --fail-under-lines 70 -- --test-threads=1
```

## Architecture Boundaries

- `taskspace-core`: task model, root model, lifecycle state machine, shared errors
- `taskspace-app`: task lifecycle use cases and adapter orchestration
- `taskspace-infra-fs`: filesystem/process primitives
- `taskspace-cli`: CLI parsing/execution/rendering

Dependency direction: `cli -> app -> core + infra-fs`

## Coding Rules

1. Keep changes small and focused.
2. Prefer explicit error messages with context.
3. Keep parsing and validation deterministic.
4. Avoid shell string interpolation when direct process APIs are possible.
5. Add tests for every bug fix and lifecycle rule change.

## Commit Messages

Use Conventional Commits:

- `feat: ...`
- `fix: ...`
- `refactor: ...`
- `test: ...`
- `docs: ...`
