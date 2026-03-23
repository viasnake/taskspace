# taskspace design intent

## What this tool is

taskspace is a task-oriented multi-root workspace manager for AI coding.
It treats a task as the primary execution unit and resolves workspace roots before entering an AI client.

## What v0.5.0 must guarantee

- Task-first lifecycle with explicit states (`active`, `blocked`, `review`, `done`, `archived`).
- Multi-root task model (`git`, `dir`, `file`, `artifact`, `scratch`).
- Fixed local registry and scratch locations under `~/.local/state/taskspace/`.
- `enter` resolves workspace context and launches adapter in one step.
- Verification remains lightweight but first-class.

## What v0.5.0 intentionally excludes

- backward compatibility with old session layout/commands
- migration from old workspace schema
- policy engine and orchestration control plane
- audit and RBAC subsystems

## Stable contracts

- Public CLI commands: `start`, `attach`, `detach`, `enter`, `list`, `show`, `verify`, `finish`, `archive`, `gc`.
- `current` task selector resolves to the latest active task.
- OpenCode is the first-priority adapter, while core remains adapter-agnostic.
- Exit codes map to `TaskspaceError` categories.

## Implementation boundaries

- `taskspace-core`: task model, root model, lifecycle and errors.
- `taskspace-app`: task registry lifecycle, resolver flow, verification and adapter entry.
- `taskspace-infra-fs`: filesystem and process execution helpers.
- `taskspace-cli`: command parsing, execution routing and output rendering.
