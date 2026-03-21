---
name: taskspace-session-management
description: Use taskspace to manage minimal session-oriented AI workspaces safely.
---

# taskspace session management

Use this skill when work should happen inside a taskspace session.

## When to use

- The user wants to start, resume, inspect, validate, archive, or remove a `taskspace` session.
- The work spans multiple repositories and should be isolated per task.

## When not to use

- The task is a small, repository-local change and the user does not want session management.
- The request is unrelated to `taskspace` commands, workspace structure, or session lifecycle.

## Defaults

- Treat one task as one session unless the user explicitly wants to reuse an existing session.
- Create or open sessions under `~/taskspace/<name>`.
- Start from `AGENTS.md` as the session entrypoint.
- Keep project checkouts in `repos/`.
- Create additional helper files or directories only when needed.
- Prefer `archive` over `rm` when cleanup intent is unclear.

## Core taskspace model

- A session is the unit of work.
- `AGENTS.md` is the AI entrypoint.
- `workspace.yaml` is machine-readable metadata and reproducibility state.
- `repos/` is the primary working area for projects.
- Additional structures are optional and created on demand.

## Standard workflow

1. Validate the environment when session state may be unclear.

```bash
taskspace doctor
```

2. Create a new session for new work.

```bash
taskspace new <name> [--template <local-yaml>] [--open] [--editor <name>]
```

3. If the session already exists, open it instead of creating another one.

```bash
taskspace open <name> [--editor <name>]
```

If the user wants to resume the latest session and does not name one, use:

```bash
taskspace open --last
```

4. If the target session is unclear, inspect available sessions first.

```bash
taskspace list
```

## Workspace coordination

- Treat each `repos/<project>` as its own project with independent conventions.
- Put deliverables in the correct project directory under `repos/`.
- If coordination artifacts are needed, create them explicitly in session root or a dedicated helper directory.

## Safety boundaries

- Do not run destructive commands without explicit user intent.
- Prefer `taskspace archive <name>` for completed work.
- Use `taskspace rm <name> --dry-run` before deletion when the target is not fully clear.
- Run `taskspace doctor` when the workspace appears inconsistent, partially initialized, or broken.

## Validation checklist

- The correct session has been created or opened.
- `AGENTS.md`, `workspace.yaml`, and `repos/` exist in the session.
- Project changes are made inside the intended directories under `repos/`.
- `taskspace doctor` reports no FAIL checks before handoff when session health matters.
- Cleanup actions match explicit user intent.

## Gotchas

- Do not create a second session for the same task unless the user asks for it.
- Do not use `rm` as routine cleanup; deletion is higher risk than archiving.
- `taskspace doctor` validates environment and workspace health, but it does not replace repository-specific build or test checks.

## Command reference

```bash
taskspace new <name> [--template <local-yaml>] [--open] [--editor <name>]
taskspace open <name> [--editor <name>]
taskspace open --last
taskspace list
taskspace ls
taskspace doctor
taskspace archive <name>
taskspace rm <name>
taskspace remove <name>
taskspace rm <name> --dry-run
taskspace rm <name> --yes
```
