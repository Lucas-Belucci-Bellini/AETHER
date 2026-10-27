#!/usr/bin/env bash
# Local quality gate: the same checks a CI pipeline should run.
# Usage: bash scripts/verify.sh   (set MSRV_TOOLCHAIN to override, or SKIP_MSRV=1)
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$*"; }

step "format"
cargo fmt --all -- --check

step "clippy (all targets, all features, warnings are errors)"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "tests (workspace, all features, incl. doctests)"
cargo test --workspace --all-features

step "feature matrix for the facade"
for features in "" web tasks ai logging full; do
  echo "  liga --no-default-features ${features:+--features $features}"
  cargo check -q -p liga --no-default-features ${features:+--features "$features"}
done

step "core stays free of HTTP dependencies"
if cargo tree -p liga --no-default-features -e normal | grep -qE '\b(axum|hyper)\b'; then
  echo "liga without features pulls axum/hyper" >&2
  exit 1
fi

if [[ "${SKIP_MSRV:-0}" != "1" ]]; then
  toolchain="${MSRV_TOOLCHAIN:-1.94}"
  step "MSRV ($toolchain)"
  cargo "+$toolchain" check --workspace --all-features --all-targets
fi

step "all checks passed"
