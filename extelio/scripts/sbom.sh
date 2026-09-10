#!/usr/bin/env bash
# Erzeugt SBOMs im CycloneDX-Format (Kapitel 16, 22.1).
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p dist

echo "==> Rust-Abhaengigkeiten"
if ! cargo cyclonedx --version >/dev/null 2>&1; then
  cargo install cargo-cyclonedx --locked
fi
cargo cyclonedx --format json --spec-version 1.5
find . -name 'extelio*.cdx.json' -maxdepth 2 -exec mv {} dist/sbom-rust.cdx.json \;

echo "==> Frontend-Abhaengigkeiten"
cd src/web
npx --yes @cyclonedx/cyclonedx-npm --output-format json --output-file ../../dist/sbom-web.cdx.json

echo "SBOMs liegen unter dist/."
