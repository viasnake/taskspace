---
name: taskspace-task-management
description: Use taskspace to manage task-first multi-root AI workspaces.
---

# taskspace task management

Use this skill when work should be coordinated through `taskspace` task lifecycle.

## Core model

- Task is the primary unit.
- A task can have multiple roots (`git`, `dir`, `file`, `artifact`, `scratch`).
- `enter` resolves workspace and launches the adapter.
- Registry is stored under `~/.local/state/taskspace/registry/tasks/`.

## Standard workflow

```bash
taskspace start "task title"
taskspace attach current <path> --type git --role source --rw --isolation worktree
taskspace attach current <path> --type dir --role docs --ro
taskspace verify current
taskspace finish current --state done
taskspace archive current
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
```
