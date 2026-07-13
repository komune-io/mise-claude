# TUI

```bash
chord inspect --tui
```

Launches the interactive audit view — the same information as `chord inspect`,
in a navigable terminal UI.

## What it shows

Your full Claude Code configuration, project and global:

- **MCP servers** — what's configured and where it came from.
- **Plugins** — installed marketplace plugins.
- **Skills** — installed skills.
- **Commands** and **Agents** — registered slash commands and agents.

## Navigating

- Move between categories and entries with the arrow keys.
- Select an entry to see its detail.
- Quit with `q`.

## Relation to `chord inspect`

The TUI and the plain command surface the same audit. Use `chord inspect` (or
`chord inspect --json`) for scripting and CI; use `--tui` to explore
interactively.

<!-- Screenshots of the TUI to be added. -->
