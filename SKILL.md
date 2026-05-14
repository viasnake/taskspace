---
name: taskspace-slot-management
description: Use taskspace to manage registered Git projects, dynamic clone slots, and pre-work sync for local AI-agent workflows.
---

# taskspace slot management

Use this skill when work should be coordinated through `taskspace` project registration, slot creation, sync, and agent entry.

## Core model

- Project is the primary registered unit.
- A project maps to one Git source.
- A slot is one clone under one project.
- `sync` fetches updates; `enter` launches the agent in the selected slot.
- Registry is stored under `~/taskspace/state/projects/`.

## Standard workflow

```bash
taskspace init
taskspace project add app ~/src/app
taskspace slot add app --count 2
taskspace sync --all
taskspace enter app:agent-1 --agent codex
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
```
