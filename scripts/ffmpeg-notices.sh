#!/bin/bash
# Regenerates FFMPEG-NOTICES.txt: the licence files of every library that BtbN's build scripts put
# into the Linux and Windows ffmpeg builds rshot bundles, each at the commit those scripts pin.
# Rerun it whenever fetch-ffmpeg.sh moves to a newer BtbN build (update `btbn` below first).
# Needs bash, git and curl.
set -euo pipefail
btbn=8267213e26c1031621e6e1210fe3aa4867214f6a # the FFmpeg-Builds commit behind autobuild-2026-08-31-13-27
out=$(cd "$(dirname "$0")/.." && pwd)/FFMPEG-NOTICES.txt
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl --retry 3 -fsSL "https://github.com/BtbN/FFmpeg-Builds/archive/$btbn.tar.gz" | tar -xz -C "$tmp"
cd "$tmp"/FFmpeg-Builds-*

# "<target> <name> <repo> <commit>" for each source the linux64/win64 GPL 8.1 builds use.
for target in linux64 win64; do
  for s in scripts.d/*.sh scripts.d/*/*.sh; do
    (
      set +euo pipefail # BtbN's scripts aren't written for strict mode
      source util/vars.sh "$target" gpl 8.1 >/dev/null
      source util/dl_functions.sh
      source "$s"
      ffbuild_enabled >/dev/null 2>&1 || exit 0
      name=$(basename "$s" .sh)
      name=${name#*-}
      [ -n "${SCRIPT_REPO:-}" ] && echo "$target $name $SCRIPT_REPO ${SCRIPT_COMMIT:-r${SCRIPT_REV:-}}"
      [ -n "${SCRIPT_REPO2:-}" ] && echo "$target $name $SCRIPT_REPO2 $SCRIPT_COMMIT2"
      [ -n "${SCRIPT_REPO3:-}" ] && echo "$target $name $SCRIPT_REPO3 $SCRIPT_COMMIT3"
      true
    )
  done
done | sort -k3,4 -k1,1 >"$tmp/sources"

{
  echo "Notices for the libraries inside rshot-ffmpeg"
  echo
  echo "rshot-ffmpeg on Linux and Windows is an unmodified FFmpeg build from BtbN FFmpeg-Builds"
  echo "(https://github.com/BtbN/FFmpeg-Builds/tree/$btbn). It statically links the libraries"
  echo "below; their licence files follow, verbatim, from each source at the pinned commit. The"
  echo "macOS build (Martin Riedl) links a subset of the same libraries, at the versions its build"
  echo "script pins, plus libklvanc (LGPL-2.1)."
  echo "See THIRD_PARTY.md for the builds and their source code."
  declare -A seen
  # One section per source; targets that share it are listed together.
  awk '{ k = $3 " " $4; t[k] = t[k] ? t[k] "," $1 : $1; n[k] = $2 } END { for (k in n) print n[k], k, t[k] }' \
    "$tmp/sources" | sort | while read -r name repo commit targets; do
    echo
    echo "================================================================================"
    echo "$name ($targets): $repo @ $commit"
    echo "================================================================================"
    case "$commit" in
      r*) echo "Subversion revision ${commit#r}: see the licence files in that checkout."; continue ;;
    esac
    rm -rf "$tmp/r"
    git init -q "$tmp/r"
    git -C "$tmp/r" fetch -q --depth 1 --filter=blob:none "$repo" "$commit" 2>/dev/null ||
      git -C "$tmp/r" fetch -q --depth 1 "$repo" "$commit"
    # Top-level licence files, and the files in a top-level LICENSES/ folder (REUSE layout).
    files=$(git -C "$tmp/r" ls-tree -r --name-only FETCH_HEAD |
      grep -iE '^((licen[cs]e|copying|copyright|notice)[^/]*|licen[cs]es/[^/]+)$' || true)
    [ -n "$files" ] || echo "No licence file at the top of the source tree: the licence is in the source files."
    for f in $files; do
      text=$(git -C "$tmp/r" show "FETCH_HEAD:$f")
      sum=$(sha256sum <<<"$text" | cut -d' ' -f1)
      echo
      echo "--- $f"
      if [ -n "${seen[$sum]:-}" ]; then
        echo "(the same text as ${seen[$sum]})"
      else
        seen[$sum]="$name/$f"
        echo "$text"
      fi
    done
  done
} >"$out.part"
mv "$out.part" "$out"
echo "wrote $out"
