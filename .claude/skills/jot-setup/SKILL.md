---
name: jot-setup
description: Use when the user wants to install, set up, or configure the jot-mcp server (e.g. "set up jot-mcp", "install the jot mcp server", "connect Jot to Claude"). Builds the Docker image and registers it with Claude Desktop and/or Claude Code by driving setup.sh.
---

# jot-mcp setup

This skill installs the `jot-mcp` MCP server (reads/writes notes in the Jot macOS app) by building its Docker image and registering it with the user's MCP client(s). All the actual work happens in `setup.sh` at the repo root — this skill's job is to gather the two inputs that script needs from the user and then run it. Do not reimplement the Docker build or config-merging logic here.

## Steps

1. Confirm `docker` is installed and the Docker daemon is running (`docker info`). If not, tell the user to install/start Docker Desktop and stop.
2. Ask the user which MCP client(s) to register with: Claude Desktop, Claude Code, both, or just print the config for manual setup. Use their answer to pick `--client claude-desktop|claude-code|both|print`.
3. If the user mentions Jot's database isn't at the default location, pass `--db-path <dir>` (the directory containing `private.sqlite`, not the file itself).
4. Run `./setup.sh --client <choice> [--db-path <dir>]` from the repo root via Bash. This is a real, non-dry-run invocation — it builds a Docker image and writes to `claude_desktop_config.json` and/or runs `claude mcp add`, both of which mutate the user's actual global config. Tell the user what you're about to run before running it.
5. Report the script's output. If it configured Claude Desktop, tell the user to restart Claude Desktop. If it configured Claude Code, verify with `claude mcp list` and confirm `jot` shows up.
