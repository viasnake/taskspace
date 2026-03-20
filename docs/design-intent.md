# taskspace design intent

## What this tool is

taskspace is a session-oriented workspace manager for AI coding.
It creates one isolated workspace for one task, with repository files and AI context files in one place.

## What v1 must guarantee

- One task maps to one session directory.
- AI context files are separated from repository content.
- Multi-repository work is possible in one session.
- `taskspace new --open` starts work immediately.
- The behavior is agent-agnostic (OpenCode, Claude Code, Gemini CLI, etc.).

## What v1 intentionally excludes

- dotfiles integration
- secrets management
- remote execution
- DevContainer integration
- plugin extension framework

## Why the structure is this way

- `~/taskspace/sessions/<session>` is the active work area.
- `~/taskspace/archive/` stores archived sessions to keep deletion reversible by default.
- `context/` stores ephemeral AI notes (`MEMORY`, `PLAN`, `CONSTRAINTS`, `DECISIONS`, `LINKS`).
- `repos/` stores cloned repositories for this session only.

## Safety and operational rules

- `rm` is destructive and requires `--yes`.
- `rm --dry-run` lets users verify targets without deletion.
- External commands are executed with argument lists (not shell strings).
- Session and repo names are validated before file operations.
- `doctor` checks structure, metadata, and command availability.

## Stable contracts

- CLI commands are stable: `new`, `open`, `list`, `rm`, `archive`, `doctor`.
- `workspace.yaml` includes a schema `version`.
- Exit codes are mapped by error category.

## Implementation boundaries

- `taskspace-core`: domain types and error categories.
- `taskspace-app`: command use cases.
- `taskspace-infra-fs`: filesystem and process execution.
- `taskspace-cli`: argument parsing and terminal output.

## Change policy

When behavior changes, keep backward compatibility for existing sessions.
If compatibility cannot be preserved, increment schema version and add explicit migration logic.
