# taskspace

**taskspace** is a session-oriented workspace manager for AI coding.

vNext keeps the default session schema minimal and agent-agnostic.

## Features

- Session-based workflow (`1 task = 1 session`)
- Minimal default session scaffold
- Reproducibility metadata in `workspace.yaml`
- Agent-agnostic session workflow
- User/AI-extensible workspace (create helper files only when needed)

## Install

### Homebrew (recommended)

```bash
brew install viasnake/tap/taskspace
```

### mise (optional)

```bash
mise use -g github:viasnake/taskspace@latest
```

## Quick Start

```bash
taskspace doctor
taskspace new demo --open
```

This will:

- Create a minimal session workspace
- Open it using commands defined in `workspace.yaml`
- Let you start work immediately

## Concepts

### Session

One task maps to one session workspace.

### `AGENTS.md`

The single session entrypoint for AI instructions.

### `workspace.yaml`

Machine-readable session facts and reproducibility metadata.

### `repos/`

Primary working area for cloned or manually added repositories.

### Additional files

Any other helper file or directory is created on demand by the user or AI.

## Directory Structure

Default session schema:

```text
~/taskspace/
  .archive/
  <session>/
    AGENTS.md
    workspace.yaml
    repos/
```

Templates may add more files and directories, but the default schema stays minimal.

## Commands

### Create a session

```bash
taskspace new <name> --open
taskspace new <name> --template ./session-template.yaml --open
```

Template examples:

```bash
taskspace new demo --template ./examples/template-minimal.yaml
taskspace new demo --template ./examples/template-monorepo.yaml
```

When a template includes `manifest.projects`, `taskspace new --template ...` clones those repositories into the session and records resolved commits in `workspace.yaml`.

### Open a session

```bash
taskspace open <name>
taskspace open
taskspace open --last
```

Notes:

- `open` reads `open.actions` from each session's `workspace.yaml`.
- `open.actions` must exist and must not be empty.
- All configured actions are executed in order.
- If any action fails, `open` fails and reports aggregated errors.
- `open` runs only in interactive local environments.
- In non-interactive/SSH/CI environments, `taskspace open` fails fast, and `taskspace new --open` creates the session and skips opening.

`workspace.yaml` example:

```yaml
version: 5
name: demo
created_at: "2026-03-23T00:00:00Z"
layout_version: 1
created_by: manual
open:
  actions:
    - command: ["opencode", "{dir}"]
    - command: ["nvim", "{dir}"]
```

### List sessions

```bash
taskspace list
taskspace ls
```

### Remove a session

```bash
taskspace rm <name>
taskspace remove <name>
taskspace rm <name> --dry-run
taskspace rm <name> --yes
```

### Archive a session

```bash
taskspace archive <name>
```

### Diagnose environment and sessions

```bash
taskspace doctor
```

### Show version

```bash
taskspace -v
taskspace -V
taskspace --version
```

### Shell completion

```bash
taskspace completion bash
taskspace completion zsh
taskspace completion fish
```

```bash
taskspace completion bash > ~/.local/share/bash-completion/completions/taskspace
taskspace completion zsh > ~/.local/share/zsh/site-functions/_taskspace
taskspace completion fish > ~/.config/fish/completions/taskspace.fish
```

## AI Integration

taskspace works with OpenCode, Claude Code, Gemini CLI, and other file-aware coding agents.

Recommended order:

1. Read `AGENTS.md`
2. Optionally apply reusable global skills (such as `SKILL.md`)
3. Create session-local helper files only when needed

## Safety

- `rm` is destructive; use `--dry-run` when unsure
- Prefer `archive` over deletion when intent is unclear
- Keep default schema minimal and explicit

## Migration Notes

- `workspace.yaml` schema is now `version: 5`.
- `--editor` has been removed from `taskspace new` and `taskspace open`.
- `taskspace open` now executes `workspace.yaml` `open.actions` in order.
- Legacy `workspace.yaml` files without `open.actions` must be updated before `open`.

Minimal v5 `open` block:

```yaml
open:
  actions:
    - command: ["opencode", "{dir}"]
```

## Development

```bash
git clone https://github.com/viasnake/taskspace
cd taskspace

mise install
mise run build
mise run check
```

Release process:

- See [Release Guide](docs/release.md) for the minimum release procedure.

## Philosophy

taskspace is a minimal session manager.

It guarantees only what is required to start and reproduce work:

- an AI entrypoint (`AGENTS.md`)
- machine-readable metadata (`workspace.yaml`)
- an isolated work area (`repos/`)

Everything else is optional and added on demand.
