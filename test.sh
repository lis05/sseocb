#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

echo "--> Checking formatting..."
cargo fmt --check

echo "--> Checking clippy..."
cargo clippy --workspace --all-targets -- -D warnings

echo "--> Running unit tests..."
cargo test --workspace

echo "--> Running bare-metal C E2E tests..."
bash scripts/run_tests.sh

