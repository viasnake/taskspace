# taskspace

Session-oriented workspace manager for AI coding workflows.

## Quick Start

1. Install tools with `mise` or `brew`:

```bash
brew install viasnake/tap/taskspace
mise use -g github:viasnake/taskspace@latest
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
