# taskspace design intent

## What this tool is

taskspace is a small local workspace manager for parallel AI-agent work.
It registers multiple Git projects, creates clone slots on demand, keeps those slots fetch-synchronized, and helps a human open Codex, OpenCode, or another agent inside a selected slot.

The primary workflow is:

1. register a project
2. create or remove slots as parallelism changes
3. fetch updates before work begins
4. let the human or agent run `git checkout` inside the slot

taskspace does not model tasks as persistent registry entries.
A task is temporary human intent; a slot is durable but disposable local infrastructure.

## What taskspace must guarantee

- A project maps to one Git source.
- A slot maps to one clone under one project.
- Projects and slots are local and inspectable under `state/projects/<project>/`.
- Every slot workspace receives `.taskspace/context.yaml`.
- `sync` only performs `git fetch --all --prune`.
- `enter` starts the selected agent with the slot as the working directory when the agent supports it.

## What taskspace intentionally excludes

- task lifecycle state machines
- task archives
- multi-root task views
- policy enforcement for Codex or OpenCode
- automatic hook installation into user or project config
- branch selection policy
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

- Public CLI commands: `init`, `project`, `slot`, `sync`, `enter`, `hook-context`.
- Project ids are explicit stable names such as `app`.
- Slot refs are explicit stable names such as `app:agent-1`.
- Context file path inside a slot: `.taskspace/context.yaml`.
- Default workspace root: `~/taskspace`.

## Implementation boundaries

- `taskspace-core`: project model, slot model, context model, shared errors.
- `taskspace-app`: project registry, slot registry, git clone/fetch orchestration, context writing, agent entry.
- `taskspace-infra-fs`: filesystem and process execution helpers.
- `taskspace-cli`: CLI argument parsing, direct app dispatch, completion, and output rendering.
