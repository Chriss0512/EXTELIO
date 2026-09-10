#!/usr/bin/env bash
# Prueft, dass die gebaute Version zu build-lock.json passt (Kapitel 22.1).
set -euo pipefail
cd "$(dirname "$0")/.."

lock="build-lock.json"
fail=0

check() {
  local label="$1" expected="$2" actual="$3"
  if [ "$expected" != "$actual" ]; then
    echo "ABWEICHUNG $label: erwartet $expected, gefunden $actual" >&2
    fail=1
  else
    echo "ok $label: $actual"
  fi
}

expected_rust=$(grep -o '"rust": *"[^"]*"' "$lock" | cut -d'"' -f4)
actual_rust=$(rustc --version | awk '{print $2}')
check "Rust" "$expected_rust" "$actual_rust"

expected_version=$(grep -o '"version": *"[^"]*"' "$lock" | head -1 | cut -d'"' -f4)
actual_version=$(grep -m1 '^version' config.yaml | cut -d'"' -f2)
check "Add-on-Version" "$expected_version" "$actual_version"

cargo_version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
check "Cargo-Version" "$expected_version" "$cargo_version"

exit "$fail"
