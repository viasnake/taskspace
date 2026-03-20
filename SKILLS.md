# taskspace SKILLS

This document defines how an AI agent should operate `taskspace` safely and consistently.

## Core model

- One task equals one session.
- A session is created directly under `~/taskspace/<name>`.
- Repository code lives in `repos/`.
- AI working context lives in `context/` and is non-commit data by default.

## Start work

1. Create a session:

```bash
taskspace new <name> [--repo <name>=<path|url>]... [--open] [--editor opencode|code]
```

2. If it already exists, open it:

```bash
taskspace open <name> [--editor opencode|code]
```

3. List available sessions:

```bash
taskspace list
```

4. Validate environment and session health:

```bash
taskspace doctor
```

## Context rules

- Update `context/PLAN.md` before major changes.
- Record key decisions in `context/DECISIONS.md`.
- Keep constraints in `context/CONSTRAINTS.md`.
- Keep short-term working memory in `context/MEMORY.md`.
- Store references in `context/LINKS.md`.

## Multi-repo operation

- Treat each `repos/<repo>` as an independent repository.
- If work spans repositories, track coordination in `context/PLAN.md`.
- Capture cross-repo design decisions in `context/DECISIONS.md`.

## Cleanup

- Archive completed work by default:

```bash
taskspace archive <name>
```

- Delete only when explicitly requested:

```bash
taskspace rm <name> --yes
```

- Preview deletion target first when needed:

```bash
taskspace rm <name> --dry-run
```

## Safety rules

- Do not commit `context/` files unless explicitly instructed.
- Do not run destructive operations without explicit user intent.
- Prefer `archive` over `rm` when cleanup intent is unclear.
- Run `taskspace doctor` when workspace state is inconsistent.

## Completion checklist

- Session is created or opened in taskspace.
- Relevant context files are updated.
- Deliverables are placed in the correct `repos/` targets.
- `taskspace doctor` has no FAIL checks.
- Cleanup action follows explicit user intent.
