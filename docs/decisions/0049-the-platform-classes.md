# 0049 — The platform classes: what each operating system is promised, and what is said where a promise cannot hold

Status: accepted (operator ruled in chat, 2026-09-07)
Date: 2026-09-07

## Context

Decision 0046 named the boundary a run stands behind so that every seat
could run on every operating system and the record would say honestly
which wall stood. Its first enactment slice then spent five rounds on
Windows, and the last two were security holds of the same shape: a
pinned script's identity is the exact bytes of its filename components,
and an interpreter's own startup grammar can resolve those bytes to a
different file.

- The first: Git Bash and MSYS enable globbing at startup for arguments
  from a native parent, so a component spelled `scripts[1]` can resolve
  to a sibling `scripts1` that the pin never covered. Closed by refusing
  a documented set of fourteen startup characters at compile, on every
  host (proposed decision 0048).
- The second: the same forgery through `powershell.exe`, whose first
  positional argument is a command string and whose tokenizer removes a
  backtick, so `scripts` + backtick + `1` resolves to `scripts1`. The
  refused set had been derived from one interpreter's behaviour while
  the grammar admits any interpreter by bare name.

Under that finding lies a larger one, which the operator named: **the
interpreter is not pinned at all.** The script's bytes are digested and
re-walked; the program that reads them is a bare name resolved through
`PATH` when the process starts. Closing the set of admitted interpreter
*names* closes nothing, because a name is not an identity — anything a
seat can place earlier on `PATH` answers to it.

That is not a Windows fact. It is true wherever Brokkr is not the wall,
which decision 0046 already names precisely: under `namespace`,
`seatbelt` and `container` the box supplies the filesystem and the
`PATH`, so script and interpreter both come from a controlled root;
under `harness` and `open` neither does. What Windows adds is that its
supported box is `harness` or `open` and nothing stronger, and that its
interpreters reinterpret filename bytes at startup in ways that differ
per interpreter and per version.

The operator ruled the scope rather than the next character: Windows is
not this shop's target platform for power builders, the effort to close
each OS-shaped hole is unbounded, and the engine will not be distorted
to chase it.

Alternatives weighed:

- **Keep extending the refused character set.** Rejected: a character is
  dangerous only relative to an interpreter, the admitted interpreter
  set is open, and each closure invites the next. The work already done
  stands; the open-ended chase does not.
- **Pin the interpreter by absolute path in a trusted prefix.** Rejected
  here, not forever: it is real hardening on Unix, has no equivalent
  convention on Windows, and is only as good as that prefix's
  permissions. It belongs to a boundary slice, not to a platform class.
- **Refuse an exec gate under `open` entirely.** Rejected: it removes
  the thing decision 0046's slice (i) was built to enable, on every
  platform, to close a hole that the record can instead state.
- **Drop Windows.** Rejected: it builds, it is tested on every pull
  request, and an operator on Windows is welcome. What changes is what
  is promised, not whether it runs.

## Rulings

1. **Three classes, and every surface names them.** A platform is
   `first-class`, `supported` or `best-effort`:

   | Class | Platforms | What it promises |
   |---|---|---|
   | `first-class` | Linux x86_64 and aarch64 | Every boundary that has landed, every gate, the full guarantee each decision states. The reference platform: where a guarantee is defined, it is defined here. |
   | `supported` | macOS arm64 and x86_64 | The engine, every verb and every readout. Boxed boundaries as their slices land (`seatbelt`, decision 0046 slice ii). A defect is a bug and is fixed. |
   | `best-effort` | Windows x86_64 | The engine, every verb and every readout, tested on every pull request. Boundaries limited to `harness` and `open`. Where an operating-system behaviour defeats a guarantee this engine states, the guarantee is **named as not holding there** rather than pursued. |

2. **Best-effort is not unsupported, and not an excuse.** Windows stays
   in the CI matrix on every pull request; a regression that breaks it is
   a defect and is fixed. What the class licenses is refusing an
   *unbounded* chase: an OS behaviour that would require this engine to
   reimplement an interpreter's startup grammar, or to defeat a shell's
   own argument handling from the parent process, is out of scope.

3. **Where a guarantee cannot hold, the engine says what it is.** This
   is the rule that makes the class honest, and it is decision 0001's
   temperament: silently short evidence is worse than none. A guarantee
   narrowed by a platform is stated in the decision that makes it, in
   the capability spec that enforces it, and in the readout that would
   otherwise imply it. No guarantee is quietly weakened on every
   platform to make one platform's story simpler.

   Applied to the finding that prompted this: the pinned-script rule is
   an integrity check over the bundle's own bytes. It is an execution
   guarantee only under a boundary that supplies the filesystem and the
   `PATH` — `namespace`, `seatbelt`, `container`. Under `harness` and
   `open` the interpreter is unpinned and resolved through an inherited
   `PATH`, so the rule defends a careless bundle and not a hostile seat,
   and decision 0048 and its capability spec say so in those words.

   **Enforcement binding:** the platform table lives in one place and
   every surface that names a platform cites it — `README.md`, the
   quickstart's install and platform paragraphs, `packaging/README.md`;
   a test that the classes named in prose match the table, in the manner
   of the recipe-table test.

4. **Above the engine, the caller carries what the platform cannot.**
   Where a Brokkr guarantee does not hold on a platform, the layer that
   dispatches the run is what makes the run safe — by choosing the host,
   by supplying an outer wall, or by not dispatching there. Brokkr's part
   is to state the fact in the record so that layer can act on it: the
   boundary is already pinned in the manifest and rendered *unboxed*
   where it is one (decision 0046 ruling 3), and the outer wall an
   orchestrator built is the field that issue #217 proposes. No
   orchestrator is named or assumed, here or anywhere.

5. **This ruling is a scope, not a licence to lower the record's
   standard.** Nothing here permits a claim the evidence does not
   support, on any platform. A finding on a best-effort platform is
   still recorded, still judged, and still closed by name or carried as
   named debt (decision 0047).

## Consequences

- The security hold that prompted this is answered by ruling 3, not by
  ruling 1: the backtick route through `powershell.exe` is real, and
  what changes is that decision 0048 stops implying an execution
  guarantee it cannot make under an unboxed boundary. Whether the
  character is added to the refused set is an engineering choice with a
  small cost and no claim attached to it.
- Decision 0048's sentence excluding shell-source interpretation for
  every admitted interpreter is wrong as written and is corrected: it
  reasons from one interpreter's parser about an open set.
- The quickstart's platform paragraph, which today says macOS and
  Windows adopters need a Linux box for boxed gates, becomes a
  statement of the classes and the boundaries each admits.
- Nothing in the engine changes for Linux or macOS.
- **Deliberately unruled.** Whether the interpreter is pinned by
  absolute path under the boxed boundaries, which would strengthen
  first-class rather than rescue best-effort, and belongs to a boundary
  slice with its own measurement. Whether `pwsh` — which takes `-File`
  positionally rather than `-Command` — behaves differently enough to
  matter; it is a measurement nobody has made and no claim rests on it.
