# taskspace

**taskspace** is a session-oriented workspace manager for AI coding.

It creates an isolated workspace per task, combining multiple repositories and AI context files into a single working environment.

## ✨ Features

* **Session-based workflow** (1 task = 1 workspace)
* **Multi-repository support**
* **Non-commit AI context layer** (`MEMORY`, `PLAN`, etc.)
* **Global AI skills definition (`SKILLS.md`)**
* **Editor integration (OpenCode / VS Code)**
* **Agent-agnostic design** (OpenCode, Claude Code, Gemini CLI, etc.)

## 📦 Install

### Homebrew (recommended)

```bash
brew install viasnake/tap/taskspace
```

### mise (optional)

```bash
mise use -g github:viasnake/taskspace@latest
```

> `taskspace@latest` shortcut will be available after mise registry integration.

## 🚀 Quick Start

```bash
taskspace doctor
taskspace new demo --open --editor code
```

This will:

* Create a new session workspace
* Initialize context files
* Generate OpenCode configuration
* Open the workspace in your editor

## 🧠 Concepts

### Session

A unit of work (e.g., bug fix, feature, investigation)

```
1 task = 1 session = 1 workspace
```

### Workspace

A directory containing:

* repositories
* context files
* AI configuration

### Context Layer

Non-commit files for AI:

```
context/
  MEMORY.md
  PLAN.md
  CONSTRAINTS.md
  DECISIONS.md
```

### Global Skills

AI behavior is defined by a global file:

```
~/.taskspace/SKILLS.md
```

This file teaches the AI:

* how to work
* how to update context
* how to operate safely

## 📁 Directory Structure

```
~/taskspace/
  sessions/
    <session>/
      SESSION.md
      AGENTS.md
      workspace.yaml

      workspace.code-workspace

      .opencode/
        opencode.jsonc

      context/
      repos/
      references/
      notes/
      output/
```

## ⚙️ Commands

### Create a session

```bash
taskspace new <name> \
  --repo app=~/src/app \
  --repo infra=~/src/infra \
  --open
```

### Open a session

```bash
taskspace open <name>
```

### List sessions

```bash
taskspace list
```

### Remove a session

```bash
taskspace rm <name>
```

### Archive a session

```bash
taskspace archive <name>
```

### Diagnose environment

```bash
taskspace doctor
```

## 🧩 AI Integration

taskspace works with:

* OpenCode
* Claude Code
* Gemini CLI
* Any AI coding agent that reads workspace files

### Instruction Order

AI reads:

1. `SESSION.md`
2. `AGENTS.md`
3. `~/.taskspace/SKILLS.md`
4. `context/CONSTRAINTS.md`
5. `context/MEMORY.md`
6. `context/PLAN.md`

## 🔒 Safety

* Context files are **never committed**
* Destructive commands are restricted
* AI behavior is controlled via `AGENTS.md` and `SKILLS.md`

## 🛠 Development

```bash
git clone https://github.com/viasnake/taskspace
cd taskspace

mise install
mise run build
mise run check
```

## 📌 Requirements

* Git
* OpenCode (or compatible AI CLI)
* VS Code (optional)

## 🧭 Philosophy

> Workspace is the unit of work — not the repository.

taskspace treats:

* repositories
* notes
* AI context

as equal components of a single working environment.

## 📄 License

TBD

## 💡 Example

```bash
taskspace new auth-fix \
  --repo app=~/src/app \
  --repo infra=~/src/infra \
  --open
```

Start coding immediately with full AI context.
