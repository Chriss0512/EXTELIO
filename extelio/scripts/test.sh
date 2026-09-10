#!/usr/bin/env bash
# Vollstaendiger Testlauf: Backend, Frontend, Formatierung, Lint.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Rust: Formatierung"
cargo fmt --all -- --check

echo "==> Rust: Lint"
cargo clippy --all-targets -- -D warnings

echo "==> Rust: Tests"
cargo test --all-targets

echo "==> Frontend: Typpruefung und Build"
cd src/web
npm ci --no-audit --no-fund
npm run build

echo "==> Content Security Policy: keine Inline-Styles oder Inline-Skripte"
if grep -q 'style="' dist/index.html; then
  echo "FEHLER: Inline-Style im Build gefunden" >&2
  exit 1
fi
if grep -qE '<script(?![^>]*src=)' dist/index.html; then
  echo "FEHLER: Inline-Skript im Build gefunden" >&2
  exit 1
fi

echo "Alle Pruefungen bestanden."
