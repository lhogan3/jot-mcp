# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`jot-mcp` is an MCP (Model Context Protocol) server, implemented as a single Rust binary, that exposes the [Jot](https://apps.apple.com/) macOS notes app's Core Data SQLite store as MCP tools. It lets an MCP client (e.g. Claude Desktop, Claude Code) read and create notes/tasks directly in Jot's database. Built on the official [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) SDK, so it's spec-compliant with any MCP client, not just Claude's.

## Commands

- Build: `cargo build`
- Run: `cargo run`
- Check (fast compile check without producing a binary): `cargo check`
- Test: `cargo test`
- Run a single test: `cargo test <test_name>`
- Format: `cargo fmt`
- Lint: `cargo clippy`
- Docker image: `docker build -t jot-mcp .`
- End-to-end setup for a user (builds the image, registers with Claude Desktop/Code): `./setup.sh` (see flags via `./setup.sh --help`; supports `--dry-run`)

Edition is 2024 (Cargo.toml).

## Architecture

Split across three modules plus a thin entry point (`src/main.rs` wires `mod` declarations and calls `mcp::serve()` inside a single-threaded Tokio runtime):

1. **`src/db.rs`** — low-level connection/time helpers shared by the data layer. The DB path is resolved by `db_path()`: it reads the `JOT_DB_PATH` env var, falling back to `$HOME/Library/Group Containers/group.hirocloud.jotApp/CoreDataStores/Private/private.sqlite`. The env var exists so the path can be repointed at a bind-mounted location inside Docker (see below) or for any user whose home directory differs from the original author's. Core Data stores timestamps as seconds since the **Apple/Cocoa epoch** (2001-01-01), not Unix epoch; `APPLE_EPOCH_OFFSET` (978,307,200s) converts between the two, and `current_apple_time()` produces a Core Data-compatible timestamp for new rows.

2. **`src/notes.rs`** — data layer that talks to the `ZCDNOTE` table (Core Data's `Z`-prefixed naming convention: `Z_PK`, `ZTITLE`, `ZTEXT`, `ZCOMPLETED`, `ZCREATED_AT`, etc.).
   - `get_notes` — reads/searches notes (`LIKE` match on title/content).
   - `create_note` — inserts a new row. Because Core Data manages its own primary keys, this manually reads and increments `Z_PRIMARYKEY.Z_MAX` for the `CDNote` entity to allocate `Z_PK`, and looks up `Z_ENT` for the same entity — both must stay in sync with Core Data's bookkeeping or the app-side ORM will misbehave.

3. **`src/mcp.rs`** — the MCP server, built with `rmcp`'s macro-based API rather than hand-rolled JSON-RPC.
   - `JotServer` holds a `ToolRouter<Self>` (built via `Self::tool_router()`); the `#[tool_router]`/`#[tool_handler]` macros generate the protocol dispatch (`initialize`, `tools/list`, `tools/call`, notification handling) from the `#[tool(...)]`-annotated methods, so there's no manual JSON-RPC parsing.
   - `get_notes`/`create_note` methods on `JotServer` take a `Parameters<T>`-wrapped, `schemars`-derived params struct (`GetNotesParams`, `CreateNoteParams`) and delegate straight to the matching `notes::` function, converting DB errors into a human-readable string result (not a protocol-level error) — this preserves the original tool UX.
   - `ServerHandler::get_info` sets `server_info` (name/version) manually since `Implementation`/`InitializeResult` are `#[non_exhaustive]` — construct them via `Implementation::new(...)` / `ServerInfo::new(...)`, not struct literals.
   - `serve()` runs `JotServer::new().serve(stdio()).await?.waiting().await?`.

When adding a new tool: add the data-layer function to `notes.rs`, define its params struct and a `#[tool(...)]` method on `JotServer` in `mcp.rs`. There's no separate schema/dispatch table to keep in sync — the macro derives both from the method signature.

Note: `rusqlite` calls in `notes.rs` are synchronous/blocking and are called directly from the async tool handlers rather than via `spawn_blocking`. This is intentional — stdio MCP servers process one request at a time, so there's no concurrent work being blocked.

## Known limitation: live UI updates

Jot's Core Data store uses persistent history tracking (`ATRANSACTION`/`ACHANGE` tables) — Jot's own UI relies on this to detect and merge changes made by other processes. `jot-mcp` writes via raw SQL (`rusqlite`), which never populates these tables, so **the running Jot app does not reliably pick up writes made by this server while it's open**; it has been observed to show zero notes until the app is force-quit and relaunched. This is not data loss — the underlying `.sqlite` file is unaffected — confirmed by re-querying `ZCDNOTE` directly after the app appeared empty.

Two things were ruled out as fixes:
- Setting `Z_OPT`/`ZCLOUDVERSION` (Core Data's optimistic-lock/CloudKit version counters) on every write, matching what Core Data itself writes on save — `notes.rs`'s write paths do this, but it did not resolve the blank-UI issue when tested directly.
- Jot exposes no AppleScript scripting dictionary (no `.sdef`, no `NSAppleScriptEnabled`/`OSAScriptingDefinition` in its `Info.plist`), so there's no supported external "refresh" command to call instead.

Actually fixing this would require writing matching rows to Core Data's private, undocumented persistent-history BLOB format (`ACHANGE.ZCOLUMNS`, `ATRANSACTION.ZQUERYGEN`) — not attempted, since getting that wrong risks real store corruption for a convenience fix. **Practical guidance: quit Jot before a batch of `jot-mcp` edits, or expect to relaunch it afterward to see the changes.**

## Docker / distribution

- `Dockerfile` — multi-stage build: `rust:1-slim` + `build-essential` (needed because `rusqlite`'s `bundled` feature compiles SQLite from source) to build a release binary, copied into a bare `debian:trixie-slim` runtime image. No SQLite runtime lib is needed since `bundled` statically links it.
- Since Jot's database lives on the host and this is a stdio process a client spawns per-session (not a long-running networked service), there's no docker-compose — it's `docker run -i --rm -v <host Jot data dir>:/data -e JOT_DB_PATH=/data/private.sqlite jot-mcp`, with the client (Claude Desktop/Code) configured to spawn exactly that command.
- `setup.sh` builds the image and, based on `--client`, either merges an `mcpServers.jot` entry into Claude Desktop's config (via an inline `python3` JSON merge — chosen because it ships with macOS and won't clobber other configured servers) or runs `claude mcp add jot --scope user -- docker ...` for Claude Code.
- `.claude/skills/jot-setup/SKILL.md` is a Claude Code skill that drives `setup.sh` conversationally (asks the user which client, runs the script) rather than reimplementing its logic — keep it that way if it needs updating.
