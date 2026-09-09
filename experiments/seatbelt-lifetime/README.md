# Seatbelt descendant-lifetime feasibility experiment

This standalone Rust experiment tests the **original-process-group candidate**
for issue #253 / decision 0046 R3. It is a falsification tool, not a working
Seatbelt implementation or the full native acceptance suite. It does not
change Brokkr's unbuilt-boundary fence or require any provider account.

On a clean published candidate checkout on macOS with Apple's Command Line
Tools and Rust stable installed:

```sh
bash scripts/validate-seatbelt-macos.sh --lifetime
```

The runner prints an evidence directory and `.tgz` archive. Attach that archive
to #253. It records source revision, host/architecture, compiler, source/binary
hashes, per-case topology and PIDs, live-state observations, heartbeats, errors,
cleanup confirmation, and a JSON summary. Allow roughly two minutes after
compilation. Run without sudo. Existing global Git/provider configuration is
not modified. The artifact contains test paths with the local account name;
review the small text logs before sharing. It contains no environment dump.

Exit codes:

- **1:** a valid native experiment completed. R3 remains OPEN, even if a future
  OS unexpectedly kills every observed child. Review each case. This experiment
  cannot establish arbitrary-payload safety or both Brokkr hands paths.
- **2:** prerequisite, observation, control or cleanup failure; evidence is
  incomplete/invalid. Keep logs and leave R3 OPEN.
- There is **no native success/activation exit**. Missing cases are not passes.

## What is measured

The candidate supervises a fresh process group. On a 300 ms timeout,
explicit cancellation, or direct-parent exit it sends SIGKILL to that group.
A fourth trigger kills the candidate supervisor itself. The observer is a
separate process, so candidate death does not disable measurement.

The matrix is three payload topologies (ordinary child, child calling setsid,
and setsid followed by a second fork) by four triggers. Helpers ignore SIGTERM.
The double-fork intermediate exits. Each case must establish two live helper
identities, the expected process-group relationship, and an advancing leaf
heartbeat before the trigger. After up to five seconds of teardown, the
observer checks process state and another one-second heartbeat interval.
Stopped processes are survivors; zombies are recorded as nonexecuting. PID
observations include a unique case path in the fixed helper argv; a fixture
never execs another program. This is a controlled experiment, not a general
hostile-process identity algorithm.

A no-cleanup negative control must leave live ordinary children and be detected.
If it does not, candidate results are invalid. Linux self-tests additionally
check ordinary timeout cleanup and known detached/supervisor-death survivors.
Unexpected outcomes are retained but classified as inconclusive: investigate
fixture failure or external interference before inferring a new OS guarantee.

Payloads execute via `/usr/bin/sandbox-exec` with the intentionally broad
profile `(version 1) (allow default)`. This isolates the lifetime question:
launching under Seatbelt alone does not establish an execution-lifetime
boundary. There are no filesystem, network, credential, overlay or hooks
claims from this profile. No speculative SBPL restriction is credited with
preventing setsid. The fixture is controlled code, not arbitrary user input.

## Bounded cleanup and limits

After recording the verdict, the outside observer writes a fixture-only cleanup
marker. Helpers voluntarily exit on that marker. Every helper also self-expires
after 30 seconds; expiry before the verdict invalidates the measurement. This
cooperation is **test cleanup only**, never the candidate's guarantee. Helpers
do not spawn arbitrary code, and their output is a fixed-size heartbeat file.
The runner leaves scratch/evidence in place; it never removes a live payload's
files. An observer crash still leaves each fixture bounded by self-expiry.

The supervisor retains its unreaped direct child while sending a negative-PID
group signal so the group identifier cannot be reused. The observer signals
only its own unreaped `Child` supervisor, never arbitrary enumerated PIDs.
It verifies known helpers stopped after fixture cleanup; missing cleanup is
an error, with the safety expiry documented separately.

This first experiment does **not** test retained output pipes, an external
kernel-backed watchdog, races under arbitrary fork storms, or both MCP/exec
integration. Those remain requirements for any replacement candidate. It
intentionally has no launchd job installation or daemon and changes no host
service settings. A rejected process-group candidate is useful evidence,
not a proof that all macOS solutions are impossible.

## Basis and next decision

Apple's [setsid documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setsid.2.html)
and [kill documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/kill.2.html)
explain session/group creation and group-directed signals. Apple's published
[launchd.plist manual](https://github.com/apple-oss-distributions/launchd/blob/main/man/launchd.plist.5)
describes cleanup of the job's process group. Our inference is that these
interfaces alone do not establish descendant cleanup after session detachment.
The native experiment measures that inference on the operator's actual Mac.

If survivors are observed, preserve SEATBELT-R3 and select a new demonstrable
native lifetime mechanism before implementing the full boundary. Do not
relabel fixture cooperation, process polling or signal delivery as kernel
containment. If no enforceable mechanism is found, return the residual to the
operator; the accepted no-survivor guarantee is unchanged.

## Local harness verification (not native evidence)

```sh
rustc --edition 2021 -D warnings --test experiments/seatbelt-lifetime/main.rs -o /tmp/seatbelt-lifetime-tests
/tmp/seatbelt-lifetime-tests
rustc --edition 2021 -D warnings experiments/seatbelt-lifetime/main.rs -o /tmp/seatbelt-lifetime
/tmp/seatbelt-lifetime --portable-self-test "$(mktemp -d)/measurement"
```

The portable self-test returns zero only when its observer detects the known
control failures and ordinary cleanup. It never writes native=true or closes
R3. The std-only fixture sits outside the production Cargo workspace so the
experiment does not become a shipped boundary or alter production coverage.
