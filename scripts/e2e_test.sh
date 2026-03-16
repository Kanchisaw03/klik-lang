#!/usr/bin/env bash
set -euo pipefail

cargo build --workspace

# Preferred command flow from project requirements.
if command -v klik >/dev/null 2>&1; then
  klik build examples/hello.klik
elif [[ -x "./target/debug/klik" ]]; then
  ./target/debug/klik build examples/hello.klik
elif [[ -f "./target/debug/klik.exe" ]]; then
  ./target/debug/klik.exe build examples/hello.klik
else
  echo "Could not find klik binary in PATH or target/debug"
  exit 1
fi

if [[ -x "./hello" ]]; then
  output="$(./hello)"
elif [[ -f "./hello.exe" ]]; then
  output="$(./hello.exe)"
else
  echo "Expected output binary ./hello or ./hello.exe was not produced"
  exit 1
fi

echo "$output"

if [[ "$output" != *"Hello KLIK"* ]]; then
  echo "Unexpected output: $output"
  exit 1
fi

echo "E2E test passed"
