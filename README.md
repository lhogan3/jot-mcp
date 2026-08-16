# jot-mcp

An [MCP](https://modelcontextprotocol.io) server, implemented as a single Rust binary, that exposes the [Jot](https://apps.apple.com/) macOS notes app's Core Data SQLite store as tools. It lets an MCP client (Claude Desktop, Claude Code, or any spec-compliant client) read and write notes, folders, tags, reminders, and trash state directly in Jot's database.

Built on the official [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) SDK.

## Tools

**Notes**
- `get_notes` — search/list notes, including folder, pinned, and tag state
- `create_note` — create a note, optionally pre-checked and/or filed into a folder
- `update_note` — change title, content, and/or folder
- `set_completed` — check/uncheck a note
- `set_pinned` — pin/unpin a note
- `set_reminder` — set or clear a reminder date
- `delete_note` — move a note to Trash (soft delete)
- `restore_note` — move a note out of Trash
- `permanently_delete_note` — hard-delete a note already in Trash
- `empty_trash` — permanently delete everything in Trash

**Folders**
- `get_folders` — list folders
- `create_folder` — create a folder

**Tags**
- `get_tags` — list tags
- `create_tag` — create a tag
- `add_tag_to_note` / `remove_tag_from_note` — manage a note's tags

## Setup

Requires [Docker](https://www.docker.com/) (the server ships as a container so it doesn't need a Rust toolchain on your machine).

```sh
./setup.sh
```

This builds the image and walks you through registering it with Claude Desktop, Claude Code, or both. Run `./setup.sh --help` for flags (`--client`, `--db-path`, `--dry-run`).

Since Jot's database lives on the host and this is a stdio process spawned per-session, there's no long-running service to manage — the client just runs:

```sh
docker run -i --rm -v <host Jot data dir>:/data -e JOT_DB_PATH=/data/private.sqlite jot-mcp
```

## Known limitation

Jot's own UI does not reliably pick up writes made by this server while the app is open — it can show zero notes until Jot is force-quit and relaunched. **This is not data loss**; the underlying `.sqlite` file is unaffected. See the "Known limitation: live UI updates" section in [`CLAUDE.md`](./CLAUDE.md) for why, and what was ruled out as a fix.

**Practical guidance:** quit Jot before a batch of `jot-mcp` edits, or relaunch it afterward to see the changes.

## Development

- Build: `cargo build`
- Test: `cargo test`
- Format: `cargo fmt`
- Lint: `cargo clippy`

See [`CLAUDE.md`](./CLAUDE.md) for architecture details.

## License

MIT — see [`LICENSE`](./LICENSE).
