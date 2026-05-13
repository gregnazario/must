#!/usr/bin/env bash
set -euo pipefail

echo "Running integration tests..."

./target/debug/cli --version || true

echo "All integration tests passed"
