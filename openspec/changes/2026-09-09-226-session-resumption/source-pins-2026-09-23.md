# Source pins for the Apple lookup and the env dispatch, 2026-09-23

Acceptance-ledger entry 13. This file pins the sources that the ported
Apple lookup (`apple_walk`, `step`'s Apple arm, `DARWIN_PATH_MAX`,
`APPLE_DEFAULT_PATH`) and the env dispatch rule (`env_program`,
`env_dispatch`) in `crates/brokkr-protocol/src/adapters/composite.rs`
cite. Each source is pinned to an immutable revision, with the SHA-256 of
the file's bytes. Every cited range was re-read against the port. It
closes the source-pin predicates of N1 (`tasks.md` 8.8.1.1, 1248–1250) and
N3 (8.8.2.1, 1284–1286), and N4's applicable source-pin cells (8.8.2.2,
1304–1305). The native macOS execution that those clauses also name is a
separate carry-over, and it is **still pending the macOS leg**.

## How the bytes were obtained

No seat of this change can reach the network. The host (the operator's
machine) retrieved each file on 2026-09-23 through the GitHub API
(`Accept: application/vnd.github.raw`) at the named tag. The commit is the
tag's commit as the API resolved it. The Ubuntu files came from Launchpad
source publication 17775740. The host's retrieval notes are in the
scratch folder `.forge/tasks/sources-entry13/MANIFEST.md`, which is not
committed. The implement seat of run
`issue-226-acceptance-ledger-entr-a5dcdf46` recomputed all sixteen digests
below locally, and each one matched. The earlier visits (`-61b7a860` and
`-ae6c17cc`) had matched the files they held. That is a local check of the host's bytes, not an independent
network verification. Line counts are `wc -l`. The first manifest table's
line column counted newline bytes and is not used.

## The pins

| repository | path | tag / version | commit | sha256 | lines |
|---|---|---|---|---|---|
| apple-oss-distributions/Libc | `gen/FreeBSD/exec.c` | Libc-1752.120.2 | `4e34d0559e3a1b081afeb8604d9e204a1f31321d` | `2a63c49bcb2891adfc824a00196f0e28d4a2549bd0bc7b3e8c3fe62a2864b527` | 328 |
| apple-oss-distributions/Libc | `sys/posix_spawn.c` | Libc-1752.120.2 | `4e34d0559e3a1b081afeb8604d9e204a1f31321d` | `24afc97cd7f03dbf0dd3d790844ebfe06f035e4fcc7486213432a88c9beb97ac` | 207 |
| apple-oss-distributions/Libc | `include/paths.h` | Libc-1752.120.2 | `4e34d0559e3a1b081afeb8604d9e204a1f31321d` | `96603992a31cd9bd9f5a0721d714d1aec4fff18131fbbe8204d4831b61cb75ff` | 101 |
| apple-oss-distributions/xnu | `bsd/sys/syslimits.h` | xnu-12377.121.6 | `ac9718fb1af618d5ce8678d0dc6e8a58f252216f` | `c82fe60eac5d7864220e1468e6b75740b07d2ad6d18fe923b495059e48c2f100` | 142 |
| apple-oss-distributions/xnu | `bsd/sys/param.h` | xnu-12377.121.6 | `ac9718fb1af618d5ce8678d0dc6e8a58f252216f` | `ee6024c8fee1eb9bcd280203532cba183fb015c5549eebcd8f45f4667529ad4a` | 245 |
| apple-oss-distributions/shell_cmds | `env/env.c` | shell_cmds-329 | `298787009e5432c5e4c378a077f98267077e3495` | `ce3d7648be3a82e0cf5f5cf5f75c1605027871781e3b1465b41492571a68f723` | 247 |
| coreutils/coreutils | `src/env.c` | v9.12 | `c0f8514d989184921d9b12a4d103a7b23abc5af8` | `b63cd812a3327c18e035c8cce2ab5c3d0ef67841543bc901df382f0a91ce8611` | 1251 |
| coreutils/coreutils | `src/coreutils.c` | v9.12 | `c0f8514d989184921d9b12a4d103a7b23abc5af8` | `5ce2971e19723536d308de7c30081f6436e320c3ab42bcc1ff1380d67e573300` | 206 |
| coreutils/gnulib | `lib/progname.c` | v1.0 | `d4ec02b3cc70cddaaa5183cc5a45814e0afb2292` | `1327584fbda2556b0bce306b0cff6eeaeded96ba93eded80d2dfad9930788654` | 92 |
| uutils/coreutils | `src/bin/coreutils.rs` | 0.12.0 | `dc1efd89948a9ca4c78c3a4b9a6ac891019a8c69` | `2a2c3af8d5174e83f85275ac425e3c8503b328139a5f7d2ad570886ca5b2cbca` | 143 |
| uutils/coreutils | `src/common/validation.rs` | 0.12.0 | `dc1efd89948a9ca4c78c3a4b9a6ac891019a8c69` | `15d99c4946c97ae9fdc4c6f7dbfb32f19e3965db35282467ec772e16eebb13a6` | 183 |
| uutils/coreutils | `src/bin/coreutils.rs` | 0.2.2 | `3a07ffc5a9bd4c283e75afa548ba1f1957bad242` | `6fba73d8db7c8eaa7aae516ab74b91102199952899ba04bbe83af432f7f609b6` | 360 |
| Ubuntu rust-coreutils | `debian/patches/require-utility-to-be-invoked-at-matching-path.patch` | 0.2.2-0ubuntu2.1 | Launchpad sourcepub 17775740 | `9b669ebb3b8e0b6a4420bdbdc60fb15ac51c68f4acef8b27e2a67b7e37eeb021` | 101 |
| Ubuntu rust-coreutils | `.dsc` | 0.2.2-0ubuntu2.1 | Launchpad sourcepub 17775740 | `c5b391c789e4189b6d4838e869c9461750aca1af2532f4b6302818b2356231f4` | 49 |
| mirror/busybox | `libbb/appletlib.c` | 1_36_1 | `1a64f6a20aaf6ea4dbba68bbfa8cc1ab7e5c57c4` | `d155776560dbe2d931cbed99d6aaf68bb877a624f4405f0f594d3c424d3fd853` | 1131 |
| torvalds/linux | `fs/binfmt_script.c` | v7.2 | `8d3ae59288f1e7d58d76558a6ee96d533bc5019f` | `81b0293a89908d56575f2b00b3550f927eb80714ef540f9634272bce56f9c656` | 159 |

