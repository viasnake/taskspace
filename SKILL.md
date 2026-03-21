---
name: taskspace-session-management
description: Use taskspace to manage session-oriented AI workspaces, including session creation, opening, validation, context updates, multi-repo coordination, and safe cleanup.
---

# taskspace session management

Use this skill when work should happen inside a taskspace session instead of directly in a repository.

## When to use

- The user wants to start, resume, inspect, validate, archive, or remove a `taskspace` session.
- The work spans multiple repositories and needs shared context in one workspace.
- The work needs durable AI context such as plans, decisions, constraints, references, or short-term memory.

## When not to use

- The task is a small, repository-local change and the user does not want session management.
- The request is unrelated to `taskspace` commands, workspace structure, or session lifecycle.

## Defaults

- Treat one task as one session unless the user explicitly wants to reuse an existing session.
- Create or open sessions under `~/taskspace/<name>`.
- Keep repository code in `repos/`.
- Keep AI working context in `context/`.
- Treat `context/` as non-commit data unless the user explicitly says otherwise.
- Prefer `archive` over `rm` when cleanup intent is unclear.

## Core taskspace model

- A session is the unit of work.
- A workspace contains repositories, AI context, and editor configuration.
- Each repository under `repos/<repo>` remains an independent repository.
- Shared planning and coordination live in `context/`, not inside individual repositories.

## Standard workflow

1. Validate the environment when session state may be unclear.

```bash
taskspace doctor
```

2. Create a new session by default for new work.

```bash
taskspace new <name> [--repo <name>=<path|url>]... [--open] [--editor opencode|code]
```

3. If the session already exists, open it instead of creating another one.

```bash
taskspace open <name> [--editor opencode|code]
```

If the user wants to resume the latest session and does not name one, use:

```bash
taskspace open --last
```

4. If the target session is unclear, inspect available sessions first.

```bash
taskspace list
```

## Context file rules

- Update `context/PLAN.md` before major implementation work or cross-repo changes.
- Record durable decisions in `context/DECISIONS.md` when they affect future work.
- Keep active constraints in `context/CONSTRAINTS.md`.
- Keep short-term task memory in `context/MEMORY.md`.
- Store useful references in `context/LINKS.md`.
- Do not create duplicate planning systems in repository files when `context/` is the right shared location.

## Multi-repo coordination

- Treat each `repos/<repo>` as its own repository with its own git history and conventions.
- Track cross-repo steps and dependencies in `context/PLAN.md`.
- Capture cross-repo design or policy decisions in `context/DECISIONS.md`.
- Put deliverables in the correct repository instead of the session root unless the file is intentionally shared session context.

## Safety boundaries

- Do not commit `context/` files unless the user explicitly requests it.
- Do not run destructive commands without explicit user intent.
- Prefer `taskspace archive <name>` for completed work.
- Use `taskspace rm <name> --dry-run` before deletion when the target is not fully clear.
- Run `taskspace doctor` when the workspace appears inconsistent, partially initialized, or broken.

## Validation checklist

- The correct session has been created or opened.
- Relevant `context/` files reflect the current task state.
- Repository changes are made inside the intended `repos/<repo>` targets.
- `taskspace doctor` reports no FAIL checks before handoff when session health matters.
- Cleanup actions match explicit user intent.

## Gotchas

- Do not create a second session for the same task unless the user asks for it.
- Do not treat `context/` as project source code by default.
- Do not use `rm` as routine cleanup; deletion is higher risk than archiving.
- `taskspace doctor` validates environment and workspace health, but it does not replace repository-specific build or test checks.

## Command reference

```bash
taskspace new <name> [--repo <name>=<path|url>]... [--open] [--editor opencode|code]
taskspace open <name> [--editor opencode|code]
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
