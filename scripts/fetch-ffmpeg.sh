#!/bin/sh
# Fetches the static ffmpeg rshot bundles as a Tauri sidecar:
#   src-tauri/binaries/rshot-ffmpeg-<rust target triple>
# Linux x86_64: BtbN FFmpeg-Builds, 8.1 release branch, GPL static (x11grab, pulse, libx264, aac).
# Pinned to a month-end autobuild: BtbN prunes the daily ones after ~2 weeks and rebuilds `latest`
# daily, but keeps each month's last release. To bump, pick a newer month-end `autobuild-*` tag
# and take the tarball's hash from that release's checksums.sha256.
set -eu
cd "$(dirname "$0")/../src-tauri"
mkdir -p binaries
triple=$(rustc -vV | sed -n 's/^host: //p')
out="binaries/rshot-ffmpeg-$triple"
if [ -x "$out" ]; then echo "$out already present"; exit 0; fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
case "$triple" in
  x86_64-unknown-linux-gnu)
    url=https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-31-13-27/ffmpeg-n8.1.2-50-g1a748fe2cd-linux64-gpl-8.1.tar.xz
    sha256=c733b4b2951e5957e15505f788b2c65a7a41b6da4b289e295852cc38079b4d2b
    ;;
  *)
    echo "no bundled ffmpeg for $triple yet" >&2
    exit 1
    ;;
esac
curl -fsSL -o "$tmp/ffmpeg.tar.xz" "$url"
echo "$sha256  $tmp/ffmpeg.tar.xz" | sha256sum -c --quiet
tar -xJf "$tmp/ffmpeg.tar.xz" -C "$tmp"
bin=$(echo "$tmp"/ffmpeg-*/bin/ffmpeg)
# Check before installing, so a bad build never sits at $out looking "already present".
"$bin" -hide_banner -devices 2>/dev/null | grep -q x11grab || { echo "sidecar lacks x11grab" >&2; exit 1; }
"$bin" -hide_banner -devices 2>/dev/null | grep -q pulse || { echo "sidecar lacks pulse" >&2; exit 1; }
"$bin" -hide_banner -encoders 2>/dev/null | grep -q libx264 || { echo "sidecar lacks libx264" >&2; exit 1; }
cp "$bin" "$out.part"
chmod +x "$out.part"
mv "$out.part" "$out"
echo "fetched $out ($("$out" -version | head -1))"
