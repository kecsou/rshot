# Third-party software

## FFmpeg (bundled as `rshot-ffmpeg`)

rshot's installers include an unmodified static build of FFmpeg, used as a separate program for screen recording
and trimming. FFmpeg is licensed under the GNU GPL v3 (this build enables GPL components such as libx264).

- Linux build: BtbN FFmpeg-Builds, release branch 8.1, release `autobuild-2026-08-31-13-27`
  (`ffmpeg-n8.1.2-50-g1a748fe2cd-linux64-gpl-8.1.tar.xz`) — https://github.com/BtbN/FFmpeg-Builds
- FFmpeg source: https://ffmpeg.org/download.html (commit `1a748fe2cd` on the `release/8.1` branch, 50 commits after
  tag `n8.1.2`: https://github.com/FFmpeg/FFmpeg/commit/1a748fe2cd43e3ead22fafb1b5b7d77f153898a8), build scripts
  at the commit that made this release:
  https://github.com/BtbN/FFmpeg-Builds/tree/8267213e26c1031621e6e1210fe3aa4867214f6a
- License text: `FFMPEG-LICENSE.txt` (the build's `LICENSE.txt`), installed next to this file.
