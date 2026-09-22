#!/bin/sh
# Type-checks (clippy, no linking) rshot for Windows and macOS from Linux, so cfg/compile errors
# surface before CI. Needs: rustup targets x86_64-pc-windows-msvc + aarch64-apple-darwin, clang, llvm-rc.
set -eu
cd "$(dirname "$0")/../src-tauri"
bin=$(mktemp -d)
trap 'rm -rf "$bin"' EXIT
ln -s "$(command -v llvm-rc || command -v llvm-rc-18)" "$bin/llvm-rc"
export PATH="$bin:$PATH" CC_aarch64_apple_darwin=clang CC_x86_64_apple_darwin=clang
# No macOS SDK here: DOCS_RS makes mac-notification-sys and objc2-exception-helper skip their
# Objective-C builds (Cocoa headers); nothing is linked, so the type-check is unaffected.
export DOCS_RS=1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/cross}"
for t in x86_64-pc-windows-msvc aarch64-apple-darwin; do
  rustup target add "$t" >/dev/null 2>&1 || true
  cargo clippy --target "$t" --all-targets -- -D warnings
done
echo "cross-check OK"
