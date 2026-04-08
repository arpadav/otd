#!/bin/bash
# Dev server: builds tailwind CSS then runs dx serve (hot-reload)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# defaults
# PORT=15204
ADDR=0.0.0.0
# ADDR="127.0.0.1"

while [[ $# -gt 0 ]]; do
    case $1 in
        -a | --addr)
            ADDR="$2"
            shift 2
            ;;
        *)
            echo "Usage: $0 [-p|--port <NUM>] [-a|--addr <ADDR>]"
            exit 1
            ;;
    esac
done

# generate tailwind CSS
tailwindcss -i "$ROOT/input.css" -o "$ROOT/assets/tailwind.css"

# run dioxus dev server
dx serve --addr "$ADDR"
