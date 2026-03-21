# taskspace design intent

## What this tool is

taskspace is a session-oriented workspace manager for AI coding.
It creates one isolated workspace for one task with a minimal, explicit session schema.

## What v1 must guarantee

- One task maps to one session directory.
- AI entrypoint, metadata, and working area are always present.
- One session structure can be recreated consistently.
- `taskspace new --open` starts work immediately.
- `taskspace new --template <local-yaml>` can clone manifest projects and persist reproducibility metadata.
- The behavior is agent-agnostic (OpenCode, Claude Code, Gemini CLI, etc.).

## What v1 intentionally excludes

- dotfiles integration
- secrets management
- remote execution
- DevContainer integration
- plugin extension framework

## Why the structure is this way

- `~/taskspace/<session>` is the active work area (session directories are directly under the taskspace root).
- `~/taskspace/.archive/` stores archived sessions to keep deletion reversible by default.
- `AGENTS.md` is the AI entrypoint.
- `repos/` is a workspace area for project checkouts created from template manifest entries.
- `workspace.yaml` stores reproducibility metadata (template reference, manifest).
- Additional helper files are created on demand by user or AI.

## Safety and operational rules

- `rm` is destructive; interactive terminals ask for confirmation without `--yes`, while non-interactive environments require `--yes`.
- `rm --dry-run` lets users verify targets without deletion.
- `open` without a name opens the latest session by directory modified time.
- External commands are executed with argument lists (not shell strings).
- Session names are validated before file operations.
- `doctor` checks structure, metadata, and command availability.

## Stable contracts

- CLI commands are stable: `new`, `open`, `list`, `rm`, `archive`, `doctor`.
- Command aliases are supported: `remove` for `rm`, `ls` for `list`.
- Version flags are supported: `-v`, `-V`, `--version`.
- `workspace.yaml` includes session metadata with a schema `version`.
- Exit codes are mapped by error category.

## Implementation boundaries

- `taskspace-core`: domain types and error categories.
- `taskspace-app`: command use cases.
- `taskspace-infra-fs`: filesystem and process execution.
- `taskspace-cli`: argument parsing and terminal output.

## Change policy

This v1 rewrite intentionally resets session schema compatibility.
Future schema changes should increment version and provide explicit migration guidance.
