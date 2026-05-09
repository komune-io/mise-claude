In addition to the standard security checks, pay special attention to:

## Bootstrap Surface (Lua hooks)

This repository is a mise backend plugin whose only responsibility is to install and run the `claude-env` Rust binary. The Lua hooks shell out, so quoting matters.

- **`hooks/backend_install.lua`** — Interpolates `version` and `install_path` (from mise) into a `cargo install claude-env --version <v> --root <path> --locked` command. Verify every interpolated value passes through `shell_quote()`. Verify `--locked` is present in any future change to this command (it pins the dependency tree resolved at publish time).
- **`hooks/backend_exec_env.lua`** — Interpolates `pwd` (read via `cmd.exec("pwd")`) and the binary path into a `cd <pwd> && <bin> install --idempotent --quiet` command. Verify both values pass through `shell_quote()`. The presence check for `claude-env.toml` is via `io.open`, not shell — that path is fine.
- **`hooks/backend_list_versions.lua`** — Pure HTTP GET to crates.io plus JSON decode. No shell. Verify the URL stays hard-coded (no interpolation).

## Delegated Surface (claude-env binary)

Out of scope for this plugin's scan. `claude-env` has its own threat model — package install routing, lockfile handling, MCP config writes, plugin marketplace fetches — and is reviewed separately as part of its crates.io publish pipeline. Do not duplicate findings here.

## Supply Chain

- The `cargo install --locked` flag pins the dependency tree resolved at publish time. Loss of `--locked` means transitive deps could shift between installs. Flag any change that removes it.
- The crates.io HTTP fetch in `backend_list_versions.lua` and `backend_install.lua` trusts the registry's response shape. A malformed response would error out; this is acceptable.
