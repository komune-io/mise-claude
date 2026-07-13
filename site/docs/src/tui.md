# TUI

```bash
chord inspect --tui
```

An interactive terminal UI over your Claude Code configuration — the same audit
as `chord inspect`, but navigable, and able to **make changes inline** without
leaving it.

## Layout

Two panes: a **tree** of everything chord found — MCP servers, plugins, skills,
commands, agents, project *and* global — on the left, and a **preview** of the
selected item on the right. `Tab` switches focus between them.

## Keys

| Key | Action |
|-----|--------|
| `↑` `↓` / `k` `j` | Move selection |
| `←` `→` / `h` `l` | Collapse / expand |
| `Enter` | Toggle expand |
| `Tab` | Switch tree ↔ preview focus |
| `/` | Search / filter |
| `i` | Toggle "enabled only" filter |
| `v` | View the selected item's file |
| `a` | Add a tool |
| `d` | Remove the selected (chord-managed) tool |
| `r` | Install the selected drift entry |
| `R` | Install everything |
| `e` | Change a plugin's scope (project / global) |
| `q` · `Esc` · `Ctrl-C` | Quit |

## Relation to `chord inspect`

The TUI and the plain command surface the same audit. Use `chord inspect` (or
`chord inspect --json`) for scripting and CI; use `--tui` to explore and fix
things interactively.
