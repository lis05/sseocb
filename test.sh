#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

export SSEOCB_G_VPBASE="${SSEOCB_G_VPBASE:-0xFFF00000}"
export SSEOCB_G_VPSIZE="${SSEOCB_G_VPSIZE:-0x00100000}"
export SSEOCB_I_VPBASE="${SSEOCB_I_VPBASE:-0x00100000}"
export SSEOCB_I_VPSIZE="${SSEOCB_I_VPSIZE:-0x00100000}"

echo "--> Checking formatting..."
cargo fmt --check

echo "--> Checking clippy..."
cargo clippy --workspace --exclude int --all-targets -- -D warnings

echo "--> Running unit tests..."
cargo test --workspace --exclude int

echo "--> Running bare-metal C E2E tests..."
bash scripts/run_tests.sh

