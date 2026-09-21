# 0063 — Windows is not a host: Brokkr runs on Linux and macOS, and a Windows machine reaches it through WSL

Status: accepted (operator ruled in chat, 2026-09-21)
Date: 2026-09-21

## Context

Brokkr has carried native Windows as a third host since the first release:
a `test (windows-latest)` leg required on every pull request, a
`brokkr-windows-x86_64` release artifact, and a Scoop channel. What that
host can actually do is narrower than the badge suggests. Decision 0043
gives Linux a box and macOS the system's own sandbox, and says of the third:
"Windows has nothing comparable and gets none." Decision 0046 has Windows
compile every shipped bundle and refuse to run the boxed ones. The product's
point is boxed delivery; on native Windows that point is absent, and what
remains is unboxed and container-boundary work.

What the host costs is steadier than any single incident:

- **A required leg on every pull request**, and platform branches the exact
  coverage gate cannot cover from one machine — the `cfg!(windows)` line that
  had to be reworked into `#[cfg]` selection to make the gate reachable at all.
- **Native-evidence obligations in open changes.** Issue #226's composite
  identity owes a Windows port of the lookup oracle, `GetBinaryTypeW`
  evidence and a native matrix — none taken, and one task ledger recorded
  Windows executions that had never run until a council caught it.
- **A security hold.** #226's second `REVIEW-SECURITY-HOLD` came from a
  heuristic that existed only for Windows path thinking: a backslash in an
  override made it "a path", and a file literally named `C:\Tools\dsh.exe`
  in the working directory executed on Linux.
- **A rule with nowhere to stand.** The resolver's security property is
  *never the working directory, otherwise native*. Windows' native process
  search puts the current directory first, so on Windows that rule could
  only ever refuse.

Measured on main on 2026-09-21: about 35 Windows-conditional sites against
189 `cfg(unix)`; on the #226 branch, 16 inside a 4,297-line resolver. The
code is small. The standing obligations are not.

The operator's words, 2026-09-21: *"I don't want spending time and effort on
windows going forward, whoever needs windows can use WSL."*

## Rulings

1. **Supported hosts are Linux and macOS. Native Windows is not a host.**
   WSL2 is Linux and is supported as Linux; that is the road for anyone on a
   Windows machine. Nothing in Brokkr distinguishes WSL from any other Linux.

   **Enforcement binding:** `README.md`'s platforms badge and install table;
   `docs/guides/quickstart.md`.

2. **No Windows effort.** No new Windows-conditional code, test, fixture,
   evidence or review obligation is written, owed or accepted. A finding
   that exists only on native Windows is recorded as out of scope under this
   decision — it is not tracked debt, because debt implies an intent to pay.

   **Enforcement binding:** the review charters read this decision through
   the house rules; `docs/house-rules.md` names the supported hosts.

3. **Open obligations are withdrawn, by name.** Every pending native-Windows
   obligation in an open change is struck — for issue #226: the Windows port
   of the lookup oracle, `GetBinaryTypeW` evidence, the native Windows
   matrix and MSRV-on-Windows execution. A ledger says *withdrawn by decision
   0063*, never *pending*, and never claims an execution that did not happen.

   **Enforcement binding:** the owning change's `tasks.md`, amended by the
   next run that touches it.

4. **CI and release stop building it.** `windows-latest` leaves the test
   matrix and the required status checks; `brokkr-windows-x86_64` leaves the
   release matrix; the Scoop channel is retired with a final note that points
   to WSL. Branch protection is the operator's to change, and the leg stays
   required until they do.

   **Enforcement binding:** `.github/workflows/ci.yml`,
   `.github/workflows/release.yml`, `packaging/`, the repository's branch
   protection.

5. **Existing Windows code is unmaintained and leaves when touched — never
   as a campaign.** Deleting is effort too. Runtime `cfg!(windows)` branches
   go first, because the exact coverage gate counts them on every host;
   `#[cfg(windows)]` items go when the file around them is next edited.
   Host-agnostic validation of *data* stays: `realms.rs` refusing a
   Windows-rooted path inside a map on every host validates what a map may
   say, and supports no host.

   **Enforcement binding:** the exact coverage gate, which already refuses a
   line no supported host can reach.

6. **The guides say so plainly.** `docs/guides/contributing-by-hand.md`'s
   check table loses its Windows row; `docs/guides/driver-authoring.md` and
   `docs/guides/provider-adapters.md` stop describing Windows behaviour as
   something the engine answers for. One sentence replaces them: *on Windows,
   use WSL2.*

   **Enforcement binding:** the guides named above.

## Consequences

- **Decisions 0043 and 0046 are amended in part.** Their Windows clauses —
  compile-and-refuse for boxed bundles, "`harness` or `container`" as the
  Windows boundary — describe a host that no longer exists. Their Linux and
  macOS rulings are untouched, and their text is not edited; this decision is
  where the amendment lives.
- **One fewer required check, and an exact gate every supported host can
  reach.** The never-cwd rule no longer carries a platform on which it can
  only refuse.
- **Someone on Windows loses nothing they could use.** Native Windows never
  ran a boxed seat. WSL2 runs everything Linux runs, box included.
- **The last Scoop version stays installable and unsupported.** Retiring the
  channel is outward-facing and happens on acceptance, with a note — not
  silently.
- **Reversible, with a precondition.** A later decision may re-admit Windows.
  It would have to bring the sandbox first; a host that cannot box is the
  state this decision ends.
- **This record enacts nothing.** Status is `proposed`. On acceptance the
  enactment is one slice: the two workflow matrices, the packaging channel,
  the README and guides, the house rules, and #226's withdrawn obligations.
