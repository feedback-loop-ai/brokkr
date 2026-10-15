# Persistent local CI

GitHub sends jobs directly to three always-on Docker containers. No hosted
runner boots to establish a VPN or SSH connection. The runner opens its own
outbound HTTPS connection to GitHub; Headscale remains available for host
administration.

| Worker | Jobs | Default CPU set | Memory ceiling |
| --- | --- | --- | --- |
| engine | Linux workspace tests | 0-7,16-17 | 12 GiB |
| coverage | Exact coverage and its ratchets | 8-15 | 14 GiB |
| build | Clippy/contracts and the Linux release binary | 18-23 | 6 GiB |

The CPU sets match this workstation's i7-13700: the two long jobs each get
four performance cores and their siblings. Engine also gets two efficiency
cores; build uses the other six. Coverage runs four test threads on its
performance cores: instrumentation brings a large-response RPC proof close
to its fixed deadline even alone, so extra concurrency makes retries slower.
Set `ENGINE_CPUS`, `COVERAGE_CPUS`, and
`BUILD_CPUS` before starting on another machine. A worker executes one job
at a time. Separate volumes prevent cross-worker Cargo locks and coverage
profile collisions. CPU sets keep the boundary proofs from competing with
coverage on the same cores. These are starting allocations, not a claim of
a measured optimum.

Rust toolchains and bubblewrap are installed once. The small Ubuntu image
also reduces the system tree inspected by the real broker boundary tests.
The runner's whole filesystem is copied to a native disk volume, which
becomes the container root: the broker refuses overlay filesystem identities.
Startup mounts the container's own proc/dev/sys into it, then starts the
unprivileged runner. SYS_ADMIN remains available to the setuid fusermount3
helper, with `/dev/fuse` exposed for real blocked-filesystem boundary proofs.
Each worker has a stable generated machine ID and Git 2.55.0 for reftable tests.
The copy preserves installed hard links. Six standard host device nodes
(`/dev/null`, `zero`, `full`, `random`, `urandom`, and `tty`) are bound into
each container to preserve the devtmpfs identities the broker proves.
Each container runs its own systemd-resolved stub, forwarding upstream to
Docker DNS; this supplies the same resolver boundary exercised on hosted CI.
Disposable test journals and repositories use bounded RAM filesystems at
`/tmp` and `/var/tmp` (2 GiB each) to avoid disk journal waits. They count
toward the worker's memory ceiling. Compiler outputs and coverage caches
remain on NVMe across jobs and restarts.
Each worker's registry, toolchains, workspace artifacts and incremental
compilation state persist on local disk. Checkout replaces tracked source files; `local-runner` cleans untracked
files while preserving only the real workspace `target` directory. Keeping
it inside the workspace also makes the test binary available to hands boxes. Hosted
cache download, compression, pruning and upload are skipped locally.
Coverage retains its existing dependency cache and still clears workspace
instrumentation and profiles before every measurement.

## Setup

Docker Compose and an authenticated `gh` with repository administration
access are needed on the host:

```sh
bash ops/ci-runner/manage.sh build
bash ops/ci-runner/manage.sh profile
bash ops/ci-runner/manage.sh register
bash ops/ci-runner/manage.sh start
bash ops/ci-runner/manage.sh status
gh variable set BROKKR_LOCAL_CI --body true
```

Registration passes a short-lived token over standard input. No personal
access token is stored in the image or container environment. Each runner
keeps its own GitHub runner credentials in its runner volume and updates
its runner application automatically.

The workflow uses the local workers when `BROKKR_LOCAL_CI` is `true`, for
pushes, merge groups, manual runs and pull requests from this repository.
Fork pull requests use hosted runners. A pre-job hook independently rejects
fork pull-request payloads before checkout. These persistent workers are for
trusted repository contributors; their state is reusable by later jobs.
Containers do not mount the host home, working checkouts, Docker socket or
host network. Nested bubblewrap and ptrace require unconfined seccomp and
unmasked container system paths, and the named `brokkr-ci` AppArmor profile,
which allows user namespaces for these containers without changing the
host's user-namespace sysctl.
The `profile` command installs this one profile using a temporary Docker
helper with MAC_ADMIN. This is not a VM security boundary.

The macOS jobs and short Linux checks continue on hosted runners so they
do not queue behind the long local jobs. Tagged release publication keeps
its existing cold hosted builds and attestation path.

## Operation

```sh
bash ops/ci-runner/manage.sh logs engine
bash ops/ci-runner/manage.sh stop
bash ops/ci-runner/manage.sh start
gh workflow run ci.yml --ref YOUR_BRANCH
```

For hosted execution, set `BROKKR_LOCAL_CI` to `false` before starting a new
workflow. Jobs already queued for a local label must be cancelled and
rerun after the variable changes. There is no automatic fallback for an
offline workstation. Docker restarts the workers after a host reboot.
`stop` preserves all state; `docker compose down -v` destroys registration
and caches and should only be used for an intentional reset.

For an image/toolchain update, stop the workers after active jobs finish,
rebuild the image, and recreate the containers. Existing home volumes keep
their tools; the workflow installs a newly pinned Rust toolchain there.
A bubblewrap pin change also requires rebuilding the image, since the
local setup step verifies its version before tests start.

Compare the first cold run with a second run of the same commit using the
Actions step timings. Keep the actual test and coverage commands identical.
Fixed broker startup deadlines still cost time; more CPU cannot shorten
those waits. Check disk usage periodically: persistent target directories
trade NVMe space for lower latency.
