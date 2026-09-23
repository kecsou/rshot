# Third-party software

rshot itself is MIT-licensed (`LICENSE`).

## FFmpeg (bundled as `rshot-ffmpeg`)

rshot's installers include an unmodified static build of FFmpeg, used as a separate program for screen recording
and trimming. These builds are licensed under the GNU GPL v3 (they enable GPL components such as libx264).

| Installer | Build | FFmpeg source |
|---|---|---|
| Linux (`.deb`, `.rpm`, AppImage) | BtbN FFmpeg-Builds, release `autobuild-2026-08-31-13-27`, `ffmpeg-n8.1.2-50-g1a748fe2cd-linux64-gpl-8.1.tar.xz` | commit `1a748fe2cd` |
| Windows | the same BtbN release, `ffmpeg-n8.1.2-50-g1a748fe2cd-win64-gpl-8.1.zip` | commit `1a748fe2cd` |
| macOS (universal) | Martin Riedl's release builds 9.0.2 for arm64 and x86_64 (https://ffmpeg.martin-riedl.de), joined with `lipo` | release 9.0.2 |

- FFmpeg source: https://ffmpeg.org/download.html.
  - Commit `1a748fe2cd` is on the `release/8.1` branch, 50 commits after tag `n8.1.2`:
    https://github.com/FFmpeg/FFmpeg/commit/1a748fe2cd43e3ead22fafb1b5b7d77f153898a8
  - 9.0.2: https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz
- Build scripts, which name every library linked into the builds and its version. x264 is commit `0480cb05` in both
  (Martin Riedl's script takes x264's `master`, which was that commit when 9.0.2 was built):
  - BtbN, at the commit that made this release:
    https://github.com/BtbN/FFmpeg-Builds/tree/8267213e26c1031621e6e1210fe3aa4867214f6a
  - Martin Riedl, at the commit that made 9.0.2:
    https://git.martin-riedl.de/ffmpeg/build-script/src/commit/6a611e19870e197bc37c6e4c7fccddebd3715466
- Each rshot release on GitHub also carries these sources: FFmpeg, x264 and both build-script trees.
- Licence text: `FFMPEG-LICENSE.txt` (the GPLv3: BtbN's `LICENSE.txt`, and FFmpeg's `COPYING.GPLv3` at tag `n9.0.2`
  for macOS, which is the same text). The licence notices of the libraries inside the builds are in
  `FFMPEG-NOTICES.txt`, made by `scripts/ffmpeg-notices.sh`.
- Where these files are installed: next to this file, in `/usr/lib/rshot/` on Linux, in rshot's install folder on
  Windows (`%LOCALAPPDATA%\rshot`), and in `rshot.app/Contents/Resources/` on macOS. In a checkout,
  `scripts/fetch-ffmpeg.sh` fetches the licence to `src-tauri/binaries/`.
