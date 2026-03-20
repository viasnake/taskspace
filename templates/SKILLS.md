# taskspace SKILLS

This document explains how an AI agent should operate `taskspace` correctly.

## Core understanding

- One task equals one session.
- A session is created under `~/taskspace/sessions/<name>`.
- Repository code lives in `repos/`.
- AI context lives in `context/` and must be treated as non-commit data.

## Command usage guide

### Start work

1. Create a session:

```bash
taskspace new <name> [--repo <name>=<path|url>]... [--open] [--editor opencode|code]
```

2. If the session already exists, open it:

```bash
taskspace open <name> [--editor opencode|code]
```

3. Check available sessions:

```bash
taskspace list
```

4. Validate environment and session health:

```bash
taskspace doctor
```

### End or clean up work

- Archive session (recommended for completed work):

```bash
taskspace archive <name>
```

- Remove session only when explicitly requested:

```bash
taskspace rm <name> --yes
```

- Preview remove target safely:

```bash
taskspace rm <name> --dry-run
```

## Context handling rules

- Update `context/PLAN.md` before major changes.
- Record important decisions in `context/DECISIONS.md`.
- Keep constraints in `context/CONSTRAINTS.md`.
- Keep working memory in `context/MEMORY.md`.
- Keep references in `context/LINKS.md`.

## Multi-repository workflow

- Treat each `repos/<repo>` as an independent repository.
- If the change spans repos, record coordination steps in `context/PLAN.md`.
- Keep shared design decisions in `context/DECISIONS.md`.

## Safety requirements

- Do not commit `context/` files unless explicitly instructed.
- Do not run destructive operations without explicit user intent.
- Prefer `archive` over `rm` when cleanup intent is unclear.
- Run `taskspace doctor` when the workspace is inconsistent.

## Completion checklist for agents

- Session is created/opened in taskspace.
- Relevant context files are updated.
- Work output exists in appropriate `repos/` directories.
- `taskspace doctor` has no FAIL checks.
- Cleanup action (`archive` or `rm`) follows explicit user intent.
