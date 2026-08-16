#!/usr/bin/env bash
# Builds the jot-mcp Docker image and (optionally) registers it with
# Claude Desktop and/or Claude Code.
set -euo pipefail

IMAGE_TAG="jot-mcp"
DEFAULT_DB_DIR="$HOME/Library/Group Containers/group.hirocloud.jotApp/CoreDataStores/Private"
DB_FILE_NAME="private.sqlite"

DB_DIR="$DEFAULT_DB_DIR"
CLIENT=""
DRY_RUN=0

usage() {
    cat <<EOF
Usage: ./setup.sh [--client claude-desktop|claude-code|both|print] [--db-path DIR] [--dry-run]

  --client     Which MCP client to configure. If omitted, you'll be prompted
               (requires an interactive terminal).
  --db-path    Directory containing Jot's ${DB_FILE_NAME} (default: the
               standard macOS location under \$HOME).
  --dry-run    Print what would change without writing any files or running
               'claude mcp add'.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --client) CLIENT="$2"; shift 2 ;;
        --db-path) DB_DIR="$2"; shift 2 ;;
        --dry-run) DRY_RUN=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

if ! command -v docker >/dev/null 2>&1; then
    echo "Error: docker is not installed or not on PATH. Install Docker Desktop first." >&2
    exit 1
fi

if [[ ! -f "$DB_DIR/$DB_FILE_NAME" ]]; then
    echo "Warning: no Jot database found at '$DB_DIR/$DB_FILE_NAME'." >&2
    echo "         Pass --db-path to point at the directory containing $DB_FILE_NAME if this is wrong." >&2
fi

echo "Building Docker image '$IMAGE_TAG'..."
if [[ $DRY_RUN -eq 0 ]]; then
    docker build -t "$IMAGE_TAG" "$(dirname "$0")"
else
    echo "(dry run) docker build -t $IMAGE_TAG $(dirname "$0")"
fi

if [[ -z "$CLIENT" ]]; then
    if [[ -t 0 ]]; then
        echo ""
        echo "Which MCP client should jot-mcp be registered with?"
        select choice in "Claude Desktop" "Claude Code" "Both" "Just print the config"; do
            case "$choice" in
                "Claude Desktop") CLIENT="claude-desktop"; break ;;
                "Claude Code") CLIENT="claude-code"; break ;;
                "Both") CLIENT="both"; break ;;
                "Just print the config") CLIENT="print"; break ;;
                *) echo "Invalid choice." ;;
            esac
        done
    else
        echo "Error: no --client given and not running in an interactive terminal." >&2
        usage
        exit 1
    fi
fi

DOCKER_ARGS=(run -i --rm -v "$DB_DIR:/data" -e "JOT_DB_PATH=/data/$DB_FILE_NAME" "$IMAGE_TAG")

print_snippet() {
    echo ""
    echo "Command to run jot-mcp:"
    printf '  docker'
    printf ' %q' "${DOCKER_ARGS[@]}"
    printf '\n'
}

configure_claude_desktop() {
    local config_dir="$HOME/Library/Application Support/Claude"
    local config_file="$config_dir/claude_desktop_config.json"

    if [[ $DRY_RUN -eq 1 ]]; then
        echo "(dry run) would merge an 'mcpServers.jot' entry into $config_file"
        return
    fi

    if ! command -v python3 >/dev/null 2>&1; then
        echo "python3 not found; paste this into $config_file under \"mcpServers\" manually:"
        print_snippet
        return
    fi

    mkdir -p "$config_dir"
    python3 - "$config_file" "$DB_DIR" "$DB_FILE_NAME" "$IMAGE_TAG" <<'PYEOF'
import json, sys, os

config_file, db_dir, db_file_name, image_tag = sys.argv[1:5]

if os.path.exists(config_file):
    with open(config_file) as f:
        config = json.load(f)
else:
    config = {}

config.setdefault("mcpServers", {})
config["mcpServers"]["jot"] = {
    "command": "docker",
    "args": [
        "run", "-i", "--rm",
        "-v", f"{db_dir}:/data",
        "-e", f"JOT_DB_PATH=/data/{db_file_name}",
        image_tag,
    ],
}

with open(config_file, "w") as f:
    json.dump(config, f, indent=2)
    f.write("\n")

print(f"Updated {config_file}")
PYEOF
    echo "Restart Claude Desktop for the change to take effect."
}

configure_claude_code() {
    if ! command -v claude >/dev/null 2>&1; then
        echo "claude CLI not found; run this yourself once it's installed:"
        echo "  claude mcp add jot --scope user -- docker ${DOCKER_ARGS[*]}"
        return
    fi

    if [[ $DRY_RUN -eq 1 ]]; then
        echo "(dry run) claude mcp add jot --scope user -- docker ${DOCKER_ARGS[*]}"
        return
    fi

    claude mcp add jot --scope user -- docker "${DOCKER_ARGS[@]}"
}

case "$CLIENT" in
    claude-desktop) configure_claude_desktop ;;
    claude-code) configure_claude_code ;;
    both) configure_claude_desktop; configure_claude_code ;;
    print) print_snippet ;;
    *) echo "Unknown --client value: $CLIENT" >&2; usage; exit 1 ;;
esac
