# taskspace

**taskspace** is a session-oriented workspace manager for AI coding.

It creates an isolated workspace per task, combining multiple repositories and AI context files into a single working environment.

## ✨ Features

* **Session-based workflow** (1 task = 1 workspace)
* **Multi-repository support**
* **Non-commit AI context layer** (`MEMORY`, `PLAN`, etc.)
* **Global AI skill definition (`SKILL.md`)**
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
  LINKS.md
```

### Global Skill

AI behavior can be guided by a reusable taskspace skill.

taskspace keeps a convenient personal copy at:

```
~/.taskspace/SKILL.md
```

This file teaches the AI:

* how to work inside a session
* how to update context files
* how to coordinate multi-repo work
* how to operate safely

The `SKILL.md` in this repository is the template you can copy or adapt for your editor's skill directory.

### Recommended Skill Installation

If you want your AI editor to use taskspace well without extra prompting, install this repository's `SKILL.md` as a reusable global skill.

Recommended cross-editor location:

```bash
mkdir -p ~/.agents/skills/taskspace-session-management
cp ./SKILL.md ~/.agents/skills/taskspace-session-management/SKILL.md
```

Why this path:

* many agent-skill-compatible tools can use `~/.agents/skills/`
* OpenCode can also use agent-compatible skill locations such as `.agents/skills`
* The skill name matches the frontmatter name: `taskspace-session-management`

OpenCode native global location:

```bash
mkdir -p ~/.config/opencode/skills/taskspace-session-management
cp ./SKILL.md ~/.config/opencode/skills/taskspace-session-management/SKILL.md
```

Repository-local installation for a single project:

```bash
mkdir -p .agents/skills/taskspace-session-management
cp ./SKILL.md .agents/skills/taskspace-session-management/SKILL.md
```

This is useful when you want taskspace behavior available only inside one repository or workspace.

### What the Skill Helps the AI Do

Once installed, the skill nudges the AI to:

* create a new session for new work instead of editing random repositories directly
* reopen an existing session instead of duplicating workspaces
* keep planning, decisions, and memory in `context/`
* treat each `repos/<repo>` directory as an independent repository
* prefer `archive` over destructive cleanup
* run `taskspace doctor` when workspace state looks wrong

### Editor Notes

* **OpenCode**: can use its native skill directory and agent-compatible locations such as `.agents/skills`
* **Codex-style setups**: usually install reusable skills as `.agents/skills/<name>/SKILL.md`
* **Other editors**: if your editor supports the agent skills standard or `SKILL.md`-based reusable instructions, use the same `taskspace-session-management` folder name and copy this file there

If your editor supports both a global skill directory and a repository-local one, use the global directory when you want taskspace conventions everywhere and the repository-local directory when you want them only in selected repos.

## 📁 Directory Structure

```
~/taskspace/
  .archive/
    <session>-<timestamp>/

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
taskspace open        # opens latest session
taskspace open --last # opens latest session explicitly
```

### List sessions

```bash
taskspace list
taskspace ls
```

If there are no sessions, `taskspace list` prints `no sessions found`.

### Remove a session

```bash
taskspace rm <name>
taskspace remove <name>
taskspace rm <name> --dry-run
taskspace rm <name> --yes
```

`rm` is destructive. In interactive terminals it asks for confirmation when `--yes` is omitted, and in non-interactive environments it requires `--yes` unless `--dry-run` is used.

### Archive a session

```bash
taskspace archive <name>
```

### Diagnose environment

```bash
taskspace doctor
```

### Show version

```bash
taskspace -v
taskspace -V
taskspace --version
```

## 🧩 AI Integration

taskspace works with:

* OpenCode
* Claude Code
* Gemini CLI
* Any AI coding agent that reads workspace files

### Instruction Order

A typical taskspace-aware AI setup reads:

1. `SESSION.md`
2. `AGENTS.md`
3. an installed taskspace `SKILL.md` from your editor's skill directory
4. `context/CONSTRAINTS.md`
5. `context/MEMORY.md`
6. `context/PLAN.md`

### Recommended AI Workflow

1. Install `SKILL.md` into your editor's global or repository-local skill directory.
2. Run `taskspace new <name> --open` for new work.
3. Let the AI operate inside the created session workspace.
4. Keep cross-repo intent in `context/PLAN.md` and durable decisions in `context/DECISIONS.md`.
5. Archive completed sessions unless you explicitly want deletion.

## 🔒 Safety

* Context files are **never committed**
* Destructive commands are restricted
* AI behavior is controlled via `AGENTS.md` and `SKILL.md`

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
