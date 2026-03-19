#!/bin/bash
# Author: aav
# --------------------------------------------------
# Description:
#   Runs doc tests for the current workspace
# --------------------------------------------------
# Usage:
#   `bash scripts/test-doc.sh [--json|-j]`
# --------------------------------------------------
# * --json (optional): outputs JSON format to be dumped
#   into a JSON file
# --------------------------------------------------

# --------------------------------------------------
# parse cli args
# --------------------------------------------------
JSON_OUTPUT=false
while [[ $# -gt 0 ]]; do
    case "$1" in
    -j | --json)
        JSON_OUTPUT=true
        shift
        ;;
    *)
        echo "Unknown argument: $1"
        echo "Usage: bash test-doc.sh [--json|-j]"
        exit 1
        ;;
    esac
done

# --------------------------------------------------
# return
# --------------------------------------------------
if [ "$JSON_OUTPUT" = true ]; then
    cargo +nightly test --doc --workspace --features "doc-tests" -- -Z unstable-options --report-time --format json
else
    cargo test --doc --workspace --features "doc-tests"
fi
