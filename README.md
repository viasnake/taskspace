# taskspace

Session-oriented workspace manager for AI coding workflows.

## License

This project is currently distributed as **NO LICENSE**.

## Scope (v0.1.0)

- Implemented: `new`, `open`, `list`, `rm`, `archive`, `doctor`
- Supported OS: Linux, macOS

## Quick Start

1. Install tools with `mise`.
2. Build CLI.
3. Create a session and open it.

```bash
mise install
mise run build
./target/debug/taskspace new demo --open --editor code
```

## Commands

```bash
taskspace new <name> [--repo <name>=<path|url>]... [--open] [--editor opencode|code]
taskspace open <name> [--editor opencode|code]
taskspace list
taskspace rm <name> [--yes] [--dry-run]
taskspace archive <name>
taskspace doctor
```

`rm` is destructive and requires `--yes` unless `--dry-run` is used.

## Global SKILLS Template

Use `templates/SKILLS.md` as a starting point for either:

- `~/.taskspace/SKILLS.md`, or
- `~/.config/taskspace/SKILLS.md`

The template is designed to teach AI agents how to operate taskspace safely and consistently.

## Design intent

See `docs/design-intent.md`.

## Release notes

Release notes are managed on GitHub Releases.

## Development

```bash
mise run check
```

This runs formatting, linting, tests, and dependency audit checks.

For coverage, run `mise run coverage` (line coverage gate: 70%+).

Dependency and GitHub Actions version updates are automated via `.github/dependabot.yml`.

`deny.toml` defines the dependency policy used by `cargo deny check`.
