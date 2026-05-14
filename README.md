# taskspace

`taskspace` manages dynamic Git workspace slots for parallel AI-agent work.

## Concept

- A project is a registered Git source.
- A slot is a clone for one project, created only when needed.
- `taskspace` handles project registration, slot lifecycle, fetch-only sync, and agent entry.
- Branch or commit checkout happens inside the slot with regular `git` commands.

## Layout

By default, taskspace stores local state under `~/taskspace`.

```text
~/taskspace/
  workspaces/
    app/
      agent-1/
      agent-2/
  state/
    projects/
      app/
        project.yaml
        slots/
          agent-1/slot.yaml
          agent-2/slot.yaml
```

Each workspace also contains `.taskspace/context.yaml`. Codex hooks and other local automation can read this file from the session `cwd`.

## Install

### mise

```bash
mise use -g github:viasnake/taskspace@v0.6.1
```

### Homebrew

```bash
brew install viasnake/tap/taskspace
```

## Quick Start

Initialize a taskspace root, register a project, and create two slots:

```bash
taskspace init
taskspace project add app ~/src/app
taskspace slot add app --count 2
```

Fetch updates before work begins:

```bash
taskspace sync --all
```

Open one slot for the next task:

```bash
taskspace enter app:agent-1 --agent codex
cd ~/taskspace/workspaces/app/agent-1
git checkout feature/auth-env
```

Add more parallelism later only when needed:

```bash
taskspace slot add app
taskspace slot list app
```

## Commands

```bash
taskspace init
taskspace project add <project> <source>
taskspace project list
taskspace project show <project>
taskspace slot add <project> [--count <n>]
taskspace slot list [project]
taskspace slot show <project:slot>
taskspace slot remove <project:slot> [--force]
taskspace sync <project>
taskspace sync --all
taskspace enter <project:slot> [--agent <codex|opencode>] [--no-sync]
taskspace hook-context [path]
taskspace completion [bash|zsh|fish]
```

## Codex Hooks

Codex command hooks run with the session `cwd` as their working directory. taskspace writes `.taskspace/context.yaml` into every slot, so a hook script can find the slot context by walking upward from `cwd`.

Example hook command:

```toml
[features]
codex_hooks = true

[[hooks.SessionStart]]
matcher = "startup|resume"

[[hooks.SessionStart.hooks]]
type = "command"
command = "taskspace hook-context"
statusMessage = "Loading taskspace context"
```

This keeps hook policy outside taskspace while giving hooks a stable local contract:

- current project id
- current slot reference
- source repository
- workspace path
- last successful sync timestamp
- updated timestamp

Project-local Codex hooks still depend on Codex trust rules for `<repo>/.codex/`. User-level hooks in `~/.codex/` can read taskspace context without modifying cloned repositories.

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
