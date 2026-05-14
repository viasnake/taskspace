# taskspace design intent

## What this tool is

taskspace is a small local checkout manager for parallel AI-agent work.
It creates a fixed number of reusable git working directories, then helps a human open Codex, OpenCode, or another agent in one of those directories.

The primary workflow is `git checkout`.
taskspace does not model tasks as persistent registry entries.
A task is temporary human intent; a slot is durable local infrastructure.

## What taskspace must guarantee

- A slot maps to one reusable working directory.
- Slots are created for parallelism count, for example `agent-1` and `agent-2`.
- Slot state is local and inspectable under `state/slots/<slot>/slot.yaml`.
- Every slot workspace receives `.taskspace/context.yaml`.
- `checkout` delegates branch and commit movement to git.
- `enter` starts the selected agent with the slot as the working directory when the agent supports it.

## What taskspace intentionally excludes

- task lifecycle state machines
- task archives
- multi-root task views
- policy enforcement for Codex or OpenCode
- automatic hook installation into user or project config
- workflow orchestration across agents

## Codex hooks integration

Codex discovers hooks from active config layers and runs command hooks with the session `cwd`.
taskspace uses that contract by writing `.taskspace/context.yaml` in each slot.

Hooks can call:

```bash
taskspace hook-context
```

from inside a slot workspace to print the nearest taskspace context.
This avoids writing `.codex/` files into cloned repositories and keeps user-level Codex hook policy independent from taskspace.

Known Codex hook constraints that shape this design:

- Repo-local hooks require the project `.codex/` layer to be trusted.
- User-level hooks can still run in untrusted projects.
- Hook command matchers and outputs are Codex-owned contracts, not taskspace-owned contracts.
- Pre-tool hooks are guardrails, not a complete enforcement boundary.

## Stable contracts

- Public CLI commands: `init`, `list`, `show`, `checkout`, `enter`, `hook-context`.
- Slot ids are explicit stable names such as `agent-1`.
- Context file path inside a slot: `.taskspace/context.yaml`.
- Default workspace root: `~/taskspace`.

## Implementation boundaries

- `taskspace-core`: slot model, context model, shared errors.
- `taskspace-app`: slot registry, git clone/checkout orchestration, context writing, agent entry.
- `taskspace-infra-fs`: filesystem and process execution helpers.
- `taskspace-cli`: CLI argument parsing, direct app dispatch, and output rendering.
