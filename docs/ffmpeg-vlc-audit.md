# FFmpeg VLC compiler-warning audit

Audited November 17, 2026, for the Linux ARM64 Flatpak build. This is a review of
the reported array-bounds warning, not a security audit of FFmpeg or a Frame test.

## Source and warning

The source is the checksum-pinned **FFmpeg 8.1.3** archive in `xr/toolchain.json`:
[upstream archive](https://ffmpeg.org/releases/ffmpeg-8.1.3.tar.xz), SHA256
`7138d28c96d9d3e3af4ee3d8cad72741f8ffb40da90c1112235dea3ecd3178a3`.
The downloaded archive was verified before inspection.

[CI job 134013303939](https://github.com/jlips24/XRbox/actions/runs/44075762930/job/134013303939)
reports potential indices 3, 4 and 5 into `info.val16` at `libavcodec/vlc.c:443`,
after recursive inlining of `add_level`. FFmpeg compilation completed.

## Bounds analysis

- `libavcodec/vlc.h` defines the element as a union of six 8-bit values or three
  16-bit values. These are alternative representations of the same six bytes.
- `ff_vlc_init_multi_from_lengths` supplies a Boolean 16-bit selector, derived
  from whether the symbol alphabet exceeds 256 entries (`vlc.c:564`).
- `vlc_multi_gen` starts `add_level` at level zero (`vlc.c:508–509`).
- `add_level` computes a limit of six or three symbols from that same selector
  (`vlc.c:429`). It recurses only when the next level is below this limit
  (`vlc.c:456–459`), passing the selector unchanged.
- Consequently the 16-bit write can reach only indices 0–2; the 8-bit write can
  reach indices 0–5. The reported 16-bit indices 3–5 are unreachable under these
  call invariants. This supports classifying this specific diagnostic as a
  compiler false positive. It does not establish correctness of unrelated accesses.

An archive search found callers of the multi-symbol initializer in Ut Video and
MagicYUV, not H.264. Our manifest disables other decoders; the job explicitly
lists only H.264 under enabled decoders. The common VLC translation unit still
compiles the generator, which explains why its warning appears.

## Sanitizer experiment and limits

A temporary harness compiled the unchanged upstream `add_level` function and its
upstream element types, with a memcpy equivalent of the eight-byte copy macro.
It exercised both symbol widths, complete codebooks with 1–3-bit symbols, and
lookup tables up to 14 bits. The harness checked the number of stored symbols
and reached both maximum legal depths.

```sh
clang -std=c11 -O2 -g -fsanitize=address,undefined -fno-omit-frame-pointer \
  .tools/ffmpeg-vlc-audit.c -o .tools/ffmpeg-vlc-audit
.tools/ffmpeg-vlc-audit
```

Observed: **78 cases passed**, reaching six 8-bit symbols and three 16-bit symbols,
with no AddressSanitizer or UndefinedBehaviorSanitizer finding. The generated
harness/binary are local ignored audit artifacts, not a fork of the dependency.
This ran with Apple Clang on the development Mac, not the SDK's ARM64 GCC. It
supports the bounds analysis; it is not exhaustive codec testing or reproduction
of the compiler diagnostic on the original compiler.

No FFmpeg source patch, dependency upgrade or warning suppression was introduced.
Revisit this assessment if the pinned source, recursion rules or enabled decoders
change. Keep compiler warnings visible in the next Linux build.
