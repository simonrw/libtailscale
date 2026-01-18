#!/bin/bash
set -e

# Only run in Claude Code web environment
if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
    echo "Not in Claude Code web environment, skipping session setup"
    exit 0
fi

if ! command -v go >/dev/null; then
    echo "Go not available, exiting" >&2
    exit 1
fi

if ! command -v cargo >/dev/null; then
    echo "Rust not available, exiting" >&2
    exit 1
fi

if ! command -v make >/dev/null; then
    echo "Make not available, exiting" >&2
    exit 1
fi

# build Go module
make build
