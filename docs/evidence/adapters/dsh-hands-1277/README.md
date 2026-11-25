# The composed dump of a real boxed dsh seat (#1277)

The controller's capture of 2026-11-24, on a Linux host with dsh
0.2.2-rc.1. An engine built from `2811aede` launched a gate seat on
`dashscope/qwen4-max` whose agent declares workspace hands. A recording
shim stood in front of `dsh`; it ran the real dsh with the driver's own
arguments, working directory and environment, and kept what went in and
what came out. The driver ran `dsh --version`, then
`dsh --profile headless --patch <seat-overlay.yml> --dump-config`, which
exited 0 with nothing on stderr.

| File | What it is |
| --- | --- |
| `seat-overlay.yml` | The overlay the driver wrote for the seat, byte for byte. |
| `dump-folded-json.txt` | What dsh printed for it: 100 rows. The driver's reader answers `Unreadable`. |
| `overlay-base64url.yml` | The same overlay with the values of `--reach` and `--spec` replaced by their base64url encodings, without padding. |
| `dump-base64url.txt` | What dsh printed for that overlay, in the same environment. |
| `dump-file-paths.txt` | The same, with the two values replaced by two long absolute paths. |
| `dump-short-json.txt` | The same, with the two values replaced by JSON documents of 13 and 36 characters. |

What the reader (`dsh_hands::composition::rows`) made of each, in a unit
test the controller ran and did not keep:

| Dump | Rows read | Lines the line rule refuses on their own |
| --- | --- | --- |
| `dump-folded-json.txt` | none: `Unreadable` | 455 and 458 |
| `dump-base64url.txt` | 100 | none |
| `dump-file-paths.txt` | 100 | none |
| `dump-short-json.txt` | 100 | none |

Lines 455 and 458 of the first dump are the content lines of the two
folded block scalars dsh writes for the long JSON values of `--reach`
and `--spec`. With only those two lines' text replaced by a plain word,
the reader reads 100 rows; with either left as captured, it refuses.

The capture was checked to hold no credential's value before it was
read. The paths are the capturing host's.
