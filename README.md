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
taskspace new demo --open --editor vscode --editor opencode
```

This will:

- Create a minimal session workspace
- Open it in the editors you choose
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
taskspace new <name> --open --editor vscode --editor opencode
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
taskspace open <name> --editor opencode --editor vscode
taskspace open
taskspace open --last
```

Notes:

- `--editor` can be repeated to compose multiple commands.
- When `--editor` is omitted, taskspace uses default editors in this order: `vscode`, `opencode`, `codex`, `claude`.
- With omitted `--editor`, unavailable default editors are skipped, and opening succeeds if at least one editor launches.
- If all default editors are unavailable, `open` fails with actionable hints.
- When `--editor` is explicitly provided, any launch failure is treated as an error.
- `open` runs only in interactive local environments.
- In non-interactive/SSH/CI environments, `taskspace open` fails fast, and `taskspace new --open` creates the session and skips opening.

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
