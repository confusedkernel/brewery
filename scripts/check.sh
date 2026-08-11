#!/usr/bin/env bash
# Runs the same checks as .github/workflows/ci.yml, so CI shouldn't surprise you.
#
# Always goes through `rustup run stable` rather than whatever `cargo` is first
# on PATH: a Homebrew-installed cargo can lag CI's stable by several releases and
# miss lints that only the newer clippy knows about.
set -euo pipefail

cd "$(dirname "$0")/.."

TOOLCHAIN="${TOOLCHAIN:-stable}"
CROSS_TARGET="${CROSS_TARGET:-x86_64-unknown-linux-gnu}"

# Put the toolchain's own bin dir first rather than using `rustup run`, which
# only resolves the command it launches: cargo would still pick up `rustc` from
# PATH. If that's a Homebrew rustc (not a rustup shim, so RUSTUP_TOOLCHAIN means
# nothing to it), you get a stale compiler and a sysroot without $CROSS_TARGET.
PATH="$(dirname "$(rustup which --toolchain "$TOOLCHAIN" cargo)"):$PATH"
export PATH

run() {
  echo "==> $*"
  "$@"
}

run cargo fmt --all -- --check
run cargo clippy --all-targets --all-features -- -D warnings

# theme.rs splits on #[cfg(target_os = "macos")], so the host target only lints
# one of the two branches. Check the other one too. Clippy never links, so the
# std for the target is all that's needed -- no cross-linker.
if rustup target list --installed --toolchain "$TOOLCHAIN" | grep -qx "$CROSS_TARGET"; then
  run cargo clippy --target "$CROSS_TARGET" --all-targets --all-features -- -D warnings
else
  echo "==> skipping $CROSS_TARGET clippy: rustup target add $CROSS_TARGET"
fi

run cargo test --all-features --locked