The tags are the highest non-prerelease tag each repository offered on the
retrieval date. Linux is the latest stable release, and busybox's mirror has
no tag above `1_36_1`. The code cited Apple's env as `usr.bin/env/env.c`,
and proposal.md 3842 linked `shell_cmds` `env/env.c`. At `shell_cmds-329`
only **`env/env.c`** exists (the other path returned not found), so that is
the path pinned. glibc's `posix/execvpe.c` is cited to the fixed 2.42
release and is outside this entry.

## Re-read against the port

Each ported block's cited range was re-read at the pin above. "Agrees"
means the port does what the pinned lines do.

### Apple lookup (Libc-1752.120.2)

| ported block | pinned lines | reading | result |
|---|---|---|---|
| Walk entry, `execvp` form | exec.c `execvp` 148–152 → `_execvpe` 318–328 → static `execvPe` 154–310; `execvP` 312–316 wraps the same walk | `execvp` gets `PATH` from `getenv`, or `_PATH_DEFPATH` when that is null, then walks in `execvPe`. The port calls this walk `execvP`, the name its stderr warning prints (`execvPe_err_preamble`, 55). | agrees; comments now name the chain |
| Walk entry, `posix_spawnp` form | posix_spawn.c 69–207 | `env_path = getenv("PATH")`, or `_PATH_DEFPATH` (92–93). | agrees |
| Direct name | exec.c 167–176, posix_spawn.c 85–90 | `strchr(name, '/')` jumps to `retry` with no search. The quote is "If it's an absolute or relative path name, it's easy." | agrees; the quote's case was corrected |
| Empty name | exec.c 179–183, posix_spawn.c 97–99 | ENOENT before searching. The port refuses first (`refuse_unspellable`). | consistent |
| Tokenising | exec.c 187–208, posix_spawn.c 103–124 | `np = strchrnul(op, ':')`; `if (np == op) { p = "."; lp = 1; } else { p = op; lp = np - op; }`; `op = NULL` at the end, else `np + 1`. A leading, trailing or doubled colon, and an empty `PATH`, each give one `.` token, which is exactly what `split(':')` gives. | **citation corrected**: the port quoted `strsep(&cur, ":")` and `lp = strlen(p)`, which are not the pinned bytes. The token sequence is unchanged. |
| Candidate build | exec.c 223–226, posix_spawn.c 135–138 | `bcopy(p, buf, lp); buf[lp] = '/'; bcopy(name, buf + lp + 1, ln); buf[lp + ln + 1] = '\0';` | agrees |
| Overflow | exec.c 215–222, posix_spawn.c 131–134 | `lp + ln + 2 > sizeof(buf)`. `execvPe` writes `execvP: <p>: path too long` and `continue`s. `posix_spawnp` sets `err = ENAMETOOLONG; goto done;`. | agrees (`apple_walk`) |
| Buffer size | exec.c 161 `char buf[MAXPATHLEN]` (via `<sys/param.h>`, 39); posix_spawn.c 76 `char path_buf[PATH_MAX]` (via `<limits.h>`, 33) | xnu `bsd/sys/syslimits.h` 111 `#define PATH_MAX 1024`. `bsd/sys/param.h` 206 `#define MAXPATHLEN PATH_MAX`, and it includes syslimits.h at 93. | agrees (`DARWIN_PATH_MAX = 1024`). Libc's own `<limits.h>` → `<sys/syslimits.h>` step was not supplied; posix_spawn.c's comment at 33 is its only support. |
| Continue arms | posix_spawn.c 146–150; exec.c 232–235 and 266–267 | ELOOP, ENAMETOOLONG, ENOENT and ENOTDIR `break` to the next entry. | agrees (`step`) |
| EACCES / `default` | posix_spawn.c 178–193 (stat at 186–187); exec.c 273–289 (stat at 282–283) | `if (stat(bp, &sb) != 0) break;` comes before `eacces = 1; continue;`, so an unstattable candidate is walked past **unremembered**. | agrees since `1d2763cf` (entry 13-fix, finding P1). The first visit found the divergence, and the second and third visits confirmed the repair. |
| Exhaustion | posix_spawn.c 195–204, exec.c 293–306 | EACCES if one was remembered, otherwise ENOENT for a search that ran to the end. | agrees |
| Unported arms | posix_spawn.c 142–145 and 151–177; exec.c 230–231, 236–265, 268–272, and `default`'s stattable non-EACCES stop | E2BIG, ENOMEM and ETXTBSY stop, and ENOEXEC re-runs under `_PATH_BSHELL`. The port refuses every one of these as unestablished. | a conservative limitation, not a misreading (see below) |
| Default search | Libc `include/paths.h` 65 `#define _PATH_DEFPATH "/usr/bin:/bin"`; 67 `_PATH_STDPATH` | `_execvpe` (exec.c 324–325) and `posix_spawnp` (92–93) read `_PATH_DEFPATH`. `execvP` takes its path from its caller. | agrees (`APPLE_DEFAULT_PATH`). **Citation corrected**: the comment gave the attribution to `execvP` and cited "line 63". |
| D10's moving-`main` ranges | exec.c 178–218 and 262–297; posix_spawn.c 97–143 and 170–195 | At the pin, exec.c's walk is 177–309 (loop 187–291, switch 229–290, exit 293–309), and 262 falls inside the ENOEXEC arm. posix_spawn.c's walk is 95–206 (loop 103–194, switch 141–193, exit 195–206). | **citation corrected** in `apple_walk`'s comment. design.md 2954–2957 still carries the moving links, which is recorded for entry 22. |

