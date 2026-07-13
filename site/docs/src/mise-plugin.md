# mise plugin

chord ships a [mise](https://mise.jdx.dev) backend plugin that bootstraps the
binary and keeps your environment topped up automatically. It's an **optional
convenience** — a secondary path, not the core way to use chord.

## Usage

Add the plugin:

```bash
mise plugin install chord https://github.com/rytmyk-ai/chord
```

Declare chord in your project's `.mise.toml`:

```toml
# .mise.toml
[tools]
chord = "latest"
```

Then:

```bash
mise install
```

From then on, when you `cd` into a directory containing `chord.toml`, the plugin
runs `chord install --idempotent --quiet` automatically. Missing tools are
installed silently; nothing happens if everything is already up to date.

## How it works

The plugin does only two things, via three Lua backend hooks:

1. **List / install** — queries crates.io for `rytmyk-chord` versions
   (`backend_list_versions`) and runs `cargo install rytmyk-chord --locked`
   when mise resolves the tool (`backend_install`).
2. **Shell entry** — adds the binary to `PATH` and runs the idempotent install
   when `chord.toml` is present (`backend_exec_env`).

All real tool management — MCP installs, plugin fetches, skills setup, lockfile
maintenance — lives in the `chord` binary, not the plugin. See the
[repository](https://github.com/rytmyk-ai/chord) for the hook source.
