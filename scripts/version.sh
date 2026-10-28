#!/usr/bin/env bash
# Prints the workspace version (Cargo.toml, [workspace.package]); the packages are named by it.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
awk -F'"' '/^version *=/ { print $2; exit }' "$root/Cargo.toml"