### The env dispatch

| ported claim | pinned lines | reading | result |
|---|---|---|---|
| `argv[0]` is the interpreter as spelled; one argument after it | linux v7.2 `fs/binfmt_script.c` `load_script` 34–138: trailing spaces and tabs trimmed at 72–74, `i_arg` at 81–85, the argument at 113–119, `argv[0] = i_name` at 121 | The optional argument is one string from the first non-space/tab after the name to the trimmed end. | agrees (`env_program`, the Linux argument rule) |
| GNU `env` reads `argv[0]` for diagnostics only | coreutils v9.12 `src/env.c` `main` 1021–1251: `set_program_name (argv[0])` 1035, `execvp (program, …)` 1242; gnulib v1.0 `lib/progname.c` 39–92 | `set_program_name` only strips a libtool `/.libs/` or `lt-` prefix and never dispatches. | agrees |
| Apple `env` reads no name | shell_cmds-329 `env/env.c` `main` 62–234, `execvp(*argv, argv)` 220 | `main` never dispatches on `argv[0]`. The program runs through `execvp`, the `execvPe` walk. | agrees; **path corrected** to `env/env.c` |
| GNU single-binary `coreutils` dispatches on the basename | coreutils v9.12 `src/coreutils.c` `main` 130–206: `last_component (argv[0])` 132, `launch_program` 93–127, `unknown program` 182–187 | Under `uu_env` there is no program. | agrees |
| busybox dispatches on the basename | busybox 1_36_1 `libbb/appletlib.c` `main` 1032–1131: strips a leading `-` and takes `bb_basename` at 1107–1110; `run_applet_and_exit` 977–997 gives `applet not found`, exit 127 | Under `uu_env` there is no applet. | agrees |
| uutils runs `env` under `env` and `uu_env` | uutils 0.12.0 `src/bin/coreutils.rs` `main` 52–143: the name is `validation::name(validation::binary_path(args))` (56–57), and the longest utility the name `ends_with` wins (62–67); 0.2.2 `main` 97–200: exact name (110), then `find_prefixed_util` (71–81, 118) | Both run `env` under `uu_env`. | agrees |
| **"uutils … refuses a renaming symlink" (`/proc/self/exe`, `Security violation`)** | Ubuntu `require-utility-to-be-invoked-at-matching-path.patch` (Julian Andres Klode, Canonical, 2025-07-21, "such that we get the required semantics for AppArmor profiles"), sixth in `debian/patches/series` of rust-coreutils 0.2.2-0ubuntu2.1. It inserts into 0.2.2 `main`, after 108 and before the exact-name dispatch (109–113), a `read_link("/proc/self/exe")` file-name comparison that prints `Security violation: Requested utility `…` does not match executable name:` and exits 1. | Upstream uutils has no such check at 0.2.2 or at 0.12.0. 0.12.0's `binary_path` (validation.rs 114–145) reads `AT_EXECFN`, not `/proc/self/exe`, and returns `argv[0]` for a `#!` launch (130–141). Its only refusal is `unknown program` (53–60). | **attribution corrected** (finding P2, below). The behaviour is kept. |

