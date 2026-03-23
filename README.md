# taskspace

`taskspace` is a task-oriented multi-root workspace manager for AI-assisted work.

## Concept

- Task-first model (not repository-first)
- Multi-root workspace per task (`git`, `dir`, `file`, `artifact`, `scratch`)
- One-step entry into AI client (`enter`)
- Lightweight verification contract (`verify`)
- Local registry at `~/.local/state/taskspace/registry/tasks/<task-id>/task.yaml`

## Install

### mise (recommended)

```bash
mise use -g github:viasnake/taskspace@v0.5.0
```

### Homebrew (alternative)

```bash
brew install viasnake/tap/taskspace
```

## Quick Start

```bash
taskspace start "migrate auth env handling"
taskspace attach current ~/src/app --type git --role source --rw --isolation worktree
taskspace attach current ~/docs/runbooks --type dir --role docs --ro
taskspace enter current --adapter opencode
```

## Commands

```bash
taskspace start <title>
taskspace attach <task|current> <path> --type <git|dir|file|artifact|scratch> --role <role> [--ro|--rw] [--isolation <direct|worktree|copy|symlink|generated>]
taskspace detach <task|current> <root-id>
taskspace enter <task|current> [--adapter opencode]
taskspace list
taskspace show <task|current>
taskspace verify <task|current>
taskspace finish <task|current> [--state <active|blocked|review|done|archived>]
taskspace archive <task|current>
taskspace gc
taskspace completion [bash|zsh|fish]
```

## Shell Completion

```bash
taskspace completion bash > ~/.local/share/bash-completion/completions/taskspace
taskspace completion zsh > ~/.local/share/zsh/site-functions/_taskspace
taskspace completion fish > ~/.config/fish/completions/taskspace.fish
```

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
