#!/bin/sh
# Fetches the static ffmpeg rshot bundles as a Tauri sidecar, with its license:
#   src-tauri/binaries/rshot-ffmpeg-<rust target triple>[.exe]
#   src-tauri/binaries/FFMPEG-LICENSE.txt (the GPLv3 text; the same for every build)
# The triple is the host's, or $TARGET (release builds: TARGET=universal-apple-darwin fetches both
# macOS builds and lipo's them, keeping the per-arch files that tauri-build asks for).
#   Linux x86_64, Windows x64: BtbN FFmpeg-Builds, 8.1 release branch, GPL static.
#   macOS arm64, x86_64: Martin Riedl's static release builds (GPLv3, system frameworks only).
# Every download is pinned by sha256. BtbN prunes its daily autobuilds after ~2 weeks but keeps each
# month's last release for about 2 years, so its pins expire ~2 years after their date. To bump:
# pick a newer month-end `autobuild-*` tag and take the hashes from its checksums.sha256; for macOS,
# a newer build from https://ffmpeg.martin-riedl.de (its .sha256 links) and COPYING.GPLv3 at the
# matching FFmpeg tag. Set `build` to the `ffmpeg -version` string, then update THIRD_PARTY.md and
# the source archives in .github/workflows/release.yml, and rerun scripts/ffmpeg-notices.sh.
set -eu
self=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
cd "$(dirname "$0")/../src-tauri"
mkdir -p binaries
triple=${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}
ext=""
[ "${triple#*windows}" != "$triple" ] && ext=".exe"
out="binaries/rshot-ffmpeg-$triple$ext"
lic=binaries/FFMPEG-LICENSE.txt
btbn=https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-08-31-13-27
riedl=https://ffmpeg.martin-riedl.de/download/macos
licurl="" # BtbN archives carry LICENSE.txt; Martin Riedl's hold only the binary
# host: the `uname -s` `uname -m` pattern that can run the build, so its checks run there only.
case "$triple" in
  x86_64-unknown-linux-gnu)
    url=$btbn/ffmpeg-n8.1.2-50-g1a748fe2cd-linux64-gpl-8.1.tar.xz
    sha256=c733b4b2951e5957e15505f788b2c65a7a41b6da4b289e295852cc38079b4d2b
    build=n8.1.2-50-g1a748fe2cd-20260831
    host="Linux x86_64"
    devices="x11grab pulse"
    ;;
  x86_64-pc-windows-msvc)
    url=$btbn/ffmpeg-n8.1.2-50-g1a748fe2cd-win64-gpl-8.1.zip
    sha256=273abb45f3f9f76c303e35ff39f5bb6c23c163ae65f6244a32b7d4a7f6cf0616
    build=n8.1.2-50-g1a748fe2cd-20260831
    host="*_NT-* x86_64" # Git Bash (MINGW64_NT-…), MSYS, Cygwin
    devices="gdigrab dshow"
    ;;
  aarch64-apple-darwin | x86_64-apple-darwin)
    if [ "$triple" = aarch64-apple-darwin ]; then
      url=$riedl/arm64/1789931890_9.0.2/ffmpeg.zip
      sha256=c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924
      host="Darwin arm64"
    else
      url=$riedl/amd64/1789931006_9.0.2/ffmpeg.zip
      sha256=7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4
      host="Darwin x86_64"
    fi
    build=9.0.2-https://www.martin-riedl.de
    devices=avfoundation
    licurl=https://raw.githubusercontent.com/FFmpeg/FFmpeg/n9.0.2/COPYING.GPLv3
    licsha256=8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903
    ;;
  universal-apple-darwin)
    for t in aarch64-apple-darwin x86_64-apple-darwin; do TARGET=$t sh "$self"; done
    lipo -create -output "$out.part" binaries/rshot-ffmpeg-aarch64-apple-darwin binaries/rshot-ffmpeg-x86_64-apple-darwin
    mv "$out.part" "$out"
    echo "made $out"
    exit 0
    ;;
  *)
    echo "no bundled ffmpeg for $triple" >&2
    exit 1
    ;;
esac
# `$out.sha256` names the archive $out came from: a new pin refetches, on any build host.
if [ -x "$out" ] && [ -s "$lic" ] && [ "$(cat "$out.sha256" 2>/dev/null)" = "$sha256" ]; then
  echo "$out ($build) already present"
  exit 0
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
trap 'exit 1' INT TERM
# shasum ships with macOS and perl; sha256sum with coreutils (Linux, Git Bash).
check() {
  if command -v shasum >/dev/null; then shasum -a 256 -c; else sha256sum -c; fi
}
curl --retry 3 -fsSL -o "$tmp/archive" "$url"
echo "$sha256 *$tmp/archive" | check
mkdir "$tmp/x"
case "$url" in
  *.zip) unzip -q "$tmp/archive" -d "$tmp/x" ;;
  *) tar -xJf "$tmp/archive" -C "$tmp/x" ;;
esac
bin=$(find "$tmp/x" -type f -name "ffmpeg$ext")
if [ -n "$licurl" ]; then
  curl --retry 3 -fsSL -o "$tmp/LICENSE.txt" "$licurl"
  echo "$licsha256 *$tmp/LICENSE.txt" | check
else
  cp "$tmp"/x/ffmpeg-*/LICENSE.txt "$tmp/LICENSE.txt"
fi
# Check before installing, so a bad build never sits at $out looking "already present". Only where
# the build runs: a cross fetch (TARGET=…) trusts the pinned hash.
# shellcheck disable=SC2254 # $host is a pattern
case "$(uname -s) $(uname -m)" in
  $host)
    [ "$("$bin" -version | head -1 | cut -d' ' -f3)" = "$build" ] || { echo "sidecar isn't $build" >&2; exit 1; }
    for d in $devices; do
      "$bin" -hide_banner -devices 2>/dev/null | grep -q "$d" || { echo "sidecar lacks $d" >&2; exit 1; }
    done
    "$bin" -hide_banner -encoders 2>/dev/null | grep -q libx264 || { echo "sidecar lacks libx264" >&2; exit 1; }
    ;;
esac
mv "$tmp/LICENSE.txt" "$lic.part"
mv "$lic.part" "$lic"
cp "$bin" "$out.part"
chmod +x "$out.part"
mv "$out.part" "$out"
echo "$sha256" >"$out.sha256"
echo "fetched $out ($build)"