## Findings

**P1: the Apple arm remembered every EACCES.** Found by the first visit
(run `issue-226-acceptance-ledger-entr-61b7a860`), which stopped. It was
repaired by entry 13-fix at `1d2763cf` and `4ce6eba2`. At this pin the
repaired `step` matches posix_spawn.c 178–193 and exec.c 273–289. It is not
open.

**P2: the refusing build was misattributed. This is not a production
finding.** Found by the second visit (run `-ae6c17cc`). The port gave the
`/proc/self/exe` name check to "uutils". It is Ubuntu's patch on uutils
0.2.2. This host's `/usr/bin/env` resolves to
`/usr/lib/cargo/bin/coreutils/env` from package `rust-coreutils`
`0.2.2-0ubuntu2.1` and reports `uu_env (uutils coreutils) 0.2.2`, according
to the host's MANIFEST. Upstream uutils 0.2.2 and 0.12.0, busybox 1_36_1 and
GNU v9.12 each run a renamed `env` (a symlink named `env` to a file named
`uu_env`). The patched build refuses it. So the implementations genuinely
disagree, and `env_dispatch`'s refusal is the fail-closed reading that
decision 0004 requires. By the operator's resolution the refusal stays, and
only the attribution moved. The corrected claims are in `composite.rs`
(`env_program`'s and `env_dispatch`'s comments) and in `composite/tests.rs`
(the native-outcome comments of
`the_selected_invocation_is_not_replaced_by_its_canonical_target` and the
R1 renamed-symlink cell). design.md 2996–3006 and 3036–3045 carry the same
attribution and sit outside this entry's files, so they are recorded for
entry 22.

The only caveat: 0.12.0's `src/uu/env/src/env.rs` and the uucore entry
macro were not supplied. So these files cannot exclude a refusal made
elsewhere in 0.12.0. Neither the kept rule nor the corrected attribution
depends on it, because the refusal stands on the patched build's measured
refusal.

## What this entry changed, and what it did not

Only comments changed. In `composite.rs` these were `Library::Apple`, `DARWIN_PATH_MAX`,
`Operation`, `apple_walk` and its two tokenising comments, the direct-name
comment in `lookup_failure`, `refuse_working_directory`,
`APPLE_DEFAULT_PATH`, the `binfmt_script` paragraph, `env_program`,
`env_dispatch`. In `composite/tests.rs` they were the default-search, working-directory,
selected-invocation, renamed-symlink, step-table, walk-table,
direct-ENAMETOOLONG, ELOOP and construction-stop comments. In
`composite/tests/native_matrix.rs` they were `ConstructionStop`,
`apple_overflows` and the ELOOP/ENAMETOOLONG wording note. No executable
byte, message string or assertion moved. The runtime refusal "that arm of
Apple's posix_spawnp switch is not pinned by this resolver" still reads
"not pinned". Every arm is now read at a pin, but the ones it names are not
ported. Rewording that string would change asserted values, so it is left
for a later unit to decide.

Pending, and not supplied by this entry:
- native macOS execution of every Apple cell, including 13-fix's two
  sealed-directory controls (the macOS leg);
- Libc's `<limits.h>` → `<sys/syslimits.h>` include step;
- `gen/FreeBSD/sysctl.c`'s `USER_CS_PATH`, which the `APPLE_DEFAULT_PATH`
  comment names for the wider `confstr` value;
- design.md's corrections (D10's moving links 2954–2957, `usr.bin/env/env.c`
  at 2988, and the uutils attribution at 2996–3006 and 3036–3045), for
  entry 22.
