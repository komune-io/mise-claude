# Introduction

**chord** is a declarative agent-tool environment manager. Declare your MCP
servers, skills, plugins, and CLI tools in one file — `chord install` handles
the rest, reproducing the same environment on every machine.

chord is part of the [rytmyk](https://github.com/rytmyk-ai) toolchain.

## The model

1. **Declare** what you want in `chord.toml`.
2. **Install** it with `chord install`.
3. **Audit** the result with `chord inspect`.

```toml
# chord.toml
[mcp]
context7 = "2.1.4"

[skills]
"anthropics/skills/frontend-design" = "latest"

[plugins]
"upstash/context7/context7-plugin@context7-marketplace" = "latest"

[cli]
gsd = "1.22.4"
```

```bash
chord install     # materialize everything above
chord inspect     # see what's installed, project + global
```

Installs are **idempotent**: re-running `chord install` skips anything already
present and only fills in what's missing.

## How these docs are organized

- **[chord.toml reference](./chord-toml.md)** — the four tables you declare in.
- **[Commands](./commands.md)** — every `chord` verb, with examples.
- **[TUI](./tui.md)** — the interactive `chord inspect --tui` audit view.
- **[mise plugin](./mise-plugin.md)** — the optional mise bootstrap.

> **Installing the binary** (`cargo install rytmyk-chord`) is developer setup and
> lives in the repository README — these docs focus on *using* chord.

## Roadmap

chord is growing into an agent-agnostic package manager. Today it targets Claude
Code; planned support includes Codex, OpenCode, and aider.
