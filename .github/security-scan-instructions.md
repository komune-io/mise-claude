In addition to the standard security checks, pay special attention to:

## Install Hook Security

This is a mise backend plugin that installs tools by running shell commands. Review changes to install logic (`hooks/backend_install.lua`, `lib/aliases.lua`) for:

- **Command injection**: Tool names, versions, and paths are interpolated into shell commands. Ensure all user-controlled values are properly quoted/escaped via `shell_quote()`.
- **Path traversal**: Verify that install paths, project roots, and binary names cannot escape intended directories.
- **Arbitrary code execution**: `post_install` commands from `TOOL_REGISTRY` and `npx`/`npm install` invocations should not allow untrusted input to influence what gets executed.

## Skills & Plugin Installation

- **skills.sh installs** (`npx skills add`): Check that owner/repo/skill parsing cannot be manipulated to run unintended commands.
- **Plugin installs** (`claude plugin marketplace add`, `claude plugin install`): Verify that plugin names and marketplace identifiers are sanitized.
- **Lock file races**: Concurrent install uses mkdir-based locks. Check for TOCTOU or bypass issues.

## MCP Server Registration

- Verify that `.mcp.json` entries use absolute paths to binaries within the expected `node_modules/.bin/` directory.
- Check that MCP server arguments (`mcp_args`) cannot be tampered with.
