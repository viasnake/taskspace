# taskspace

`taskspace` creates reusable git checkout directories for AI agent work.

## Concept

- Git checkout is the primary workflow.
- A slot is a reusable working directory for Codex, OpenCode, or another local agent.
- Slots are created for the desired parallelism, not for individual tasks.
- When one task is done, reuse the same slot for the next task by checking out another branch or commit.
- taskspace only prepares and tracks local working directories; humans open agents inside those directories.

## Layout

By default, taskspace stores local state under `~/taskspace`.

```text
~/taskspace/
  workspaces/
    agent-1/
    agent-2/
  state/
    slots/
      agent-1/slot.yaml
      agent-2/slot.yaml
```

Each workspace also contains `.taskspace/context.yaml`. Codex hooks and other local automation can read this file from the session `cwd`.

## Install

### mise

```bash
mise use -g github:viasnake/taskspace@v0.6.0
```

### Homebrew

```bash
brew install viasnake/tap/taskspace
```

## Quick Start

Create two reusable checkout slots from a source repository:

```bash
taskspace init ~/src/app --slots 2
```

Use a slot for the next task:

```bash
taskspace checkout agent-1 feature/auth-env
taskspace enter agent-1 --agent codex
```

When that task is complete, reuse the same slot:

```bash
taskspace checkout agent-1 main
taskspace checkout agent-1 feature/next-change
taskspace enter agent-1 --agent opencode
```

## Commands

```bash
taskspace init <source> [--slots <n>]
taskspace list
taskspace show <slot>
taskspace checkout <slot> <git-ref>
taskspace enter <slot> [--agent <codex|opencode>]
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

- current slot id
- source repository
- workspace path
- last requested checkout
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
