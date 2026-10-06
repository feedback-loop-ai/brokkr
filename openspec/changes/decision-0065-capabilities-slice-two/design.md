# Decision 0065 slice two — broker, gates and citable calls

Status: proposed design for operator ruling; documents only.
Change: decision-0065-capabilities-slice-two.
Provenance: [evidence.md](evidence.md#scope-and-source).

## Context

[proposal.md](proposal.md) states the adopted scope. Slice-one D4's admission
ordering and D11's restriction deferral remain constraints. The design adds
broker mediation, gate policy and evidence without changing realm authority.
The 2026-10-05 ruling adds a separate server box; this visit is recorded in
[evidence/design-boxed-broker.md](evidence/design-boxed-broker.md). The source inventory below explains the implementation cuts; the durable
[review record](evidence.md#repair-adjudication) records dispositions.

The following evidence was rechecked at this base. Paths use the crate name
plus src unless prefixed with contracts; line facts are starting evidence,
not promised locations after another thread merges.

| Checked source | Design consequence |
| --- | --- |
| runtime/capabilities.rs:234–242, :477–490, :621–716 | SiteAsks lacks class; ToolDialect discards the four MCP fields. Type the edge and carry site facts. |
| runtime/capabilities.rs:1436–1526, :1626; cli/doctor.rs:999–1001 | Validate every grant before the unbuilt-kind fence; remove native-binding index/expect assumptions before enablement. |
| core/realms.rs:403–517 | Three reserved keys today; reserve retain by realm version, never globally reinterpret old restrictions. |
| runtime/bundle.rs:6054–6091 | Class is consulted only for a harness fragment today; capability policy must receive canonical class on every site. |
| runtime/engine.rs:4434–4512; protocol/hands.rs:1119–1142 | Harness launches hands. A broker belongs in that config and attempt tree. |
| protocol/native_controls.rs:2433–2456 | Final proof admits only the current transport; adding flags alone will refuse. |
| cli/init.rs:679, :745, :859, :1776–1792; cli/verbs/setup.rs:44–51 | Generated declarations and printed scaffold instructions are independent migration consumers; U1f2 precedes strict admission. |
| protocol/process.rs:488–497; runtime/engine/checkpoints.rs:221–235 | Private observation data cannot cross the closed store fence unchanged; delay emission until all consumers exist. |
| protocol/adapters.rs:1603–1620, :1836–1891 | Names are clamped and Codex emits only item type; normalize/correlate before display conversion. |
| runtime/engine.rs:2091–2107, :2394–2406; engine/sequence.rs:395–416, :616 | Single/panel sinks and step dispatch/settlement all need the same attribution and evidence barrier. |
| store/lib.rs:316–329; store/seat_record.rs:57–115 | Append and offline reading share engine-line contract dispatch; v6 must be additive on valid old records. |
| protocol/adapters.rs:687–704; protocol/secret.rs:20–30, :105–137; cli/tests/machine_proof.rs:3141–3184 | One injector overwrites existing entries; HOME/TMPDIR remain globally valid. Protect broker-owned keys before lookup, and preserve injector cardinality. |
| runtime/engine/marks.rs:25–55, :128–155; capabilities.rs:1314–1370; protocol/adapters.rs:226–269, :398–425 | Selection, stale-notice clearing and capability rendering already exist. Derive discovery from that selected intent in U7c/U7d. |
| runtime/bundle.rs:4352–4362; contracts/tool-dialect.v1.schema.json | Binding minimum is an existing policy fact; MCP version/connection/secrets/retained already have a frozen schema. |
| agents/charters/researcher.md:20–30 | Existing prose names both capabilities and the exact DATA clause in one paragraph. |

## Goals / Non-Goals

A broker launch must be reconstructible from one sealed selected holding.
All effects remain above pure core/view (0071 ruling 1); no second policy
resolver, general plugin registry or trait-object catalog (rulings 2, 5, 10).
Preserve the current native denial and private-origin final-command proof.
Every future unit must have a production consumer, bounded files and binding
tests (rulings 4, 6, 9).

No URL client, native response retention, nonempty restriction transport,
whole-harness box, new provider, new shipped MCP server or
capability grant, comparison/slice-three work, garbage collector or release.
R2 does not build macOS namespace support; macOS keeps its exact refusal.
This is a document plan; U0 and implementation proofs remain pending.

## Decisions

### D1. Migration, checkpoint handoff and provenance

| Alternative | Decision, evidence and owner |
| --- | --- |
| Repository declarations alone migrate strictness | Reject. init.rs independently generates Claude, Codex and dsh metadata at :679/:745/:859 and selects it at :1776–1792. U1f2 migrates those consumers and instructions before U1g; SI2 requires exact parity and fresh-scaffold compile success/refusal in init_doctor/init_stacks (0071 rulings 5, 9). |
| Normalize and immediately emit in U4c | Reject. process.rs:488–497 forwards driver data; engine.rs:2091–2107 and checkpoints.rs:221–235 pass it to store/lib.rs:316–329. v6 accepts complete groups, not private observations. U4c keeps legacy emission; U4e/U4f install all consumers, then U4f2 activates emission with native boundary proofs (rulings 3, 9). |
| Keep provenance only in branch commits or behavior scenarios | Reject. A squash need not retain those commits. [evidence.md](evidence.md#visit-chronology) owns chronology and complete reconciliations; deltas describe behavior, with masking/session/startup reasons at their owners (ruling 5). |
| Restrict strictness to seats requesting MCP | Reject. 0065 ruling 6 says "A harness's own MCP configuration is never inherited". SI2 applies to every model seat, including empty grants. U0 supplies evidence; an unseatable self roster needs an operator decision, never an exemption (rulings 3, 8, 9). |

R1–R5, namespace-only broker admission, tool-dialect v1, seat-record v6,
manifest v12, the next realms version after v7, D11 and both native-binding
panic repairs stand. The operator alone accepts 0077. The claim-by-claim
reconciliation is in [evidence.md](evidence.md#repair-council-reconciliation).

A generic scaffold framework, provider substitution or initialization exemption
is rejected: the three U1f2 paths already own generation and instructions;
applicable assessment parity and independent compile proofs bind the necessary
copies (0071 rulings 5, 6, 9). Whole-adapter equality would erase legitimate
stack-specific tools and does not replace checking measured limitations.

A runtime rollout flag, permissive interim schema or public observation version
is rejected: U4's source/dependency order preserves legacy output until all
consumers exist. Extra runtime combinations or durable private fields add no
needed consumer and weaken the typed boundary (rulings 3, 5, 6).

### D2. U0 is an experiment with an explicit decision rule

U0 uses disposable host-side configurations, fake sentinel MCP servers and
real installed harnesses on Linux/macOS. It changes evidence only. Capture
binary version, host, cold/eligible-resume/replacement shape, effective config
sources, exact argv/environment names (redacted), server lifecycle log,
tool listing/discovery/call attempts, native call events and limitations.
No production default is changed by this experiment.

| Harness | Candidates and distinct measurements |
| --- | --- |
| Claude | Strict MCP flag plus explicit empty, hands-only and engine-sentinel config. Plant user/project/plugin ambient sentinels; inspect starts, lists and denied direct calls. |
| LaneTally | The actual wrapper and child, with the same sentinel matrix. Neither Claude's result nor successful argv forwarding qualifies the wrapper. |
| Codex | (a) a private engine-owned config/home containing only needed auth references and session/model configuration; (b) replacement of the complete MCP table using supported config precedence. Measure user, project, managed and plugin sources, auth functionality, normal model/effort, session storage/rejoin, deferred discovery, and native/MCP event call identities independently. |
| dsh | Existing profile/plugin configuration versus an engine-only profile candidate; separately measure ambient exclusion and engine-server loading. No new plugin is built here. |
| exec | No model MCP surface. Record inapplicable, not measured strictness. |

The positive control must start/discover/call each planted ambient sentinel.
A candidate succeeds only if all those ambient servers neither start nor
appear nor answer while the engine sentinel works. A model voluntarily
ignoring a server is not exclusion evidence. Run with and without hands and
on each currently supported serving shape; do not qualify a new resume
shape. Existing native OFF must survive both candidates.

Choose a mechanism only after its row passes. If both pass, prefer the one
that isolates configuration without copying credentials/session data, with
fewer independent precedence assumptions. If neither passes, declare the
shape unmeasured/unsupported with SI2's exact cause; no fallback to ambient
config. U0 also captures the exact native/MCP call identifiers and repeated
start/completion semantics required by U4. A changed harness version needs
new evidence or refusal. Measure native resumed-history replay versus new calls.
For secret-bearing use, plant noncredential canaries in the selected store and
child environment, prove the positive controls readable without isolation,
then probe workspace hands and every model-native read/process surface on each
shape. Protected mode bits and a write-denial flag are not a passing result.
Record store-read, process-read, native-write and ambient-MCP results separately;
MB2 refuses secret-bearing holdings unless both read channels are excluded.
Secret-free holdings can pass other protections without this qualification.
Measurement can close rows through honest refusal;
admitting additional shapes or inventing a third mechanism requires updating
this plan before dependent implementation. #500 can supply evidence only
when its actual observations meet this matrix; its verdict is not authority.

### D3. One admission order, two kinds of refusal

Use typed SiteClass, capability classes, BindingKind and compatibility causes.
No string vocabulary or Value is carried into the new policy state (0071
rulings 2, 3, 8). Preserve the existing constitutional checks that precede
capability resolution. New ordering is:

1. Strictly load original source bytes, all consulted definitions and every
   selected-realm grant/dialect; schema/duplicate/containment/reserved-key
   faults are unconditional. Loaded-library and inline DATA lint uses verified
   charter bytes, including asks later subtracted.
2. Until U9b, return the exact existing realm-wide MCP unbuilt-kind cause
   after grant validation, even for unused or offices-empty MCP grants.
   U2 tests non-native seams directly; it adds no compile bypass.
3. Per remaining site/candidate: subtraction, grant presence, office reach,
   nonempty tool set. An unused valid grant is pinned and inactive after U9.
4. For an applicable MCP request, MB2's hard checks in order: namespace,
   workspace hands, network false, recognized model channel, proved native-tool
   write confinement protecting evidence, then measured store/child-process
   read isolation for secret-bearing dialects. Requires and wants both refuse
   on failure; secret-free holdings skip only the last check.
5. GP1 for both kinds: writes prohibition before explicit-office egress.
   Requires refuses; wants loses the whole holding with its exact notice.
6. MCP route clearance against the binding minimum (U5a2), then
   provider/native or MCP carriage, representable identity and connection,
   then empty-only restrictions. URL and argv references
   get SC1 causes. MCP nonempty restriction cause is exactly
   "MCP nonempty restrictions are deferred to the restriction-transport slice",
   through GP1's required/optional forms. Never discard only the restriction.
7. Independently prove every known unheld native power OFF and every model
   shape strictly isolated, even after drops, scope exclusions and subtraction.
   Missing measured isolation is not a compatibility drop.
8. Seal the complete outcome; establish D6's managed-writer protection before
   any writer dispatch. Bind the selected reach, executable/package and network
   intent and aggregate reservations before broker spawn. The broker reobserves
   sources, excludes the store and verifies the empty box before value lookup. Preflight the complete
   command and repeat independent final assessment immediately before spawn.
   Runtime drift is a hard launch failure, never a late optional drop.

The adapter's former dead mcp key becomes carriage data, separate from
strictness. A provider can exclude all ambient servers but be unable to load
a granted server. That distinction permits a wants drop only if the empty
or hands-only strict plan and native OFF remain provable.

U2 changes binding access to exhaustive native/MCP/reserved-hands variants.
An absent native binding returns a typed missing-binding cause; it never
indexes, expects, invents an empty provider or treats MCP as native.
The public compile path remains fenced until all later dependencies land.

### D4. Typed contracts and version ownership

SC1–SC5 own public observables. New internal types carry Connection::Stdio
or Url, RetentionDisposition::Inherit or Veto, implementation kind and call
state. Closed serde shapes deny unknown fields. Arbitrary MCP payload/schema
JSON exists only in the bounded parsing/forwarding edge; durable authority,
ledger and journal projections are typed. Typed errors preserve exact text;
existing legacy String APIs can render a typed error at their outer adapter
without adding new Result<_, String> functions.

| Surface | Planned contract / consumer |
| --- | --- |
| Tool dialect | Reuse frozen v1 and its existing embedded copy. Retain four discarded fields, same bound bytes/digest. Valid declared argv references remain data but are unexecutable in this slice. |
| Realm | The next realms version after v7. Only false is legal for reserved retain; absent means inherit. Version-aware reserved keys keep a v6/v7 retain spelling as a dialect restriction. Include every v7 provisional-office field. |
| Manifest | v12 implementation union plus declared/disposition/effective retention per candidate. Native has declared/effective false. No transient plan path, PID, resolved secret or call ID is identity data. |
| Seat record | v6 attribution group capability/dialect/tool/call_id/call_state, optional response_sha256 on succeeded/failed only, one settled broker row (no public started), and conditional turn requirement from SC4. Old rows remain valid. |
| Private transport | Shared typed broker plan and ledger records within protocol/broker.rs and its ledger child; consumed by CLI broker and runtime engine. No new public driver-protocol version or policy input. |

The v6 store dispatch boundary follows its existing engine-line convention:
select the development line actually on main when U4b lands, document why
its additive acceptance preserves older rows in that line, and test adjacent
lines. This does not require a version bump. Add new schema files and exact
embedded copies with their consumers in one PR; never edit old versions.
Manifest v12 has no existing embedded manifest copy to duplicate: its
production projection and contract validation are the consumer.

Restriction schemas remain validated/digested edge data, with no network
schema retrieval. Reserved-key traversal must receive the realm version;
hard-coding retain into the global old GRANT_KEYS would corrupt v6 semantics.
Changed declarations still move identity even when effective retention does
not; secret rotation does not. Old runs read honestly and cannot acquire new
MCP authority on resume.

### D5. Broker boundary and protocol

The tree is engine → driver → harness → broker → bubblewrap → trusted
bootstrap → dialect server (bootstrap becomes the server by exec). Hands is
another harness child, with a separate tool-call box. Decision 0077 keeps
authorization with the engine, launch with the harness, mediation with the
broker and journal writes with the engine. The broker remains outside both
boxes. Decision 0065 ruling 6's invariant is "nothing a model writes can
alter any of the three" (argv, restrictions, secrets). The
[2026-10-05 addendum](operator-ruling-2026-10-03.md#addendum-2026-10-05-each-mcp-server-runs-in-its-own-box)
implements it by construction for user-installed servers too; it supersedes
the held U6c argument, shebang and loader proofs outright.

The engine writes one `cap-<capability>` entry per selected holding beside
`brokkr`. Its sealed plan binds run/effect/attempt/site/instance,
server/capability, dialect and definition digests, version, connection, exact
tools, empty restrictions, retention and binding clearance. Add the effective
hands reach (workspace, declared binds of every mode, actual Git reach),
executable/package roots, compact source/ancestry/mount facts and source-set
digest, network disposition, bootstrap identity and excluded store/control
roots. It carries names/locators, never values. Runtime's existing typed
egress is projected once to isolated/shared network, not copied into a
second `PlanEgress` catalogue. Clearance in the private plan is the sealed
admitted comparison receipt bound to dialect and binding-policy digests; the
broker validates that receipt, not a second copy of runtime's egress ordering.
The broker reobserves filesystem facts but
never reloads grants. No descriptor number becomes durable authority.

Only the protected inventory authenticates a plan; a path and equal digest
alone do not. Independent final parse-back checks the exact broker server
set, executable, argv, plan ownership and environment removals. Authored
equal bytes retain authored provenance. All plan fields have immediate
binding/validation consumers; future server invocation is not an excuse for
unused public state (0071 rulings 2, 3, 6, 8).

#### Bind set and installation rule

Extract the common namespace/mount construction from
`hands.rs::box_argv` into `hands/namespace.rs`. Keep `box_argv`'s existing
workspace/exec callers (`execute_in`, `run_boxed_in`), its workspace/Git/bind
orchestration and `overlay_argv` behavior. Both profiles use one builder and
one `HOST_TOOLCHAIN_BINDS` table. The server profile narrows that table's
`/etc/ssl` entry to `/etc/ssl/certs`; hands retains its existing source.
This consumed projection is the only certificate difference, not a copied
system list. The closed server profile has no workdir,
Git, bundle, declared hands, host HOME or host-backed private-directory
mount, and never uses a fake workdir or copies the workspace argv skeleton
(0071 rulings 4, 5, 10).

Exactly these system sources are read-only where present: `/usr/bin`,
`/usr/lib`, `/usr/lib64`, `/usr/include`, `/usr/share`, `/usr/local`,
`/usr/libexec`, `/bin`, `/sbin`, `/lib`, `/lib64`, `/etc/ssl/certs`,
`/etc/ca-certificates`, `/etc/alternatives`, `/etc/ld.so.cache`,
`/etc/ld.so.conf`, `/etc/ld.so.conf.d`. Canonical aliases such as `/bin` to
`/usr/bin` preserve in-box spelling without adding sources. Missing optional
sources stay absent. No entire `/`, `/usr`, `/etc` or host home is a shortcut.
Never bind `/etc/ssl/private` or other certificate-directory siblings; missing
TLS configuration fails inside. An unreadable entry within an actual source
still refuses, rather than being silently omitted.
Use generated passwd/group/hosts/nsswitch, PID-scoped procfs and minimal dev.
Only shared network adds the checked read-only resolver `/etc/resolv.conf`.
Generated nonsecret identity bytes can use sealed memory FDs and
`--ro-bind-data`; no host identity/credential file is copied wholesale.

Resolve argv[0] using its absolute path or fixed `/usr/local/bin:/usr/bin:/bin`,
never ambient PATH/cwd; a relative slash path refuses. Retain its search and
symlink resolution chain. MB3 owns the closed tree distinction: a resolved
entry directly in the fixed system set's shared bin/sbin directories is a
single-file system entry. It never derives `/usr` or `/usr/local` as a
package root. The file itself must be singly linked; its support files are
admitted by the separate system-source rule. A symlink into a dedicated
installation is classified by its resolved location, not its launcher path.
Every other entry is a package: its canonical parent, or one level above an
immediate `bin`/`sbin` parent. `/opt/docs/bin/server` binds `/opt/docs` with
siblings; `/usr/lib/docs/bin/server` checks the entire `/usr/lib/docs` tree
but adds no mount when already covered. Every regular package file must be
singly linked, including system-contained or root-owned packages. Otherwise
bind that tree read-only at its canonical absolute path. Root `/`, host HOME
or an ancestor containing HOME refuses. Exec the resolved executable; do not
invent a package closure for a system runtime or add mounts from its argv.
The current Brokkr binary, if needed outside the system set, is bound as one
checked read-only bootstrap file, never its development checkout.

Do not inspect program arguments, wrappers, shebangs, ELF/RUNPATH, language
imports or environment values to guess dependencies. Do not run an installer
or expand mounts after errors. Anything missing fails **inside** this box;
a bare runtime has exactly the same filesystem and receives no extra mounts
from its arguments. Operator-installed code remains trusted, and version
negotiation is an assertion after exec, not executable authentication.

The installation is trusted as a whole. A broad but otherwise admissible
`~/.cargo/bin/server` derives `~/.cargo` and can expose unrelated
`credentials.toml`; masking knows declared values only and store exclusion
protects the selected store only. Recommend a dedicated credential-free
package tree. Admission discovers neither arbitrary credentials nor malicious
installed content. It excludes current/concurrent managed seat writes; it
cannot authenticate bytes planted by a completed seat whose earlier bind was
removed. The operator must establish trusted installation provenance or
reinstall after such exposure. A source metadata digest and version assertion
are not historical byte authentication. Future proofs must stay within these
limits (0071 ruling 9).

#### Reach and host-source admission

MB3 owns exact refusal precedence and causes. Compare both containment
directions between every source/destination and canonical seat reach: the
workspace, all declared `ro`/`rw`/`overlay` roots, and effective Git reach.
An encompassing system bind is refused too. The fixed read-only toolchain
is common infrastructure, not itself a declared reach root; explicit binds
into it do count. No program-tree regular file may have link count above one;
directory link counts do not decide this. Special files refuse; symlinks
never add binds, and a target in seat reach refuses even if not mounted.

Use a bounded Linux observer in `hands/namespace/sources.rs`: descriptor-
relative `openat`/`fstatat` with no-follow component walks, retained source
root handles and an active ancestor stack, and strictly parsed
`/proc/self/mountinfo`. Do not retain one FD per traversed file. Record device,
inode, mount root and relative subpath. Map nested mounts independently;
different mount IDs do not make a bind alias a different file. Resolve chain
identity before and after observation. Refuse unknown/ambiguous filesystem
identity, including overlay/remote arrangements the observer cannot prove.
The admitted source handles, not pathname reopens, supply bubblewrap's
`--ro-bind-fd`. The checked launcher is executed through its pinned file
handle; its trusted host runtime gets only a cleared fixed environment.
No server-loader qualification is reintroduced by checking trusted box inputs.

MB3 fixes traversal at 1,000,000 entries, depth 64, 40 symlink hops per
resolution, 65,536 mount records, 256 MiB metadata and the absolute startup
deadline. There is no skipped unreadable entry or permissive truncation.
Check the entire effective source set, not just the extra package bind.
Program/bootstrap links keep their unconditional specific cause. For a
multiply-linked system support/launcher file outside the program tree, apply
MB3's complete kernel write-exclusion predicate: mapped owner distinct from
every managed writer, no group/other write bits or extended access ACL, and
proved inability of any managed surface to acquire owner authority, change
permissions/ACLs, bypass DAC or remount the source. Identity/mode/ACL facts are
observed through bound descriptors; credential mappings and privilege
confinement are bound to the plan and enforced for its lifetime. Unknown or
overflow mappings refuse. Root ownership or protected_hardlinks alone is
insufficient. The proof holds across unenumerated hard-link names because
ownership and write permissions are inode properties; it never waives reach,
store or mount-alias checks. A linked user-owned library refuses. Program
files cannot use this exception even when root-owned. Protect original
resolution ancestors and aliases against managed seat replacement, not
merely other uids. Native surfaces must be measured to
exclude new writable aliases for the attempt lifetime too, or refuse. A
read-only mount is no snapshot of a source writable under another name.

The same proof covers bubblewrap/bootstrap, generated input sources, store,
plan and evidence roots. Retained handles close path substitution, not writes
to their contents; absent writable aliases and reach confinement supply that
second obligation. Check mount destinations for collisions, inspect all actual
nested read-only state at readiness and fail closed on mismatch. All admitted
managed writers must preserve the same exclusion, including writers admitted
later from another worktree. D6 orders their admission against durable source
reservations; a snapshot of current reach or a worktree-local lease alone
cannot supply the lifetime guarantee. Reuse the protected inventories for
coordination, with no separate registry or daemon. Arbitrary operator host
mutation remains outside the claim.
No previous preparation's verdict may be cached or subtree skipped to meet
limits. Source metadata is bounded independently of live handles; overflow
or cancellation refuses through MB3's identity cause before lookup.

C2 feasibility is measured under SD4: U6c5's actual observer must complete
within 10 s on each recorded Linux qualification profile, and U6f/U9b must
complete readiness/handoff/initialization within 20 s overall on those
profiles. Include both engine preparation and broker reobservation in that
full-path total, and report each observation separately under its 10 s budget.
Deadline/cancellation proof covers blocking source operations and control I/O;
a clock check between potentially stuck calls is insufficient. These limits
claim no general containment of server CPU, memory or denial of service.
These qualification budgets leave margin under the unchanged
30 s runtime bound; they introduce no bypass or configurable timeout.
Record cold and five warm samples, counts, FD/memory high-water marks,
kernel/filesystem and mapped privilege facts for Linux x86_64/aarch64.
The current boxed descriptor-metadata survey is only partial cost evidence;
no native identity observer or namespace was qualified by it. Large CUDA/TeX
installations can exceed the fixed limits and refuse. If ordinary profiles
cannot qualify, repair this specification before U9b, without pruning the
source set or weakening writable-alias exclusion.

#### Environment, store and readiness

The fixed server environment is exactly `PATH=/usr/local/bin:/usr/bin:/bin`,
`HOME=/runtime/home`, `TMPDIR=/tmp`, `USER=runner`, `LOGNAME=runner`,
`LANG=C.UTF-8`, `LC_ALL=C.UTF-8`, `BROKKR_HANDS_BOX=1`. The server builder owns
this one table and derives its reserved names from it. HOME and TMPDIR are
separate fresh private tmpfs per server, cwd is `/runtime/home`. They do not
reuse `Session` host directories. Clear first, then add only fixed entries
and declared decision-0012 bindings; no inherited cache, git, loader, updater
or harness credential environment survives.

Shared name validation precedes fixed-key checks: PATH/IFS/LD_PRELOAD/
LD_LIBRARY_PATH/BROKKR_ stay denied, `_JAVA_OPTIONS` still fails the name
grammar. Otherwise-valid fixed-key collisions (including HOME/TMPDIR) take
MB3's kept startup-input cause before lookup, even for benign current values.
Otherwise-valid loading names such as PYTHONUSERBASE, CLASSPATH, LUA_PATH,
GEM_HOME and PHPRC are allowed **only at final confined exec**. Their host
path targets are read-only binds or absent; private tmpfs starts empty.
No language denylist or semantic analysis of secret values remains. This
claims filesystem confinement, not that a trusted runtime cannot interpret
text or download code over its declared shared network. Decision 0012's
transformed-secret limit remains.

The store is never mounted or inherited as a descriptor, even under a system
bind or bind alias. Metadata-only exclusion from hands-readable reach and
server-readable mounts runs even for zero declared names; the two MB4 causes
stay distinct. Missing or unprovable store identity refuses with MB3's
filesystem-identity cause. A protected empty store is the secret-free control.
The existing resolver's pathname metadata/read pair is not sufficient:
extract its parser into a consumed typed reader and read the admitted
no-follow file descriptor outside the box, preserving modes, names, clearance
and safe operator text. Do not read the store again to classify an error.
U0 still must exclude native store/process reads; boxing the server does not
protect the host broker's secret-bearing memory from native tools.

The order is bound plan and eligibility/compatibility, valid/fixed names,
resolution, reach/link/identity, store exclusion, verified empty namespace,
then value lookup and final exec. Until U6f the public command refuses before
any lookup or box spawn; tests exercise private consumed seams. Once serving
is complete, open the owned ledger before box preparation so setup failure
can close Failed without calls. An unbound plan opens no ledger.

Launch the current Brokkr binary's private bootstrap with safe fixed environment
and sealed nonsecret exec intent. One ready message (at most 4 KiB) travels
on a separate anonymous control pipe, never MCP stdout, and binds this child,
plan/source-set digests and observed namespace identities. Verify actual
mounts, tmpfs, PID namespace and network, not an environment marker. Sources,
readiness, handoff and initialization share one absolute 30 s startup deadline
and the attempt's remaining bound. Polling/control I/O remains cancellable.
Missing/duplicate/truncated/excess/wrong-child readiness takes MB3's box-
establishment cause with zero lookups and dialect starts.

Only then call the protocol-owned binding sender. It uses the admitted store
handle, U6a's crate-private `secret::bind_environment` on a cleared prepared
Command, and serializes that prepared environment inside the secret boundary.
One bounded length-delimited frame (1 MiB, 256 bindings, depth 4) crosses an
anonymous pipe to the waiting bootstrap; the shared receiver checks exactly
the declared names and framing. The sender closes its write end after the
frame. The receiver requires exact completion followed by EOF before exec:
premature EOF, any trailing byte or an open pipe past the deadline takes
MB4's handoff cause. Then it closes unrelated FDs, sets the final server
environment and execs the sealed command. Neither handoff direction can
change command, mounts, network or grants. No value appears in argv,
`--setenv`, `--args`, temporary files or host launcher environment. Temporary
plaintext has no Debug/logging and is best-effort wiped. Keep exactly one
`expose_for_spawn` invocation inside `bind_environment`, pinned by
`machine_proof.rs`, and separate leak scans at the actual host launch edge.

Keep one private status pipe from bootstrap to broker, whose write end is
close-on-exec. It carries at most one fixed tag from MB3's closed failure enum:
HandoffFailed or ExecFailed. No errno text, stderr or MCP content decides the
cause. The handoff tag preserves receiver-side rejection even after the sender
has closed; one exec-only tag would lose that distinction. Exec success closes
the pipe atomically. After sender completion, EOF advances only to bounded MCP
initialization, never to a successful session. It cannot distinguish bootstrap
death from exec success; the protocol check must still succeed. Apply MB3's
observed-stage causes for invalid status, death and initialization failure.
Close all other control/source descriptors before exec; the status descriptor
closes on exec, leaving only intended stdio. This private launch result is
consumed in U6c8, with no positive handoff acknowledgement, public protocol,
second handshake negotiation or additional authority channel.

All box admission refusals are pre-lookup, with MB3/MB4's exact typed causes.
Missing/invalid store values require lookup. Invalid/failed handoff has
`MCP server secret environment could not be delivered`; failed confined exec
has `MCP server could not start inside its box`. These later failures create
zero calls and cannot be misreported as pre-lookup tests. The kernel runs
shebangs in the box; no parser or host fallback attempts to predict it.

#### Network, hosts, stdio and lifetime

`local` selects an unshared network namespace (no host loopback or external
network); `contracted`/`uncontracted` select shared network; omission remains
uncontracted. Shared network includes host loopback and Linux abstract Unix sockets,
which filesystem mount exclusion cannot hide. Those host services remain
reachable to the trusted server as the ruling requires. This implements no
destination allowlist or D11 restrictions.
R2 independently keeps the seat's workspace hands at network false. U5a2
compares the dialect's existing typed egress with the binding minimum before
carriage, using its exact requires refusal/wants drop; shared network never
upgrades clearance and runtime drift is never a late wanted drop.

Only Linux bubblewrap can supply this server profile. Decision 0043 says
"The boundary is never simulated: no `bwrap`, no tool." Existing namespace
availability checks remain; missing descriptor-mount support or loss of the
checked launcher takes MB3's box-unavailable cause. macOS retains R2's exact
nonnamespace/availability refusal; it does not gain server boxing. Decision
0063's supported hosts remain Linux and macOS.

Pipe server stdin/stdout through broker filtering/masking and bounded stderr
drainage. Keep broker, bubblewrap, bootstrap and payload in the attempt's host
process group: no `--new-session`, detached group or server daemon. Reuse
`--die-with-parent`, PID/IPC/UTS and supported cgroup namespace isolation,
capability drops, and #403's group/descendant/subreaper/pidfd settlement.
Namespace isolation is not cgroup containment; #472's table-read and
read-to-fork residuals remain. Prove cancellation before readiness, during
handoff/initialize/call, startup failure, normal exit and restart with real
Linux processes. Tmpfs dies with the box; durable plans/ledgers/staging stay
outside it. No server change is needed in `hands/overlay.rs` or Session's
reaper. If nonsecret host scratch is necessary, Session only manages its
lifetime after ancestry protection; it never owns recovery evidence.

#### Seven held findings and retained duties

| Finding | Construction and remaining check |
| --- | --- |
| H1 resolution directories | Boxing removes later seat resolution reach; pre-mount search/symlink/ancestor identity and handle-bound sources still must pass. A canonical filename alone is rejected. |
| H2 ELF/RUNPATH/Python siblings | Loading sees only the approved set or fails inside; remove dependency analysis. Whole-source alias/hard-link/nested-mount protection remains, including system libraries. |
| H3 bare runtime | No separate program-argument proof; arguments add no mounts. Executable/tree admission and normal MCP initialization/version/catalog checks remain. |
| H4 loading environment names | Clear ambient state and confine allowed bindings to final exec. Shared name/fixed-key checks, safe host launcher and readiness-gated handoff remain; no expanded loader denylist. |
| H5 private ancestry | HOME/TMPDIR/cwd are private tmpfs by construction. Host plan/store/evidence/launcher/bootstrap ancestry still needs complete owner/identity protection. |
| H6 hidden store aliases | Boxing alone does not remove readable aliases. Store identity must be absent from both readable reach sets and inherited descriptors, with native read qualification still independent. |
| H7 shebang mismatch | Delete the parser: the kernel executes inside the admitted box. Protected tree and typed in-box exec failure remain, without host retry. |

These are design dispositions of the supplied HIGH findings, not repaired
runtime findings. Keep digest/plan binding, all other MB3 protocol/version/
filter/refusal requirements, typed causes, output masking and evidence
settlement. No held U6c branch supplies an implementation base.

Use the MCP 2025-06-18 stdio framing and lifecycle, whose standard describes
newline-delimited JSON-RPC and initialization/version negotiation.
See the [transport](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
and [lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
contracts. MB3 deliberately admits a bounded subset, not every optional
feature. The [tools contract](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
distinguishes JSON-RPC errors from tool results with isError; both become
failed terminal outcomes when the child returns them.

MB3 fixes literal request/response/depth/catalog/time/concurrency bounds.
Reject unsupported versions rather than echoing an arbitrary version as the
current hands server does. Initialize the real child, compare serverInfo
version with the dialect pin, enumerate every catalog page, reject repeated
cursors/duplicate names, verify every granted tool exists, then expose only
that subset. Preserve each admitted tool's schema as DATA. Dynamic catalogs
never widen authority. Do not advertise sampling, roots, prompts, resources,
elicitation or subscriptions; reject their requests, bound irrelevant
notifications by MB3's 1 MiB per-operation allowance, and implement cancellation locally with no new power.
Error responses use fixed safe causes; never echo raw child stderr/arguments.

Responses match an outstanding typed request ID, expected method and session
phase. String and numeric IDs differ; wrong, unsolicited, duplicate or late
responses take MB3's protocol cause. The private call sequence never copies a
raw ID into journal identity. Initialization/list/call deadlines are absolute;
progress/ping traffic never resets them. Timeout or cancellation ends an
uncertain session; no reconnect or replay of an external action fills a gap.

Each valid tools/call gets an internal sequence unrelated to JSON-RPC ID.
Append and fsync Started before forwarding; append/fsync Terminal after
masking/staging and before delivering. Local denials get Refused with no
child invocation or digest. Serialization uses one active call, while the
reader continues handling cancellation and bounded protocol frames.
Clock/deadline/process observations stay in the shell; tests inject clock
facts and closure seams, not mock-only traits (0071 rulings 1, 2).

Decode complete bounded protocol messages before masking all text, including
object keys and known encodings in textual content. Use existing
canonical::to_bytes and sha256_bytes once on the masked result/error object;
reuse that buffer for staging and delivery data. Before serialization, reject
numbers whose exact JSON value would change under existing numeric conversion
(large integers and precise decimals need negative controls). This bounded edge
guard is not a second canonicalization implementation. Reject duplicate keys
both before and after masking; do not parse or normalize embedded documents.
Reattach the original correlation envelope only to the masked response.
Stderr is drained with bounded masked tail and the existing rolling byte
window large enough for each known encoding, before any lossy UTF-8 conversion;
no raw diagnostic reaches a log. protocol/secret.rs:572–580 explains the
stream-overlap invariant; split multibyte/encoded canaries test its reuse.
A transformed secret outside 0012's known encodings remains its documented
limit. U6a already placed the shared injector in protocol/secret.rs, with one
production accessor invocation in total. Its machine proof scans that module too and
separates the method definition from actual calls; it must reject a second
invocation there as well as in another module. Existing harness callers consume
the extracted helper immediately; no secret_spawn module or lib registration.

After ordinary string/key redaction, check bounded prepared output and its
correlation/structural fields using the existing masking primitives/encoding
catalogue. A known occurrence that cannot be removed while preserving shape
returns MB4's "MCP response cannot be safely masked". A forwarded call records
Failed with no digest and stages/delivers no unsafe bytes; safe local error
output carries only that cause. Unsafe correlation ends the session without
an unsafe frame. Neither converting a number to text nor changing its value
is permitted. This check is distinct from numeric precision validation and
runs even under a retention veto. Legacy mask_json semantics stay unchanged;
no second serializer, encoding table or plaintext accessor is introduced.

### D6. Protected evidence and ownership

Use the engine-resolved host root
HOME/.local/state/brokkr/capabilities/<repo-sha256>/<run>/<attempt>/ outside
the operated tree. The shell resolves paths; core/view receive typed facts.
Hash the canonical operated worktree path, not the common Git directory or a
model string. Use owner-rooted no-follow handles, owner-only parents and
exclusive files. The sealed broker-plan inventory pins each plan and expected
ledger, including disjoint retention shares and possible fallback slots, before
launch. Existing engine process observations establish proven never-started
cases; uncertain missing evidence refuses. Do not add another independently
editable capability inventory or Prepared/Started/Settled launch model.
Evidence survives engine restart; hands Session's delete-on-drop scratch does
not satisfy that lifetime. D5's builder-owned startup values are never resolved
from secret bindings. The host evidence root keeps its descriptor-bound
ownership; child HOME/TMPDIR/cwd are fresh in-box tmpfs, not evidence siblings. A missing/private-root mismatch parks, never rebuilds
from an untrusted worktree lookalike.

Private plan, ledger and staging roots must be outside every seat's writable
reach. Store secrecy is a separate obligation: hands must not read the store,
and native tools must have U0 proof excluding both store and child process
secret reads. The ordinary .forge/secrets.env path is unsafe when exposed by
hands; an operator can select an inaccessible --secrets-file, but relocating a
store alone does not prove native-read isolation. Native read-only mode only
proves write containment. A secret-bearing shape that cannot qualify takes
MB2's hard refusal. No whole-harness box is invented to waive that result.

Published bytes stay at .forge/artifacts/sha256/<hex>. U8a establishes a
read-only overlay after all user binds, blocks writable aliases/overlaps, pins
ancestors and rejects symlinks, nonregular/multiply-linked artifacts and root
drift. Atomic publication uses bound handles, same-filesystem exclusive temp,
fsync, no replacement and parent sync; existing matching bytes are verified
before reuse. The journal digest is appended only after verified publication.
The broker writes only private staging, never the artifact root or forge.db.

U8a2 applies this protection to ALL Brokkr-managed writers sharing that
canonical worktree, including zero-grant panel siblings, sequence/fallback
sites, boxed exec and future runs with old artifacts. The checked common
spawn_site door (engine.rs:3192 onward) covers every driver-bearing site; the
protection plan must reach composition before the final check and be checked
again there. Protect the root before the first writer, not when its first
retaining member is reached. Existing artifacts alone require protection.
A site unable to honor it takes CR2's managed-writer refusal before dispatch.

Serialize managed runs sharing that root with one exclusive host-side writer
lease held across the run's owned-process lifetime, not one lock per member.
Panel members of the same admitted run share its protection; separate worktrees
retain parallelism only under the additional source admission below. All
current-version runs take this lease, even with no MCP holding/artifact yet, so a retaining run cannot race an already active writer.
Use the existing process/attempt facts to recover abandoned ownership; an
uncertain surviving writer refuses rather than treating lock release as process
settlement. No daemon or general lock service is introduced. U9 migration must
quiesce older engines that cannot participate; arbitrary operator host writes
remain outside this managed-seat guarantee. The observable serialization cost
and refusal of unsafe writer shapes are accepted for evidence integrity.

Source protection has a wider coordination scope than the artifact-root lease.
U8a2's `engine/broker.rs` admission operation takes one short-lived exclusive
host admission lock under the protected `HOME/.local/state/brokkr/capabilities/`
root. All participating current-version engines use that same owner-bound
root; alternate or unprovable coordination domains cannot qualify a shared
source. While holding the lock, read the existing protected run/attempt
inventories and owned-process settlement facts, compare the incoming run's
complete writer reach/privileges and protected sources with every unsettled
reservation, then durably seal its reservation before releasing the lock or
spawning. Zero-grant runs also publish their writer facts in that inventory;
there is no separate editable writer list or service. Inventory traversal and
lock acquisition share D5's metadata/entry limits and absolute startup deadline;
exhaustion takes MB3's identity cause. Refuse incomplete, inaccessible or
ambiguous participation; do not assume
an absent answer means disjointness. This lock orders admissions only, so
proved-disjoint worktrees can execute concurrently.

The common `spawn_site` door and sequence path consume the reservation before
any writer starts. Reserve the union of permitted member/fallback/step reach
before the first dispatch, or atomically extend and recheck it before the new
writer; sealed per-attempt plans still describe only the selected holding.
A source/writer conflict takes MB3's identity cause before incoming dispatch
(and before lookup when the incoming party is a broker). Artifact-root
conflicts retain CR2's existing cause. Keep a source reservation until all
its owned processes have settled; release it under the same admission lock
only after durable settlement. Engine death or a free advisory lock does not
release it: uncertain survivors keep the reservation and refuse conflicting
starts. Use existing attempt facts and #403 settlement, not a new process
scanner or stronger cleanup claim. This is proposed work, not an existing
host-wide lease facility on main. A mechanism needing a fourth file is split
before implementation; no deferred runtime recheck may replace this exclusion.
The coordination root and lock are themselves protected control inputs for
every writer, including zero-grant runs. A writer that could replace either
cannot participate. Engines unable to establish the same coordination domain
refuse a source admission; operator migration must quiesce nonparticipants.

U8a/U8a2 real namespace proofs must include a retaining member plus a zero-grant
attacker, boxed exec, an already-running writer, historical artifacts, ancestor
replacement, aliases and hardlinks alongside legitimate worktree writes. These
are a distinct dispatch/coordination PR; a producer-only mount test cannot close
CR2. Read/write protections never claim a sandbox for malicious operator-selected
server binaries. #403 and 0012's transformed-secret residuals remain named.

### D7. Ledger and crash semantics

CR3 owns the literal bounds, quota accounting and lifecycle. Use closed typed
Opened, Started, Terminal and Closed variants. Every record has a contiguous
record_seq and fixed owner; calls additionally have the next call_seq and tool.
A header reference may avoid repeating the immutable owner. No arguments, raw
JSON-RPC IDs, response bodies or arbitrary child text become durable metadata.
The broker exclusively opens a plan once: reconnect refuses rather than resetting
sequence. Reserve Started/Terminal/Closed capacity before accepting a frame.
Validate request shape and bounded tool identity at the edge, then fsync Started:
that is acceptance. Tool, active-call and retention-budget denials occur after
it and have private Started/Terminal, with no child forwarding. At ledger
capacity stop reading; no fictitious checkpoint describes unread input. Failed
Started persistence blocks successful completion and forwarding; recovery uses
only the verified prefix, including a complete record whose acknowledgement was
lost. CC2 and SC4 pin these cases without clamping an invalid identity.

| Ledger state | Accepted next record | Otherwise |
| --- | --- | --- |
| Empty | Exactly one owner-bound Opened | Lifecycle refusal |
| Open | Started with next call_seq; or Closed with no unresolved calls | Lifecycle refusal |
| Calls begun | Terminal for the matching begun call/tool; additional Started only for an already decoded bounded local refusal | Lifecycle or specific owner/tool/conflict refusal |
| Closed | End of file only | Lifecycle refusal |

Opened has record_seq 1. Terminal cannot precede Started, match another owner,
or occur twice in one ledger; conflicting duplicates have their specific cause.
Closed requires every accepted call terminal and carries SessionDisposition::Clean
or Failed { cause: BrokerSessionFailure }, a closed typed safe-cause vocabulary.
Latch the first fatal box-admission/readiness/handoff/exec or
initialization/version/protocol/response-limit/timeout,
unsafe-correlation or ledger-limit cause. Orderly EOF and successful harness
output cannot clear it. Ordinary child tool errors and recorded local denials
alone leave session disposition clean; their call outcomes remain unchanged.
Reserve and sync closure before normal exit. Failed persistence preserves only
the verified prefix and its existing missing/partial/lifecycle cause, without
inventing a specific I/O diagnosis during recovery. Missing Closed is a lifecycle
failure after process settlement, even if zero calls were observed. An incomplete
call still yields interrupted evidence; it cannot certify a successful session.
A partial live append is never inspected as corruption: folding is settlement-only.

Reserve retention before forwarding using CR3's 256 MiB attempt-wide shares
and 8 MiB per-call reservation. Include possible fallback slots up front; unused
shares are not dynamically redistributed, and reconnect never creates quota.
Charge actual complete bytes only after durable staging and retain the charge
through settlement, including duplicate content. Staging and publication copies
stay within 512 MiB of new response data. Share exhaustion refuses before external
work. Historical content is not evicted and quota failure never silently disables
retention. This bounded static allocation avoids another service or allocator.

Call identity is SHA-256 over an existing canonical typed tuple: native =
attempt/site/instance/provider/measured new-call identity; broker =
attempt/site/instance/server/call_seq. Prefix n- or m- separates domains.
No Debug encoding or truncation. U0 must distinguish resumed historical events
from new activity; history is not restamped as a current call. A measured
single-event native format may use a per-attempt ordinal; otherwise missing
correlation refuses. MCP holding of an abstract capability never enables the
same-named native power: only a selected native binding can do that.

Private Started is fsynced before forwarding; masked staging and Terminal are
durable before delivery. After owned-process settlement, project exactly one
public checkpoint per accepted call: succeeded, failed, refused or interrupted.
Native observed rows remain observations without completion claims. Broker
telemetry is diagnostic only. The pure view reads one outcome directly, without
reassembling public stages. The cost is delayed authoritative MCP visibility.

The engine iterates verified ledger records from disk with bounded memory at
the existing checkpoint settlement barrier. Use a fenced append returning an
actual commit result; do not send broker rows through Checkpoints::offer's
lossy/held progress queue. Before retry, compare committed call_id and full typed
payload; confirmed identical rows skip, conflicts refuse. Existing append
patiences govern contention and failed/indeterminate settlement. Durable ledger
position is not an authoritative cursor; restart reconstructs from the journal.
No terminal attempt result of any kind bypasses this barrier. It also judges
validated session dispositions, before admitting any harness result. A Failed
session overrides harness success using existing failed/indeterminate handling
and the exact recorded cause. Clean closure is necessary, not sufficient for
success. Zero-call failures affect the attempt without inventing a tool row;
no new public schema field or phase-machine input is needed.

The deterministic harness fixture must deliberately ignore broker failures and
report success for wrong version before calls, protocol failure after one
terminal call, and exhaustion after exactly 4,096 calls. Assert the respective
fixed cause and 0/1/4,096 checkpoint counts, plus healthy zero-call and ordinary
tool-error/denial controls. Removing only the engine disposition judgment must
fail them; process cleanup removal is separate evidence. Restart rechecks the
durable outcome and never repeats the external action.

| Failure window | Recovery and result admission |
| --- | --- |
| Before durable Started | No forwarding; persistence failure blocks the call and successful attempt. |
| After Started without durable Terminal | Settle processes, append one interrupted row without digest; an external effect may have happened. |
| Staging without Terminal | Orphan bytes prove no result, no inferred digest. |
| Terminal before harness delivery | Publish the child's prepared outcome, not a claim of receipt; no replay. |
| Publication before checkpoint | Verify/reuse artifact, append one absent settled row. |
| Journal commit before acknowledgement | Full payload match skips duplicate; no cursor recovery is needed. |
| Lock outlasts append patiences | Preserve disk evidence, take existing failed/indeterminate settlement, never admit clean completion. |
| Valid Closed with Failed disposition | Preserve exact safe cause and call rows; reject harness success, including zero calls; no synthetic tool checkpoint. |
| Invalid owner/order/gap/partial tail/missing ledger | Keep verified prior calls; interrupt unresolved calls only after settlement; retain the exact CR3/CR4 cause and block success. |

The same barrier covers single seats, all panel members, sequence steps,
stop/timeout/failure and engine restart. An unexpected ledger is an integrity
failure; a missing ledger differs from a proven never-started process. No
child effect is replayed to repair evidence. The engine alone writes journal
rows and the existing phase machine alone chooses the next phase.

### D8. Charter check, native attribution and inspect

GP2 owns deterministic paragraph semantics: requested names with safe-name
boundaries, normalized wrapped whitespace, exact DATA clause, no headings or
fenced code satisfying prose. One qualifying declaration paragraph per requested
capability suffices; later references pass without repeating the clause. Scan
verified bytes once for each loaded office
and inline requester. Test fences, CRLF/blank lines, repeated paragraphs and
name prefixes. Do not add an NLP classifier or claim it controls model behavior.

Normalize observations at the harness edge before the existing 80-character
display clamp. Use selected sealed holdings for attribution; native inventory
alone describes known tools but grants none. Reject ambiguous reverse mapping
at compile and unheld known calls at runtime. Engine-owned site/instance/
boundary stamps are applied together. Missing MCP telemetry fields limit
diagnostics but cannot erase the authoritative broker ledger; missing native
identity blocks a completeness claim with CC2's exact cause.

For discovery, U7c projects only this selected intent's server/tool identifiers
and measured adapter hands.notice discovery identifier into the existing
spawn-mark path. Clear stale primary/previous holdings on every selection,
including wanted drops. Do not reconstruct them from emitted argv or expose
the plan, secret-store or ledger locator. U7d parses the new typed edge once
and renders fixed guidance in the existing capability contract; the workspace
notice's contract stays unchanged. Configuration consumes the selected facts
before rendering lands. Separate selection and rendering tests/removals bind
Codex deferred discovery, fallback clearing, hostile prose and no-MCP/native
controls, all outside the requested-effect digest. No new notice registry or
parallel prompt carrier is needed (0071 rulings 3, 5, 6).

One pure brokkr-view projection exposes each settled call's attribution
and optional digest. CLI/TUI/browser can paint that common fact without
re-deriving capability ownership. No new comparison behavior is introduced.
The thin selection/printing handler stays in cli/verbs/readouts.rs beside
inspect; filesystem verification remains in runtime and attribution in view.
Native lost/stranded checkpoint failures remain enforced independently; an MCP
ledger cannot reconstruct missing native observations.
Inspect adds --capability-call <call_id> to its existing run selector; it finds
exactly one journal-owned call, selects that checkpoint's digest, and opens the
verified bytes via the runtime artifact reader. With --json it emits a
provenance envelope plus the verified canonical response; otherwise prints
provenance and those bytes. Unknown call selection says
"capability call is not recorded in this run". SC4/CR5 own absent/missing/
corrupt causes; no caller-supplied path or fresh server response is accepted.

### D9. Unit boundaries, consumers and merge safety

The last section is the only executable merge order. Each row is one PR,
not an objective with hidden follow-up edits. Shipped JSON, a new schema,
its embedded copy, module registration, extraction source and destination
all count. Tests and report-only evidence are separate. A newly factored
module is used by an existing production caller in the same PR; no unused
public staging API is added. The broker command is a consumer even while
realm compilation remains fenced; it still requires a bound engine plan.
U6b–U6e, including U6c2–U6c8, implement consumed parsing/validation and bounded private session seams,
but the public handler refuses before spawn until masking, ledger and admission
protections are complete in U6f. A directly invoked CLI cannot bypass this just
because realm compilation is fenced. Keep incomplete session seams private;
no exported “not implemented” scaffolding counts as a production consumer.

U1, U2, U3 and U4 have no dependency on one another. U1/U4 require their U0
evidence; U2/U3 do not. The listed order serializes their merges and conflicts,
not their logic. Later feature units explicitly join their outputs. U5c's
versioned grant change and U6a's injector extraction also have independent roots.
Within box preparation, U6c3's hands-only extraction is independent and
U6c7 needs only landed U6a; the chosen order serializes them. U6c4 joins
the session and builder, and U6c8 joins bootstrap and store reader.

The merge order has **54 PRs**: the prior 47 rows plus U6c2–U6c8.
The returned council's EOF/status and writer-lifetime criteria refine the
existing U6c6/U6c8 and U8a2 responsibilities; they add no file, row or task ID.
Their integration controls remain U6f/U8e/U9b's, with SD4 measuring all source
observations rather than hiding engine preparation outside the total.
The returned source-policy repair stays in U6c4/U6c5's existing three-file
budgets; new measurement task 28.17 belongs to U6c5 and adds no production
file or PR. U6f/U7c/U8a2/U9b consume and prove the same amended facts.
U6c restarts from current main; both held attempts are reference only.
Its former fourth-file visibility exception is not carried forward: U6c
counts the shared protocol module and its lib.rs registration, and U6c2 separately extracts the session.
U6c3–U6c5 count shared construction and source observation; U6c6 counts
bootstrap dispatch, U6c7 makes room for typed descriptor-bound store reads,
and U6c8 owns the confined handoff. Each cut has an immediate consumer.
 Shared-file combinations U5a/U5c/U6f/U7d/U9b/U10a
retain their consumers and proof ownership; U4a/U4b and U8a/U8a2 stay separate
because their unions exceed three production files. Stable task IDs identify
work, not numeric merge order. No fourth file or gate exemption is implied.

U1f2 sits between U1f and U1g. Extract generated adapter declarations
from the 1,895-line init.rs into a consumed init/adapters.rs, including its
registration in init.rs, and migrate Claude/Codex/dsh MCP metadata from the
U0-qualified declarations. Exact parity tests bind required generated copies
to adapters/*.json; no independent support vocabulary or fabricated evidence.
The third production file, verbs/setup.rs, replaces its claim about inheriting
"your own settings and MCP servers". Generated agents/README.md instructions
come from init.rs and follow the same rule. Update matching guide transcripts
as documentation in this unit. dsh's separately hired Claude reviewer and
exec's inapplicable model surface retain their own facts. No new grant,
provider preference or resume qualification is introduced.

U1f2 proves metadata and legacy compile compatibility at its merge. U1g
repeats init_doctor/init_stacks against mandatory admission: U0-qualified
fresh scaffolds compile, unsupported or missing evidence yields SI2's exact
site/provider/shape diagnostic. The init self-check uses the existing
`<unmapped>` context and "scaffolded bundle failed to compile" wrapper; compile
from the generated workspace uses realm starter. Tests isolate the first
failing model site and assert the full corresponding diagnostic. Separately
qualify the dsh work candidates and make its Claude reviewer the first failing
site for each SI2 cause; an intake refusal proves no reviewer check. If U0 cannot
qualify a shipped shape, record its refusal; never change an unrelated native
OFF or roster rule to obtain a success.

U4 uses explicitly delayed emission, with this consumer-first handoff:

| Boundary | Producer and consumer state | Required native proof |
| --- | --- | --- |
| U4a/U4b | Store validation extraction then additive v6 dispatch; drivers emit legacy records. | Real compile→driver/process→engine→store append→export/verify remains valid for legacy records and adjacent engine lines; private/partial fields refuse direct append. |
| U4c/U4d | Normalization is consumed internally by existing legacy telemetry lowering; its outward checkpoint remains exactly the old shape. Shared observation types have that production consumer. U4d adds compiled attribution index, consumed by compile checks. | No new checkpoint key or partial identity is emitted; legacy native and local-tool records remain valid across ordinary/inline/fallback/panel/sequence/resume/replacement paths. |
| U4e | Single/panel engine edge decodes the shared observation type once, removes private fields, derives the whole group and preserves legacy records. No shipped producer emits observations yet. | A deterministic driver fixture supplies observations through the real process/engine/store path; exact selected attribution and spoof removal hold. Shipped-driver legacy boundary tests still pass. |
| U4f | Sequence/resume/replacement use the same consumer, with measured history filtering and call correlation; legacy handling remains. | Repeat both legacy and injected-observation paths for steps, selected fallbacks and resumed history; all private fields disappear before append. |
| U4f2 | Only now switch the existing driver serializer to emit normalized observations for U0-qualified formats, with U4b/U4e/U4f installed. No feature flag or public staging API. | Real native compile→driver→process→engine→append→export/verify for every site shape; complete SC4 groups, ordinary legacy records, deduplication and exact missing/unheld refusals. |

The normalization type and encoding have one home in the existing U4c
protocol module. U4c must use it for legacy lowering, not add an unused future
API; U4e consumes the same type at its private wire edge. The process carrier
and store/lib.rs need no edit: engine attribution consumes observations before
Checkpoints::offer. If another production path proves necessary, split and
inventory it before implementation. Private fields are never admitted to v6
as a staging shortcut. A legacy missing group before activation is unrecorded,
not a claim of new attribution. After activation missing required telemetry
fails with CC2's exact cause rather than silently reverting to legacy.

Each U4a–U4f merge runs the native legacy boundary matrix in capability_launch
plus the relevant store/engine/resume suites. U4e/U4f additionally test their
consumers through a deterministic fixture, not by pretending shipped emission
has activated. U4f2 repeats the complete matrix through actual adapters and
pins complete payloads, counts and append/export/verify outcomes. Independently
removing the delay fails the pre-activation legacy assertion; bypassing the
engine consumer fails the final schema boundary; removing selected attribution,
call deduplication or history filtering fails its own exact assertion. Restore
all mutations and record their failures. These prove native handoff under
CC1/CC3/SC4 and SD2; the U9 MCP fence cannot substitute (0071 rulings 3, 9).

New production modules stay within 800 lines; new test modules within 2,000.
Existing oversized files shrink or remain at/below their committed baseline,
with sufficient extraction included in the row before adding behavior.
No new suppression is authorized. The row cannot grow a fourth production
file just because a constructor or registry needs updating: amend the row
into another ordered PR first, retaining its task ownership and all proofs.
Likewise if a U0-qualified mechanism needs files beyond U1's inventory,
split that inventory before implementing it. No partial path is enabled
to satisfy a file limit. A strictness result that makes bundles/self
unseatable cannot waive its compile gate: return upstream for an operator
roster decision before merging U1; do not add a guessed harness exemption.

New tests use the named owning suites. Large old test files may relocate
existing cases into a test-only child file in that same suite to make room,
without production registration changes outside the row. Every actual
test-file move is listed in that unit's evidence and cannot change coverage.
Shared builders and unwind-safe environment guards are mandatory. New
integration suites are named explicitly below; no frozen fixtures change.

### D10. Validation and honest handoff

Every implementation row records requirement IDs, exact assertion/variant,
compiling behavior-removal mutation, intended failure, restored pass and
tested commit. Pin each module's refusal text once and otherwise assert typed
variants; a first failing table row proves no later row. Shared helper tests
do not replace the real compile→launch→broker→fold test at U9b.

Every row owes cargo test --workspace, self compilation, formatting, clippy
and exact coverage. Final candidate also runs locked all-feature tests,
locked all-target/all-feature clippy with -D warnings, self and verify
compiles, strict OpenSpec and diff checks. Namespace and exact coverage run
on a capable host outside the nested box; literal nonzero equality for lines,
branches and logical functions is unchanged. Linux/macOS results and remote
CI name the final head. CI/release/local coverage consume the same
rust-nightly-version.txt. Pending results are never substituted with a
previous head or a docs pass.

Witness/compose pins are remeasured in every row that changes their inputs,
with the reason appended; no invented digest and no historical channel
replacement. The shared pin files are tests, not an exception to source
budgets. Unit records update this change's tasks and later implementation
evidence; planned checks are never recorded as executed proof.

This docs-only amendment runs staged/unstaged diff checks, strict OpenSpec
and typos where available; it changes no tests and claims no Cargo or runtime
proof. Its validation and handoff are recorded in
[evidence/design-boxed-broker.md](evidence/design-boxed-broker.md#council-design-validation).
The returned specify visit also runs the existing decision-ledger test when
available, because SC-1 is a defect that gate owns (0071 ruling 11).
Earlier document validation stays in [evidence.md](evidence.md#repair-validation). Runtime results stay with their
implementation units; a document pass supplies no behavioral evidence.

### D11. Artifact consistency decisions

| Analyze category | Judgment and owner |
| --- | --- |
| Duplication | Earlier specification history remains in evidence.md; this visit uses evidence/design-boxed-broker.md; behavior reasons stay at MB3/MB4 and CR2–CR4. Shared injector/canonicalizer/view and selected discovery remain unchanged (0071 rulings 3, 5, 7). |
| Ambiguity | SI2 specifies fresh-scaffold measured success and exact unsupported/missing causes, including the actual unmapped versus mapped consumer. CC1/SC4 specify legacy versus attributed checkpoint states (rulings 3, 8, 9). |
| Underspecification | D9 and U1f2/U4f2 name migration, consumers, emission activation and intermediate boundary proofs. U0 still decides mechanisms; failure means refusal, not guessed support (rulings 3, 9). |
| House alignment | Each PR has at most three production files; init.rs and resume.rs baselines are inventoried. Typed edges, existing authority, frozen versions, native OFF and Linux/macOS scope stand (rulings 1–10). |
| Coverage | All 26 requirements retain task ownership. New scaffold and emission tasks remain open, as do masking, session and startup controls. Every intermediate U4 merge has native compile-to-journal proof (ruling 9). |
| Inconsistency | Unit rows, dependencies, detailed consumers, task groups and Hot files must agree; archive folds behavioral scenarios and retains evidence with file/section references (rulings 3, 5). |
| Gates | File/function/clone, whitespace, format and coverage findings belong to their gates. The scaffold-migration and emission-order units correct the plan; no implemented regression or weakened refusal is claimed (rulings 4, 9, 11). |

### D12. Council reconciliation for the boxed-server amendment

This records the first council at `35593f8c`, whose robustness and simplicity
positions were read in full. The returned positions at `d53745b7` supersede
those notes and are reconciled in D14; the rejected 51-row sketch below is
historical. Claims are advisory; the ruling, MB3/MB4 and checked source decide. The
durable record is in [this visit's evidence](evidence/design-boxed-broker.md#council-reconciliation).

| Claim / position | Disposition and evidence |
| --- | --- |
| Both: shared construction, bounded installed layout, no analyzer/installer/daemon | Adopt. `box_argv` already has two callers and the single system list; frozen v1 has no package-root field. D5 keeps those consumers and allows absent dependencies to fail inside (0071 rulings 2, 5, 6, 10). |
| Robustness: canonical paths and nlink alone miss bind aliases and path replacement | Adopt. Mount roots/subpaths plus inode identity, bounded no-follow observation and actual descriptor mounts are required. System sources receive the same protection; unknown identity refuses (rulings 3, 8, 9). |
| Both: waiting bootstrap and private binding pipe | Adopt. `--clearenv` is too late to protect the host dynamic loader if Command already has loading bindings. Readiness precedes lookup; `bind_environment` remains the sole accessor, with a counted protocol API and receiver (rulings 3, 5, 8–10). |
| Simplicity: keep the entire bootstrap in session.rs and use only four new rows (51 total) | Reject that cut, retain its minimal-service intent. `mcp` and injector visibility are private, secret.rs is 706 lines, and the bootstrap/control observer would accumulate responsibilities. Seven added rows explicitly count module visibility, consumed extractions and a bootstrap module; the 54-row table is authoritative (rulings 4–6, 10). |
| Robustness: source observer, separate bootstrap and possible store extraction | Adopt with concrete ownership. U6c5 names the bounded Linux observer, U6c6 registers bootstrap, U6c7 consumes typed store extraction before U6c8 connects the descriptor reader and injector. The consumed protocol/broker module also gives U6d a reachable ledger namespace without a fourth visibility file. No fourth-file exception or mirrored egress enum (rulings 2–6, 8). |
| Both: allow valid loading names only in confined exec; retain fixed-key checks | Adopt. The injector overwrites keys; shared grammar already rejects `_JAVA_OPTIONS`. One fixed table protects keys, and no semantic loader denylist survives (rulings 3, 5, 9). |
| Robustness: missing store and same-handle reading need explicit decisions | Adopt conservative refusal even for no bindings, with a protected empty-store control. The present resolver skips empty names and reopens a pathname; neither proves excluded identity. MB4 scenarios and U6c7/U6c8 own the remedy (rulings 3, 8, 9). |
| Both: keep native-read qualification, all-writer evidence protection and durable zero-call failures | Adopt. R2 does not box the harness and process.rs consumes harness status; U7/U8/U9 retain their independent duties and exact typed causes (rulings 1, 3, 7–9). |
| Both: #403 cleanup without new supervision or stronger claims | Adopt. Omit --new-session; preserve the explicitly recorded #472 residuals. Real Linux process evidence and macOS refusal remain future proofs (rulings 6, 9, 10). |

Bounded traversal and handoff, absent-store refusal and system-link controls
are encoded as MB3/MB4 scenarios before this design. No earlier accepted
ruling is rewritten, no unresolved upstream fault is hidden downstream, and
no runtime HIGH is claimed closed by a document. D11's consistency judgment
is qualified by this visit's document audit and pending implementation gates.

### D13. Returned review dispositions

The returned chief's C1/C2, S1–S3 and SC-1 are answered at their earliest
owners, before this design and tasks. All three review positions were read
in full as well as both original council positions. The earlier D12 adoption
of blanket system-link refusal is superseded only by the distinctions below;
its reasons remain historical, not an additional rule to enforce.

| Finding / claim | Decision and evidence |
| --- | --- |
| C1, medium correctness: ordinary system links and `/usr` package root defeat positive controls | Adopt. MB3 now distinguishes singleton shared-bin entries from whole package trees and admits multiply-linked system support only under a complete kernel write-exclusion proof. The review saw 146 linked `/usr` entries; this seat's boxed survey also found linked support files. Reject uid-only or blanket root-owned relaxation of program files: the commission requires every regular program file singly linked (0071 rulings 3, 8–9). U6c4/U6c5 own it, U6f/U9b repeat it. |
| C2, low correctness: no measured observer/startup budget | Adopt. The descriptor-metadata survey is partial evidence only. SD4 sets Linux profiles, cold/warm samples, 10 s observation and 20 s full-startup qualification budgets within the existing 30 s hard bound. New 28.17 owns real observer measurement; U6f/U9b own full-path measurement. No cache, skipped sources or relaxed refusal; missing measurements block qualification (ruling 9). |
| Additional source feasibility observation | The broad certificate bind reaches unreadable `/etc/ssl/private`. MB3 narrows the server projection to public `/etc/ssl/certs`, preserving hands' source set and all actual-source refusal checks. This removes unneeded authority instead of skipping unreadable bound bytes (rulings 5, 9–10). |
| S1, low security: package roots can contain unrelated credentials | Adopt as a trust residual in MB3/D5. A dedicated installation is recommended; otherwise trusted server code sees those bound bytes. Selected-store exclusion and declared-value masking do not cover arbitrary credentials. Do not invent credential scanning or claim a stronger boundary (ruling 9). |
| S2, low security: shared network includes local services | Adopt in MB4/D5. Explicitly name host loopback and Linux abstract Unix sockets and positive/negative sentinels. Reject a loopback ban/allowlist contrary to the supplied egress ruling (rulings 3, 9). |
| S3, low security: current reach cannot authenticate historical installations | Adopt in MB3/D5 and U8a2. Earlier planted bytes remain an operator installation-provenance concern. Keep current/concurrent managed-writer alias exclusion; no historical authentication claim (ruling 9). |
| SC-1, info: amendment note breaks decision ledger | Adopt the gate's finding. Put the one-line ruling link in ordinary Context prose without a malformed header marker or amendment-verb/date target. Preserve accepted status, ruling and test; run the existing ledger gate where available (ruling 11). |
| Correctness INFO: prepared-environment serialization is not proved by accessor count | Retain U6c8's separate leak scans over the actual pipe/launcher boundary. The one-accessor pin proves cardinality only (rulings 5, 9). |
| Correctness INFO: empty store and dist-only layout costs | Retain the deliberate MB3/MB4 rules. Empty protected stores and dedicated package/bin entries are documented controls; no lookup or widening exemption is added (rulings 3, 8–9). |

These are documentary repairs and explicit threat-model limits, not runtime
closure of any held finding. The specification is still proposed and U9b
remains fenced pending every actual host/behavior proof.

### D14. Returned council reconciliation

The journal's `returned_from` is clarify `clear` at `d53745b7`: no ambiguity
or earlier-artifact fault was returned. Read both replacement council positions
in full, robustness and simplicity, against that head and the current source.
Retain D13's C1/C2, S1–S3 and SC-1 repairs and every recorded answer. These
refinements make existing lifetime/framing/failure promises testable; they do
not reopen the operator ruling or claim a runtime finding closed. The durable
record is [the returned council visit](evidence/design-boxed-broker.md#returned-council-design-visit).

| Current claim | Disposition and evidence |
| --- | --- |
| Both: retain shared builder, bounded installation rule and delete startup analysis | Adopt. `box_argv` has two live callers, one system table and mandatory workspace/private-host mounts. U6c3/U6c4 keep the consumers while introducing the separate server profile; missing dependencies fail inside. No second box, package discovery, v2 field or language analyzer (0071 rulings 2, 4–6, 10). |
| Both: retain C1's complete system write-exclusion predicate, unconditional program links and certificate narrowing | Adopt. The returned observations support those distinctions; root ownership alone is not the predicate, and a blanket system-link refusal defeats the required positive controls. MB3/D5 remain the owners (rulings 3, 8–9). |
| Robustness: a later cross-worktree writer can invalidate preparation-time source facts | Adopt as an explicit lifetime proof. The artifact-root lease alone is insufficient. D6 orders admissions through one host lock and the already planned protected inventories, keeping reservations through settlement; MB3 now tests both orders, concurrent starts, engine death and a disjoint control. Reject next-call detection as protection (rulings 1, 3, 8–10). |
| Robustness: a decoded first frame does not prove no trailing frame | Adopt. Require sender close and receiver EOF before exec, with open-pipe, partial-frame and trailing-byte controls in MB4/U6c8. One fixed frame remains sufficient; no negotiation or generic RPC (rulings 3, 5, 8–10). |
| Robustness: distinguish failed exec from failed MCP initialization | Combine with receiver-side handoff reporting on the private close-on-exec status pipe. At most one of two fixed failure tags preserves MB4's handoff cause or MB3's boxed-exec cause; an exec-only tag would lose a late receiver rejection. EOF permits initialization only. Stderr and bootstrap death cannot certify success. U6c8 consumes it and U6f/U8e/U9b repeat stage-specific failures (rulings 3, 8–10). |
| Both: one injector, safe host environment, pre-secret readiness, independent native reads and durable zero-call failure | Adopt without duplication. The crate-private injector overwrites entries, `read_store` currently reopens a path, and the machine proof counts accessor calls only. Keep U6c7/U6c8's descriptor read, fixed-key checks and separate leak scans, plus U8's engine judgment (rulings 3, 5, 8–9). |
| Robustness: count engine observation and broker reobservation; prove cancellation of blocking work | Adopt in SD4/D5 and existing measurement task 28.17, repeated by U6f/U9b. No survey, cached admission, source pruning or timeout knob supplies qualification; metadata bounds do not claim general resource containment (rulings 1, 9–10). |
| Simplicity: retain 54 units, withdrawing its old 51-row sketch | Adopt the current position. Private visibility and the measured hands/secret file sizes still justify the exact extraction cuts. Keep all stable IDs, three-file budgets and U9b-only compilation; no new module or service is needed for these criteria (rulings 4–6). |
| Both: explicit installation/network/cleanup risks and no additional services | Adopt. S1–S3, 0012 transformations and #403/#472 residuals stay named. Reject credential scanners, attestation, destination filters, source caches, public lifecycle events and a separate supervision/registration service; the existing protected inventory owns coordination (rulings 5–6, 9–10). |

No architectural principle is waived. File/function/clone checks remain the
house gates' authority (0071 ruling 11). The final document audit retains all
26 requirements and existing task owners; no task is closed by this visit.

## Risks / Trade-offs

- Harness config precedence may defeat isolation → U0 controls admission,
  and SI2 refuses affected sites even if this makes an existing recipe unusable.
- Secret-bearing native reads may be impossible to isolate → refuse that
  holding; a secret-free control may still qualify. U0 decides, not mode bits.
- Version reporting happens after code executes → admit protected source mounts
  and exec only inside the box; the operator installation is still trusted code.
- Conservative package layout, single-link program files, unproved system
  write exclusion, bounded traversal and a required existing store can refuse
  legitimate installations → use a dedicated protected package and empty store
  for secret-free use; never widen mounts or skip uncertain identity. Shared
  system support permits protected hard links under MB3's full predicate.
- Installation credentials/history and shared-network host services remain
  trust limits → document S1–S3 explicitly; no unknown-secret masking,
  historical authentication or network allowlist is claimed.
- A metadata survey is not a startup benchmark → SD4's cold/warm Linux
  qualification budgets and U6c5/U6f/U9b owners gate positive claims. Large
  installations may still hit the fixed bounds; missing measurements stay pending.
- Descriptor mounts/control-FD carriage require real host proof → missing
  support refuses; document validation cannot establish readiness.
- One box and private tmpfs per server add startup and memory cost → accept
  per-session isolation; no pool, persistent server or unboxed fallback.
- Protected artifacts live under a writable repository ancestor → bound
  handles, read-only mount overlays, alias refusal and adversarial real
  namespace proofs for every managed writer precede enablement. Same-root runs
  serialize, and an unsafe zero-grant writer may now refuse. Source admission
  also orders cross-worktree starts; crash-stale or unprovable reservations
  can refuse a conflicting run until settlement is established.
- Process groups are not cgroups → reuse #403, retain #472/macOS residuals;
  namespace activation requires real Linux cleanup evidence.
- Exact recording costs a sync before each external call → pay it, with
  bounded serial calls and explicit failure; no asynchronous unsafe queue.
- Response persistence and receipt cannot commit atomically → distinguish
  child outcome from receipt, and never replay an uncertain external effect.
- Authoritative broker calls appear only at settlement → retain durable private
  evidence and diagnostics without a second public lifecycle.
- Fixed retention shares can refuse a busy broker while another share is idle →
  accept this predictable bound rather than redistribute authority during a call.
- Shape-preserving masking can refuse a valid JSON response → accept the
  bounded refusal instead of exposing a scalar secret or changing evidence.
- Harness success can hide child failure → persist and judge session outcome;
  process cleanup alone proves neither healthy closure nor successful calls.
- Many source seams are oversized → extract with their consumers first;
  each split is budgeted rather than hidden in one broad wiring PR.

## Migration Plan

0077 is accepted; its new pointer leads to the supplied ruling. Operator rules
on this proposed detailed amendment before further implementation. Every listed PR then
starts from current main, updates changed path/number facts, passes its gates,
is signed and lands through the merge queue. No long-lived implementation
branch is assembled by this document run. U6c starts again from main after
landed U6a/U6b; neither held analyzer nor its past proof claims is merged.
Existing installations need the conservative package layout and excluded
store; macOS keeps its refusal, and no frozen contract migration is added.

U1f2 migrates generated declarations and instructions before U1g. U1 can
refuse previously ambient-configured seats; guides and doctor must not call
unmeasured configurations safe. U3 narrows gate native
holdings, so realms that intend reviewer egress list its office explicitly.
The repository's own grants remain empty. U4 installs record acceptance and
all attribution consumers before U4f2 emits new fields, preserving valid
legacy checkpoints at every merge without rewriting history. U5 adds the next realms version after v7 and v12
identity, without editing frozen contracts. U9b alone enables admitted v1
stdio grants with empty restrictions. Before that migration, quiesce older
Brokkr engines whose write reach could overlap the protected sources or
artifact roots; a different worktree name does not exclude that overlap. Their
launches cannot honor the new managed-writer coordination. New runs protect
existing artifacts even without grants.

Rollback before U9 simply leaves the MCP fence. After U9, an emergency
rollback restores the compile refusal and refuses new starts; it does not
erase ledgers/artifacts, reinterpret an old manifest, replay external calls
or revert native strictness/gate protections. Existing in-flight operations
are settled through the same journal/cleanup path, not left detached.

## Open Questions

No policy ambiguity is left open by this visit. The remaining unknowns are
U0 measurements, implementation feasibility evidence for descriptor mounts/
control-FD readiness/source identity, and external results: which D2
candidate qualifies each harness/shape, observed call identities, and final
host/CI evidence. Their failure outcomes are specified as refusal or pending.
No guessed precedence, decision acceptance or exemption is assumed.

## Hot files

The unit table below is the authoritative complete path inventory. The following
collision map is for concurrent main work; new modules are explicitly included.
At each PR recheck these paths and baseline counts against main.

| File / baseline lines where oversized | Units | Coordination |
| --- | --- | --- |
| `crates/brokkr-protocol/src/native_controls.rs` (4539) | U1a, U1g, U7a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-protocol/src/native_controls/mcp.rs` | U1a, U1c, U7a, U7d, U8a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/agents.rs` (1671) | U1b, U1f, U3b, U7b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/agents/load.rs` (1334) | U1b, U3b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/agents/mcp.rs` | U1b, U7b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/adapters.rs` (7268) | U1c, U4c, U4f2, U6a, U7d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/adapters/mcp.rs` | U1c, U1g, U7d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/claude.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/codex.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/lanetally.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/dsh.json` | U1e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/exec.json` | U1e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/bundle.rs` (7815) | U1f, U3a, U3c, U4d, U5a2 | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/bundle/mcp.rs` | U1f, U7b, U9b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/init.rs` (1895) | U1f2 | Generated adapters and agents/README; keep copied strictness metadata bound to shipped declarations by parity proof |
| `crates/brokkr-cli/src/init/adapters.rs` | U1f2 | Consumed extraction from init.rs; registration included in that parent |
| `crates/brokkr-cli/src/verbs/setup.rs` | U1f2 | Printed scaffold instructions and matching guide transcripts |
| `crates/brokkr-runtime/src/engine.rs` (4862) | U1g, U4a2, U4e, U4f, U7c, U8a2, U8c, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/capabilities.rs` (2369) | U2, U3a, U4d, U5a, U5a2, U5f, U9b | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/binding.rs` | U2, U5a, U5a2, U9a, U9b | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-cli/src/doctor.rs` (1567) | U2, U9a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/capabilities/gates.rs` | U3a | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/agents/charter_data.rs` | U3b, U3c | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/bundle/charters.rs` | U3c | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-store/src/seat_record.rs` (880) | U4a, U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-store/src/seat_record/validation.rs` | U4a | v6 reserved for this slice; preserve append/export/verify dispatch |
| `contracts/seat-record.v6.schema.json` | U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-store/src/seat-record.v6.schema.json` | U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-protocol/src/adapters/capability_calls.rs` | U4c, U4f2 | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/lib.rs` | U4c, U6c | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/capabilities/attribution.rs` | U4d | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/engine/capability_calls.rs` | U4e, U4f, U8d | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/checkpoints.rs` | U4a2, U4e, U8d | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/sequence.rs` | U8a2, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/resume.rs` (1062) | U4f | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-view/src/lib.rs` (2767) | U4g, U8f | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-view/src/capability_calls.rs` | U4g, U8f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/dialect.rs` | U5a | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-core/src/realms.rs` (902) | U5c | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `crates/brokkr-core/src/realms/grants.rs` | U5c | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `contracts/realms.v<N>.schema.json` | U5c | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `contracts/run-manifest.v12.schema.json` | U5f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/manifest.rs` | U5f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-protocol/src/secret.rs` (706 at this visit) | U6a, U6c7, U6c8 | 0012 single-injector proof; no second plaintext accessor |
| `crates/brokkr-cli/src/cli_args.rs` (699 at this visit) | U6b, U6c6, U8g | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/lib.rs` (1918) | U6b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/broker.rs` | U6b, U6c, U6c2, U6c6, U6e | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-cli/src/broker/session.rs` | U6c2, U6c4, U6c5, U6c8, U6d, U6e, U6f, U8b | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-protocol/src/broker/ledger.rs` | U6d, U8b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/broker/rpc.rs` | U6e, U6f | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-cli/src/broker/output.rs` | U6f, U8b | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-protocol/src/hands.rs` (1267 at this visit) | U6c3, U6c4, U7a, U8a | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/broker.rs` | U7c, U8a2, U8c, U8d, U8e | Protected inventory, cross-worktree admission lock and lifetime reservations; #403/#415 cleanup and ongoing splits |
| `crates/brokkr-runtime/src/engine/marks.rs` | U7c | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-protocol/src/hands/evidence.rs` | U8a | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/artifacts.rs` | U8c | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-cli/src/verbs/readouts.rs` | U8g | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/doctor/capabilities.rs` | U9a | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-protocol/src/hands/namespace.rs` | U6c3, U6c4, U6c5 | Count registration and same-unit consumers; recheck size and concurrent main edits |
| `crates/brokkr-protocol/src/hands/namespace/sources.rs` | U6c5 | Includes system write-exclusion/credential observation and task 28.17 measurement; split before implementation if ceilings cannot hold |
| `crates/brokkr-cli/src/broker/bootstrap.rs` | U6c6, U6c8 | Ready state, complete-frame EOF and close-on-exec status; count registration/consumers and recheck ceilings |
| `crates/brokkr-protocol/src/secret/store.rs` | U6c7 | Count registration and same-unit consumers; recheck size and concurrent main edits |
| `crates/brokkr-protocol/src/broker.rs` | U6c, U6d | Shared consumed protocol edge; registration counts in lib.rs |
Also coordinate `crates/brokkr-cli/tests/init_doctor.rs` and
`crates/brokkr-cli/tests/init_stacks.rs` (U1f2/U1g),
`crates/brokkr-runtime/tests/witness_digests.rs`,
`crates/brokkr-runtime/src/bundle/compose_tests.rs` (every identity-changing
row), `crates/brokkr-runtime/tests/frozen_contracts.rs` (U4b/U5c/U5f/U10a),
`crates/brokkr-cli/tests/machine_proof.rs` (U6a, U6c7, U6c8, U6f), the exact owning suites
listed with each unit, and `contracts/README.md` plus U9b's guides.
The current `protocol/process.rs` data carrier, `store/lib.rs` append fence, `protocol/process/tree.rs` and `cli/render.rs` are review seams with no
planned production edits: shared consumers suffice. If they actually need
editing, budget a new split before touching them. Their invariants remain
covered by store, secret, cleanup and readout proofs.

The document-only collision set is this change's evidence/proposal/deltas/design/
tasks and `docs/decisions/0077-the-capability-broker-is-a-harness-child.md`.
The already adopted index row in `docs/decisions/README.md` and reciprocal
pointer in `docs/decisions/0065-capabilities-are-the-realms-to-grant.md` remain
part of the commission and must be preserved. R1–R5 and every operator addendum remain verbatim. The only new visit
record is evidence/design-boxed-broker.md; historical evidence.md is untouched.
U10a's final document fold additionally touches the seven living files
`openspec/specs/strict-mcp-isolation/spec.md`,
`openspec/specs/mcp-capability-broker/spec.md`,
`openspec/specs/gate-capability-policy/spec.md`,
`openspec/specs/capability-call-checkpoints/spec.md`,
`openspec/specs/capability-response-retention/spec.md`,
`openspec/specs/slice-two-contracts/spec.md` and
`openspec/specs/slice-two-delivery/spec.md`, and moves this change to the
actual dated directory under `openspec/changes/archive/`. These are future
document paths; the fold remains pending until implementation is complete.

## Slice two units

One row is one PR, from main, signed and through the merge queue after the
operator rules. Follow this order. Dependencies are semantic prerequisites,
not simply the preceding row; the independence of U1–U4 is explicit in D9.
No row through U9a lifts the MCP compile fence. U9b is the sole enabling PR.
The lettered splits are required by the checked seams: record schema plus
embedded copy already consumes two files; multi-server wiring crosses runtime,
protocol and adapter data; storage/folding/inspect crosses CLI, protocol,
runtime and pure view. Minimal ledger writing is deliberately pulled into U6.

`contracts/realms.v<N>.schema.json` denotes one exact future file allocated
as **the next realms version after v7** at U5c, including v7's fields. It is
the only intentionally deferred filename number. Every other production path
below is literal. New paths are proposed modules/schemas, not files created
by this document plan. Constructors/registration are counted in their row.
Future tests extend the named suites; capability_broker already exists under
the consolidated CLI integration binary. New capability_artifacts and runtime
capability_ledger/capability_broker_launch modules are registered through the
existing test-only roots (CLI tests/it.rs and runtime tests/it.rs), not new
undeclared binaries. Tests may use child modules of the owning suites to stay
below 2,000 lines; list every test move in unit evidence.

Each row closes exactly its numbered tasks, including tests/mutations and
gate evidence. Test lists below are the allowed owning suites; every row
also updates this change's tasks and later implementation evidence. Measured
witness/compose pins accompany only rows that change their inputs.

| PR | Dependencies | Objective and task IDs | Production files (maximum three) |
| --- | --- | --- | --- |
| U0 | Independent | Measure isolation and telemetry; 1.1–1.2 | None |
| U1a | U0 | Extract existing MCP transport checks; 2.1–2.2 | `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U1b | U1a | Type adapter MCP facts; 3.1–3.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/load.rs`; `crates/brokkr-runtime/src/agents/mcp.rs` |
| U1c | U1b | Build isolated serving configurations; 4.1–4.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U1d | U1c | Record Claude, Codex and LaneTally declarations; 5.1–5.2 | `adapters/claude.json`; `adapters/codex.json`; `adapters/lanetally.json` |
| U1e | U1d | Record dsh and exec declarations; 6.1–6.2 | `adapters/dsh.json`; `adapters/exec.json` |
| U1f | U1e | Thread independent strict intent; 7.1–7.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/bundle.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U1f2 | U1f | Migrate generated declarations and instructions; 7.3–7.4 | `crates/brokkr-cli/src/init.rs`; `crates/brokkr-cli/src/init/adapters.rs`; `crates/brokkr-cli/src/verbs/setup.rs` |
| U1g | U1f2 | Seal and enforce every launch; 8.1–8.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs` |
| U2 | Independent | Remove both native-binding panics; 9.1–9.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs`; `crates/brokkr-cli/src/doctor.rs` |
| U3a | Independent | Apply gate classes; 10.1–10.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/gates.rs`; `crates/brokkr-runtime/src/bundle.rs` |
| U3b | U3a | Check loaded office charters; 11.1–11.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/load.rs`; `crates/brokkr-runtime/src/agents/charter_data.rs` |
| U3c | U3b | Check inline requester charters; 12.1–12.2 | `crates/brokkr-runtime/src/bundle.rs`; `crates/brokkr-runtime/src/bundle/charters.rs`; `crates/brokkr-runtime/src/agents/charter_data.rs` |
| U4a | U0 | Make room for additive record validation; 13.1–13.2 | `crates/brokkr-store/src/seat_record.rs`; `crates/brokkr-store/src/seat_record/validation.rs` |
| U4a2 | U4a | Erase driver-supplied attribution at the engine edge; 13.3–13.4 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/checkpoints.rs` |
| U4b | U4a2 | Publish and consume seat-record v6; 14.1–14.2 | `contracts/seat-record.v6.schema.json`; `crates/brokkr-store/src/seat-record.v6.schema.json`; `crates/brokkr-store/src/seat_record.rs` |
| U4c | U4b | Normalize calls while retaining legacy emission; 15.1–15.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/capability_calls.rs`; `crates/brokkr-protocol/src/lib.rs` |
| U4d | U4c | Bind attribution to compiled holdings; 16.1–16.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/attribution.rs`; `crates/brokkr-runtime/src/bundle.rs` |
| U4e | U4d | Stamp single and panel calls; 17.1–17.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs`; `crates/brokkr-runtime/src/engine/checkpoints.rs` |
| U4f | U4e | Bind sequence and resumed observations; 18.1–18.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/resume.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs` |
| U4f2 | U4f | Activate native observation emission after all consumers; 18.3–18.4 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/capability_calls.rs` |
| U4g | U4f2 | Derive call evidence once; 19.1–19.2 | `crates/brokkr-view/src/lib.rs`; `crates/brokkr-view/src/capability_calls.rs` |
| U5c | Independent | Extract grants and mint the versioned veto; 22.1–23.2 | `crates/brokkr-core/src/realms.rs`; `crates/brokkr-core/src/realms/grants.rs`; `contracts/realms.v<N>.schema.json` |
| U5a | U2, U5c | Extract typed dialect policy and bind retention; 20.1–21.2, 24.1–24.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/dialect.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U5a2 | U5a | Compare MCP egress with the bundle's binding minimum; 24.3–24.4 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs`; `crates/brokkr-runtime/src/bundle.rs` |
| U5f | U5a | Publish manifest v12 with its live native consumer; 25.1–25.2 | `contracts/run-manifest.v12.schema.json`; `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/manifest.rs` |
| U6a | Independent | Share the one secret injector; 26.1–26.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/secret.rs` |
| U6b | U5f, U6a | Introduce the broker command as a closed handler; 27.1–27.2 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/lib.rs`; `crates/brokkr-cli/src/broker.rs` |
| U6c | U6b | Define and consume the bound plan; 28.1–28.2 | `crates/brokkr-protocol/src/lib.rs`; `crates/brokkr-protocol/src/broker.rs`; `crates/brokkr-cli/src/broker.rs` |
| U6c2 | U6c | Extract the consumed broker session; 28.3–28.4 | `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6c3 | Independent | Extract the shared namespace builder; 28.5–28.6 | `crates/brokkr-protocol/src/hands.rs`; `crates/brokkr-protocol/src/hands/namespace.rs` |
| U6c4 | U6c2, U6c3 | Consume the server namespace profile; 28.7–28.8 | `crates/brokkr-protocol/src/hands.rs`; `crates/brokkr-protocol/src/hands/namespace.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6c5 | U6c4 | Bind and verify host source identity; 28.9–28.10, 28.17 | `crates/brokkr-protocol/src/hands/namespace.rs`; `crates/brokkr-protocol/src/hands/namespace/sources.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6c6 | U6c5 | Register the private waiting bootstrap; 28.11–28.12 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/bootstrap.rs` |
| U6c7 | U6a | Extract the shared typed store reader; 28.13–28.14 | `crates/brokkr-protocol/src/secret.rs`; `crates/brokkr-protocol/src/secret/store.rs` |
| U6c8 | U6c6, U6c7 | Wire the confined environment handoff; 28.15–28.16 | `crates/brokkr-protocol/src/secret.rs`; `crates/brokkr-cli/src/broker/bootstrap.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6d | U6c8 | Establish durable ledger records before calls; 29.1–29.2 | `crates/brokkr-protocol/src/broker.rs`; `crates/brokkr-protocol/src/broker/ledger.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6e | U6d | Serve the filtered protocol; 30.1–30.2 | `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-cli/src/broker/rpc.rs` |
| U6f | U6e | Mask output and prove complete session cleanup; 31.1–32.2 | `crates/brokkr-cli/src/broker/rpc.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-cli/src/broker/output.rs` |
| U7a | U1g, U5f, U6f | Represent the complete server set; 33.1–33.2 | `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-protocol/src/hands.rs` |
| U7b | U7a | Consume adapter carriage and selected holdings; 34.1–34.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/mcp.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U7c | U7b | Provision protected per-attempt plans; 35.1–35.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/marks.rs` |
| U7d | U7c | Deliver checked configurations and selected discovery; 36.1–37.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U8a | U7d | Protect artifact paths in workspace hands; 38.1–38.2 | `crates/brokkr-protocol/src/hands.rs`; `crates/brokkr-protocol/src/hands/evidence.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U8a2 | U8a | Protect every managed writer before dispatch; 38.3–38.4 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/sequence.rs` |
| U8b | U8a2 | Stage retained responses before delivery; 39.1–39.2 | `crates/brokkr-cli/src/broker/output.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-protocol/src/broker/ledger.rs` |
| U8c | U8b | Publish verified content-addressed artifacts; 40.1–40.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/artifacts.rs` |
| U8d | U8c, U4g | Fold settled calls through confirmed append; 41.1–41.2 | `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs`; `crates/brokkr-runtime/src/engine/checkpoints.rs` |
| U8e | U8d | Settle and recover every attempt's evidence; 42.1–42.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/sequence.rs`; `crates/brokkr-runtime/src/engine/broker.rs` |
| U8f | U8e | Expose retained evidence in the pure view; 43.1–43.2 | `crates/brokkr-view/src/capability_calls.rs`; `crates/brokkr-view/src/lib.rs` |
| U8g | U8f | Open a cited artifact through inspect; 44.1–44.2 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/verbs/readouts.rs` |
| U9a | U8g, U2, U3c, U5a2 | Prepare whole-plan MCP doctor reporting; 45.1–45.2 | `crates/brokkr-cli/src/doctor.rs`; `crates/brokkr-cli/src/doctor/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U9b | U9a | Enable the proved namespace path with guides; 46.1–47.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U10a | U9b | Audit removals, validate, fold and hand off; 48.1–49.2, 49.3 | None |

### U0 — Measure isolation and telemetry

Execute D2's per-harness ambient sentinels, strict config, Codex discovery/event and dsh loading matrix, including native historical-replay detection and separate store/process canary read-isolation controls. Record adapter evidence only; no code changes.

Closes tasks 1.1 and 1.2; requirements [MB2](specs/mcp-capability-broker/spec.md), [SI1](specs/strict-mcp-isolation/spec.md), [SD1](specs/slice-two-delivery/spec.md).
Proof: Positive controls and each cold/resume/replacement shape are reproducible; missing read isolation refuses secret-bearing holdings, while secret-free eligibility is assessed separately.

Evidence/documents only; do not add behavior-mirroring tests for this row.

Documents/evidence: `docs/evidence/adapters/slice-two-mcp-isolation.md`, `docs/evidence/adapters/slice-two-mcp-observations.json`.

### U1a — Extract existing MCP transport checks

Move the current single-server transport/parser checks into the named module, retaining production callers and exact behavior; create room under the existing file baseline.

Closes tasks 2.1 and 2.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Unchanged hands-only exact-state and authored-option tests, with no added server acceptance.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U1b — Type adapter MCP facts

Extract McpSupport and its loader into agents/mcp.rs; type measured/unsupported/unmeasured ambient isolation, native-write confinement and store/process read isolation separately by invocation shape. Consume at load; legacy server maps grant no authority.

Closes tasks 3.1 and 3.2; requirements [MB2](specs/mcp-capability-broker/spec.md), [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Closed decoding, absent evidence and wrapper-specific results have exact variants; a read-only flag never supplies a secret-read proof.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1c — Build isolated serving configurations

Factor existing config assembly into adapters/mcp.rs and implement only U0-qualified isolation shapes. Typed engine input crosses the private serving edge; the current no-broker plan is empty or hands-only.

Closes tasks 4.1 and 4.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md).
Proof: Exact cold/resume/replacement configuration, auth/session controls and missing evidence refusals; the module is used by existing launch builders.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U1d — Record Claude, Codex and LaneTally declarations

Populate declarations from U0 with exact evidence scope; unsupported/unmeasured is a valid outcome, never guessed support.

Closes tasks 5.1 and 5.2; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Adapter-load and whole-file identity tests pin only observed facts.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1e — Record dsh and exec declarations

Record dsh's separate exclusion/carriage verdict and exec's inapplicable model surface. No plugin or harness is added.

Closes tasks 6.1 and 6.2; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: No inheritance from Claude, no fabricated dsh hands support, and exec remains a script path.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1f — Thread independent strict intent

Extract capability-relevant candidate composition from bundle into bundle/mcp.rs. Carry strict empty/hands intent through candidates and inline site facts independently of emitted config.

Closes tasks 7.1 and 7.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md).
Proof: Primary/fallback and nested sites retain distinct intended sets; equality of authored bytes never supplies origin.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1f2 — Migrate generated declarations and instructions

Extract the generated adapter definitions into init/adapters.rs with immediate
init.rs consumers and registration; keep init.rs at or below its 1,895-line
baseline. Migrate Claude/Codex/dsh strict metadata and limitations from U0 and
U1d/U1e. Bound copied facts by exact parity tests; keep native OFF, empty
grants, dsh's Claude reviewer and unmeasured resume facts. Update generated
agents/README.md prose in init.rs and printed instructions in verbs/setup.rs;
no instruction promises ambient MCP inheritance. These are three production
files, including the consumed extraction.

Closes tasks 7.3 and 7.4; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Each generated provider's applicable strict metadata and limitations
equal its shipped source without requiring equality of stack-specific tools;
legacy compile still works before activation. Under U1g repeat fresh-scaffold
success for each qualified roster/shape/stack, plus SI2's exact unsupported
and missing-evidence diagnostics at init's unmapped and workspace starter
compile paths. Isolate the dsh roster's Claude reviewer as the first failing
site for unsupported and missing evidence, independently of intake. Pin
instruction text once; independently removing generated metadata, drifting a
copied assessment or restoring the ambient-inheritance instruction fails its
intended assertion.
Synthetic test assessments prove plumbing, not live U0 qualification.

Owning tests: `crates/brokkr-cli/tests/init_doctor.rs`, `crates/brokkr-cli/tests/init_stacks.rs`.

Documents/evidence: `docs/guides/quickstart.md`,
`docs/guides/starters/bun.md`, `docs/guides/starters/go.md`,
`docs/guides/starters/node.md`, `docs/guides/starters/python.md`,
`docs/guides/starters/rust.md` (matching generated instruction transcripts).

### U1g — Seal and enforce every launch

Bind U1f facts at dispatch and consume the final checked isolated configuration at all serving builders. Mandatory strict admission activates only after U1f2 migrates every generated declaration/instruction, including no-ask sites. Shrink engine composition by using existing extracted helpers.

Closes tasks 8.1 and 8.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: All SI2 shapes and final isolation removal fail exactly; every shipped and fresh-scaffold compile either passes measured support or reports its exact unsupported/unmeasured refusal, never a filename exemption. Repeat U1f2's init_doctor/init_stacks matrix with strict admission active.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`, `crates/brokkr-cli/tests/init_doctor.rs`, `crates/brokkr-cli/tests/init_stacks.rs`.

### U2 — Remove both native-binding panics

Extract kind-specific binding projection and replace the indexed native binding and doctor expect with exhaustive typed outcomes. Native behavior and the public MCP fence stay unchanged.

Closes tasks 9.1 and 9.2; requirements [SC5](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Direct non-native seam fixtures prove both panic sites fixed; native controls and the old compile refusal are exact.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`, `crates/brokkr-cli/src/doctor/capability_tests.rs`.

### U3a — Apply gate classes

Thread canonical executable SeatClass and stable office once; extract the common pure class check. Writes precedes egress; D4 scope and independent native OFF remain.

Closes tasks 10.1 and 10.2; requirements [GP1](specs/gate-capability-policy/spec.md).
Proof: Exact native reads/writes/egress requires/wants, all nested sites, subtraction and fallback tests; helper-level MCP cases do not bypass compile.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U3b — Check loaded office charters

Apply the shared deterministic DATA checker to verified loaded-office charters. Each requested capability needs one qualifying declaration paragraph; later references need no repeated clause, including for dropped or subtracted asks.

Closes tasks 11.1 and 11.2; requirements [GP2](specs/gate-capability-policy/spec.md).
Proof: Existing multi-capability researcher paragraph and later references pass; missing/deferred/fenced clauses and prefix collisions fail with the exact owning capability.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U3c — Check inline requester charters

Reuse the same DATA checker for verified inline requesters and all executable site forms; do not duplicate the paragraph grammar or derive permission from the reminder.

Closes tasks 12.1 and 12.2; requirements [GP2](specs/gate-capability-policy/spec.md).
Proof: Inline declaration and later-reference positives, missing-clause negatives, verified pin drift and every nested site shape bind the same checker.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U4a — Make room for additive record validation

Extract existing validation functions into a consumed child module; preserve dispatch and export/verify behavior.

Closes tasks 13.1 and 13.2; requirements [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: D9's native legacy compile-to-journal matrix passes at this merge. Historical version and exact refusal tests stay green; the oversized parent shrinks.

Owning tests: `crates/brokkr-store/src/tests.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4a2 — Erase driver-supplied attribution at the engine edge

Split ahead of U4b by operator ruling of 2026-10-05. U4b's council found that seat-record v6 would admit a complete capability-call attribution group supplied by a driver, so a forged `response_sha256` would become sealed journal evidence, before U4e and U4f teach the engine to own that group. Before any checkpoint is appended, the engine removes every CC1 attribution field a driver supplied, at both seams: engine.rs's single-site and panel sinks, through engine/checkpoints.rs. Only the engine can ever write that group. Legacy telemetry passes unchanged, and nothing new is emitted.

Closes tasks 13.3 and 13.4; requirements [CC1](specs/capability-call-checkpoints/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: A deterministic driver fixture supplies a complete forged group through the real process, engine and store path. The appended record carries none of it, under v5 now and under v6 once U4b lands; legacy rows are unchanged; each assertion has a compiling removal.

Owning tests: `crates/brokkr-runtime/src/engine/tests.rs`, `crates/brokkr-runtime/src/engine/boundary_tests.rs`.

### U4b — Publish and consume seat-record v6

Add the public/embedded v6 schemas and consume them in version dispatch. Admit one native observed or broker settled attribution group; public started is invalid. Preserve old-shaped rows and conditional broker turn absence.

Closes tasks 14.1 and 14.2; requirements [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: D9's native legacy compile-to-journal matrix passes at this merge. Exact old/new boundary cases, full group dependencies, digest-state restrictions, no invented turn and explicit started rejection; embedded bytes match their source.

Owning tests: `crates/brokkr-store/src/tests.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

Documents/evidence: `contracts/README.md`.

### U4c — Normalize calls while retaining legacy emission

Extract telemetry normalization at the harness edge into the shared typed observation module. Consume it immediately in existing telemetry lowering, preserving the exact legacy checkpoint shape and behavior: no private observation, new key, partial call identity or attributed group is emitted. Parse measured identity before display clamping; new emission and outward deduplication activate only in U4f2 after every engine consumer. No unused public staging API.

Closes tasks 15.1 and 15.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Claude/Codex/dsh measured fixtures prove normalization internally and exact legacy output externally. D9's native compile-to-journal matrix pins no new field, valid append/export/verify and ordinary checkpoints. A compiling mutation that emits a private observation early must fail the legacy boundary assertion; this is not waived by the MCP fence.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4d — Bind attribution to compiled holdings

Build a typed reverse attribution index from each selected native holding and adapter inventory; compile-refuse ambiguous or unrepresentable names. Extract existing projection logic to keep parents below baseline.

Closes tasks 16.1 and 16.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: D9's native legacy compile-to-journal matrix passes at this merge. Exact selected dialect/tool, long-name and ambiguous-map cases, no substring matching or inventory-only grant.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4e — Stamp single and panel calls

Install the shared typed observation consumer beside current boundary/site stamps. Consume/remove private transport fields before Checkpoints::offer, erase driver authority, derive the full SC4 group and assign attempt-owned native IDs; local calls stay ordinary and known-unheld observations refuse. Preserve legacy input unchanged while shipped drivers still emit it; no producer activation in this PR.

Closes tasks 17.1 and 17.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md).
Proof: D9's legacy matrix plus deterministic driver observations through the real process/engine/store boundary for ordinary, inline, fallback and panel calls. Exact full payloads/counts, no private or spoofed fields, and append/export/verify success are required. Bypassing observation consumption independently fails the schema boundary proof.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4f — Bind sequence and resumed observations

Install that same observation consumer for sequence, eligible resume and replacement, using U0-measured new-call identity and ignoring replayed history. Consume private fields before append and preserve selected fallback ownership. Shipped drivers still emit valid legacy records until U4f2; both paths remain tested.

Closes tasks 18.1 and 18.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: D9's legacy boundary matrix and injected-observation compile-to-journal tests for sequence/resume/replacement assert exact payloads and append/export/verify. Fresh calls stay distinct, history creates no new use, start/completion count once; missing identity takes CC2's exact cause. Independent consumer/history/deduplication removals each fail their assertion.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4f2 — Activate native observation emission after all consumers

Switch the existing adapter serializer from legacy lowering to normalized
observation output only after U4b, U4e and U4f are installed. This edits just
adapters.rs and adapters/capability_calls.rs. The engine alone derives complete
public attribution; no private field or partial group reaches the store.
Unrelated legacy checkpoints stay valid. Missing required identity after
activation takes CC2's refusal, never an unattributed downgrade or guessed ID.

Closes tasks 18.3 and 18.4; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Real native compile→actual adapter→process→engine→fenced journal append,
export and verify for ordinary, inline, fallback, panel, sequence and eligible
resume/replacement. Assert exact full groups/selected owners and call counts,
no observation or forged fields, duplicate start/completion once, distinct new
calls, no restamped history, exact unheld/missing causes and local-tool legacy
controls. Preserve earlier-merge legacy proofs. Independently bypass the engine
consumer and remove selected attribution, deduplication and history filtering;
each compiling mutation must fail its own boundary assertion. No MCP grant or
broker is needed; U9 remains fenced.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`, `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-store/src/tests.rs`.

### U4g — Derive call evidence once

Consume a pure call projection in existing view construction, representing native observations and the new settled broker states without stage grouping or grant lookup.

Closes tasks 19.1 and 19.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Exact observed/succeeded/failed/refused/interrupted values and honest historical absence; no view I/O, clock or authority decision.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U5c — Extract grants and mint the versioned veto

Extract version-aware grant parsing and reserve retain false only in the next realms version after v7, allocated on main in this PR. Preserve all #487 fields, old restriction meanings and existing error text.

Closes tasks 22.1, 22.2, 23.1 and 23.2; requirements [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md).
Proof: Old/new exact round trips, omission/false and invalid veto values, provisional-office compatibility and unchanged frozen pins.

Owning tests: `crates/brokkr-core/tests/realms.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

Documents/evidence: `contracts/README.md`.

### U5a — Extract typed dialect policy and bind retention

Extract the consumed dialect loader while retaining typed v1 connection, version, secret names, retained, egress and sends. Preserve frozen acceptance and the MCP compile fence; URL/argv-reference execution receives only the specified compatibility causes after enablement.

Closes tasks 20.1, 20.2, 21.1 and 21.2; requirements [SC1](specs/slice-two-contracts/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Exact retained fields/digests, duplicate/schema/containment parity, no store/process effects and unchanged native/fenced behavior.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

Make dialect restriction-reservation checks use the grant's version, carry inherit/veto into the typed holding and preserve D11. Bound identifiers without granting secret clearance. The egress-minimum comparison belongs to U5a2 (operator ruling, 2026-10-04): the minimum is parsed and every serving context is built in bundle.rs, which this row does not own.

Closes tasks 24.1 and 24.2; requirements [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md), [MB2](specs/mcp-capability-broker/spec.md).
Proof: Four retention outcomes, legacy retain as restriction, direct reserved-key claims refused, reserved keys absent from restriction validation whatever the schema's composition (operator ruling 2026-10-04), and unchanged inactive grants.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U5a2 — Compare MCP egress with the bundle's binding minimum

Split from U5a by operator ruling of 2026-10-04. Carry the minimum bundle.rs already parses (`parse_egress_minimum`) into capability resolution, and judge each MCP dialect's typed egress, as U5a retains it, against that minimum before provider carriage. A requires refuses and a wants drops with MB4's exact cause, "MCP dialect '<dialect>' has egress '<class>' below binding minimum '<minimum>'". Native holdings and the realm-wide MCP compile fence stay as they are until U9b. Doctor's matching comparison is U9a's.

Closes tasks 24.3 and 24.4; requirements [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Below, at and above each minimum (local, contracted, uncontracted) for requires and wants, an absent minimum, unchanged native outcomes and the unchanged fenced compile, each with a compiling removal.

Owning tests: `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U5f — Publish manifest v12 with its live native consumer

Extract manifest projections and emit v12 native implementation/retention plus the typed MCP shape while MCP still refuses compilation. Keep every consulted and inactive grant identity fact.

Closes tasks 25.1 and 25.2; requirements [SC3](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md).
Proof: Real native compiles validate v12; internal MCP projection is typed; independent identity changes and old manifest reads are exact.

Owning tests: `crates/brokkr-runtime/tests/frozen_contracts.rs`, `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

Documents/evidence: `contracts/README.md`.

### U6a — Share the one secret injector

Move bind_environment into existing protocol/secret.rs with a narrow typed error; existing harness spawns consume it immediately and location comments follow it. Keep exactly one expose_for_spawn production invocation, counting secret.rs too. No new module or lib registration.

Closes tasks 26.1 and 26.2; requirements [MB4](specs/mcp-capability-broker/spec.md).
Proof: The machine proof counts actual accessor calls across all production modules including secret.rs, distinguishes the method definition, and asserts the one injector location. Adding a second call inside secret.rs and separately outside it must fail; existing leak scans and safe diagnostic text remain bound. No environment fallback.

Owning tests: `crates/brokkr-cli/tests/machine_proof.rs`, `crates/brokkr-protocol/src/adapters/tests.rs`.

### U6b — Introduce the broker command as a closed handler

Add Cmd plus handler for broker serve with bounded engine-plan locator/digest arguments. An unbound manual invocation refuses; no raw server argv, grants or secret values are CLI options. Move dispatch code out of the oversized CLI parent.

Closes tasks 27.1 and 27.2; requirements [SD3](specs/slice-two-delivery/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md).
Proof: Exact CLI parsing, bound-plan refusal and unchanged commands; compile still refuses MCP.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c — Define and consume the bound plan

Restart from current main's U6a/U6b; neither held branch is merged. Define the shared closed plan and failure types in protocol/broker.rs, register that public module in protocol/lib.rs, and consume it immediately in CLI broker.rs. Keep native_controls/mcp.rs private for transport checks; this separate protocol edge allows later ledger types to be consumed without another parent visibility edit or duplicate Transport export. The handler validates protected inventory/owner/digest, every plan field and selected box intent, without resolving secrets or spawning. Keep the command's bound-plan and incomplete-serving refusals; no duplicate egress vocabulary or unused public alias. This row deliberately does not create session.rs: its registration/extraction is U6c2.

Closes tasks 28.1 and 28.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SC1](specs/slice-two-contracts/spec.md).
Proof: Exact owner/digest/inventory and closed-shape refusals; altered executable/tree/reach/network/bootstrap identity cannot acquire authority. Native OFF and the realm-wide MCP fence stay exact; counters prove zero lookup and spawn.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c2 — Extract the consumed broker session

Move U6c's admission implementation into session.rs, registered and immediately called by broker.rs. Preserve exact public behavior and all plan checks. Keep plan state private except for the already consumed shared types; the extraction creates room for preparation and later protocol assembly without growing broker.rs beyond its ceiling.

Closes tasks 28.3 and 28.4; requirements [MB3](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: The same bound/unbound CLI and admission assertions pass unchanged. An independent removal of the delegated admission fails its intended assertion; no secret or process effect appears.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c3 — Extract the shared namespace builder

Extract common namespace/mount serialization and generated-identity construction from hands.rs::box_argv into the registered namespace child. Existing execute_in and run_boxed_in immediately consume it through box_argv. Preserve the single HOST_TOOLCHAIN_BINDS owner and workspace-specific workdir/Git/bind orchestration in hands.rs, including overlay_argv's session/RAM difference. Add no unused server variant or public API yet.

Closes tasks 28.5 and 28.6; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Exact workspace and boxed-exec argv/environment/mount-order parity, relative workdir behavior and #504 overlay lifetime controls. Both actual existing callers bind the extraction; hands behavior is unchanged.

Owning tests: `crates/brokkr-protocol/src/hands/tests.rs`, `crates/brokkr-runtime/src/engine/boundary_tests.rs`.

### U6c4 — Consume the server namespace profile

Add the closed server profile and the narrow hands API that session preparation consumes immediately. Reuse the extracted builder and system table. Derive MB3's system-entry/package-entry tree, server-only public-certificate projection, canonical reach checks and MB4's one fixed environment table/reserved set here; construct the full nonsecret server intent with private tmpfs, egress-projected network and no new-session. The public preparation path consumes/checks this intent then retains incomplete-serving refusal before box spawn; it does not claim that identity or readiness is complete yet. Namespace methods keep room for U6c5's observer.

Closes tasks 28.7 and 28.8; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SC1](specs/slice-two-contracts/spec.md).
Proof: Exact singleton system and dedicated package roots (including a system-contained package), certificate narrowing with an unreadable private sibling, relative/root refusals, all reach modes and containment directions, fixed-key collisions with zero lookups, exact fixed environment/network and absence of workspace/Git/overlay/default host private directories. Independent builder and reserved-key removals fail separately; no dummy workspace or second table.

Owning tests: `crates/brokkr-protocol/src/hands/tests.rs`, `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c5 — Bind and verify host source identity

Register the source observer under namespace.rs and consume it in the server profile and broker admission. Implement D5's bounded no-follow observation, mountinfo root/subpath alias comparison, complete source/link/ancestry checks, MB3's system-support kernel write-exclusion predicate and handle-backed --ro-bind-fd inputs. Bind mapped writer credentials and privilege confinement with the source facts; unknown mappings or unsupported ACL facts refuse, and program/bootstrap links have no exception. Secure launcher/bootstrap/control inputs and store identity, including secret-free empty-store handling, without reading values. Use existing protocol rustix/libc dependencies; no hidden Cargo file. Unknown facilities or identities refuse, and all public serving stays closed. The returned owned handles remain live until actual box readiness in the later launch path.

Closes tasks 28.9, 28.10 and 28.17; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [SC1](specs/slice-two-contracts/spec.md).
Proof: Each MB3/MB4 filesystem cause and precedence with zero lookup/start: both overlap directions/modes, resolution chain replacement, unconditional program/bootstrap hard-link refusals, protected system-link positives and independent owner/mode/ACL/mapping/privilege negatives, nlink-one bind alias, nested mount, special/unreadable/cyclic/over-limit data, absent/aliased store, mount-source replacement and native alias creation. Exact-bound positive controls and descriptor-source identity checks each have independent removals. Task 28.17 records SD4's actual observer cost and Linux profiles; a boxed metadata survey cannot close it. Ordinary system entries and protected packages must pass with installed system hard links left intact.

Owning tests: `crates/brokkr-protocol/src/hands/tests.rs`, `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-runtime/src/engine/boundary_tests.rs`.

### U6c6 — Register the private waiting bootstrap

Add a private BrokerCmd variant and exhaustive handler in the existing binary, with registration in broker.rs. The bootstrap consumes sealed nonsecret intent from inherited control descriptors, verifies the actual namespace/mount/tmpfs/network state and emits the bounded ready message. It accepts no store/grant locator as authority and no arbitrary unconfined exec. Absent or invalid private control context takes the box-establishment cause. Its complete waiting/verification handler is a production consumer; actual binding receipt is wired in U6c8, and incomplete serving still cannot start a dialect server.

Keep readiness distinct from the subsequent private binding and exec-status channels specified in D5. This row grants neither a public bootstrap API nor early exec; U6c8 supplies those channels with their consumers.

Closes tasks 28.11 and 28.12; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Real CLI invalid-context refusal, wrong-child/digest/namespace and duplicate/truncated/excess readiness controls, fixed absolute deadline, no marker-only or stdout readiness, safe host loader environment and pre-secret cancellation. Real Linux control-descriptor carriage must be measured; unavailable support refuses with zero lookups/starts.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c7 — Extract the shared typed store reader

Extract store parsing and typed I/O/mode/missing-name causes into a private registered store child, immediately consumed by the existing secret.rs store/resolver functions. Preserve their public text and semantics; legacy String adapters render typed errors only at their existing edge. The reader consumes a file handle and checks that same handle, so U6c8 can reuse the admitted store descriptor. Keep bind_environment in secret.rs. This consumed extraction makes room below secret.rs's measured 706-line starting size; no public descriptor API is staged without a caller.

Closes tasks 28.13 and 28.14; requirements [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Exact existing store/mode/name/UTF-8 and legacy empty-name behavior, one parse/read without an error-classification reread, no ambient fallback and unchanged single-accessor machine proof. Typed variants and text pins bind the extraction and descriptor reader separately.

Owning tests: `crates/brokkr-protocol/src/secret/tests.rs`, `crates/brokkr-cli/tests/machine_proof.rs`.

### U6c8 — Wire the confined environment handoff

Add only the narrow protocol sender/receiver API consumed immediately by session and bootstrap. The sender reads the admitted descriptor after verified readiness, calls bind_environment on a prepared cleared Command and encodes that environment inside secret.rs; receiver validates the same closed frame and builds only the final confined environment. Session consumes the complete preparation/launch operation through the existing command seam; the public incomplete-serving check stays before invoking it until U6f. No second plaintext accessor, encoding catalogue, shell wrapper, file transport or host env binding. Carry typed read errors from U6c7 and keep secret buffers out of Debug/logging. Require a complete binding frame followed by EOF before actual server exec;
close the sender after its one frame and reject any trailing byte. Close all
unrelated descriptors, keeping only the private typed-failure status pipe marked
close-on-exec until exec. Use that pipe for a typed handoff or exec error, never stderr;
EOF authorizes initialization only, and all startup uncertainty stays failed.

Closes tasks 28.15 and 28.16; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Single-injector machine proof plus independent host argv/env/pipe-diagnostic leak scans; exact one-frame limits/name equality, EOF/timeout/NUL/duplicate refusals, zero starts on handoff failure, and protected installed server with DOCS_TOKEN and allowed loading names. Fixed HOME/TMPDIR collisions remain zero lookup/start, while missing/invalid store values and delivery failures are accurately post-readiness/post-lookup. Real namespace and bootstrap consumers prove the handoff; the public fence stays unchanged.
Also bind open-after-frame timeout, truncation, trailing bytes/second frames,
each failure tag, malformed status and bootstrap death at each side of sender completion.
Independently remove the EOF check and status-error propagation. A successful
exec with failed initialization must retain its separate protocol cause;
assert the child's exact environment and `/proc/self/fd` with a purpose-built
fixture rather than compensating for a shell's startup changes.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-cli/tests/machine_proof.rs`, `crates/brokkr-protocol/src/secret/tests.rs`.

### U6d — Establish durable ledger records before calls

Define and consume the shared closed ledger variants and durable writer, with exclusive single plan lifetime, contiguous record/call sequences and reserved terminal/closure capacity. Closed carries Clean or Failed with a latched typed safe cause, synced before normal exit; EOF cannot clear it. Local refusals also have private Started/Terminal.

Open the owned ledger before invoking D5 box preparation. Consume U6c's shared typed failure catalogue for all pre-secret admission/readiness causes and post-lookup delivery/exec failures. Each can close Failed with zero calls; unbound plans create no ledger. Keep first-failure precedence and verified-prefix recovery when closure itself fails.

Closes tasks 29.1 and 29.2; requirements [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Missing/duplicate Opened, Terminal-before-Started, post-Closed records, missing closure, restart of the same plan and each write/sync failure take exact variants; no journal writer. At 4,096 calls the next frame stays unread and closure carries the ledger-limit failure. Wrong-version zero-call and fatal-protocol after-call sessions retain their first cause; healthy zero-call and recoverable call errors may close Clean. Closure write/sync failure preserves verified-prefix recovery, never invented success. Durable local refusals each have one Started/Terminal pair; failed Started persistence admits neither forwarding nor successful completion.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6e — Serve the filtered protocol

Implement the bounded filtered protocol with private Started/Terminal records. Correlate typed response IDs to method/session state, fix absolute deadlines and bound non-response traffic. Fatal protocol/init/version/limit/timeout outcomes latch CR3 session failure; ordinary tool errors and local denials remain call outcomes. Public serving remains closed until U6f; no unsafe intermediate proxy.

The protocol consumes only U6c8's admitted boxed transport. Startup admission, readiness, handoff and initialization use the same absolute startup deadline; list/call limits remain MB3's. No unboxed child, shell or reconnect path exists. Test missing in-box dependencies separately from protocol corruption and from pre-secret refusals.

Closes tasks 30.1 and 30.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Allow/deny/catalog/version/method controls; wrong-type/late/duplicate/phase IDs, endless progress, cancellation, concurrent calls and persistence failures never forward an unrecorded or uncertain retry. Invalid request shapes/vocabulary and 257-byte names refuse before acceptance; a valid 256-byte ungranted name records one exact denial without truncation.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6f — Mask output and prove complete session cleanup

Complete the serving protections in session.rs using existing secret masking and canonical byte/hash functions at the edge. Share one masked buffer, reject duplicate keys and numeric value changes, and independently refuse unsafe scalar/structural secret occurrences before staging or delivery with MB4's exact cause. Keep legacy masker semantics and shared encodings. Drain stderr with raw-byte overlap before lossy decoding. Only then can the bound public session serve; retained plans still refuse until U8b.

Complete the boxed launch path before removing incomplete-serving refusal: exact approved read-only system/package/bootstrap binds, absent workspace/store/evidence, two-server tmpfs separation, each egress class, safe host launch environment and confined binding delivery. Repeat all pre-secret cause counters, singleton system and dedicated native/script/package controls with ordinary protected system hard links left intact, SD4's full-startup budget, missing dependency failures and valid code-loading-name controls. The single-injector machine proof and leak scans cover the actual launcher as well as output; masking and readiness have independent removals. Use installation canaries to prove the stated credential/history limits without claiming authenticity or unknown-secret masking; test host-loopback and abstract Unix socket sentinels for both local denial and shared reach.
Repeat complete-frame/EOF and exec-status controls through the enabled public
session, including bootstrap death and successful exec followed by failed
initialization. SD4's total includes every actual source observation.

Closes tasks 31.1 and 31.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md).
Proof: Literal/encoded/split/multibyte leak scans, digits-only scalar refusal versus text-redaction/unrelated-number controls, masking-created key collisions, and unsafe-correlation failure without raw frames. Assert failed forwarded call with no digest and no unsafe body for retention on/off/veto; the refusal itself passes leak scans. Remove the scalar check independently of numeric-precision validation; direct command cannot bypass plan/ledger/startup/masking protections, and a retained plan never silently degrades. Repeat the independent HOME/TMPDIR collision removals at the final public spawn boundary, including zero lookup/start and normal-binding/private-directory controls.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-cli/tests/machine_proof.rs`.

Exercise #403 process ownership for broker, bubblewrap, waiting bootstrap and server with real host observations. Cover cancellation before readiness, during handoff, initialization and call, failed exec, normal exit and engine restart. Omit --new-session; prove private tmpfs teardown and pipe closure without deleting durable evidence. Make no production tree rewrite.

Closes tasks 32.1 and 32.2; requirements [MB5](specs/mcp-capability-broker/spec.md), [SD4](specs/slice-two-delivery/spec.md).
Proof: Linux real-process positive and cancellation/timeout tests; report existing subreaper/cgroup residuals without claiming them fixed.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-runtime/src/engine/cleanup_tests.rs`.

### U7a — Represent the complete server set

Move hands config helpers into the common typed MCP builder and replace singleton transport intent with exact named server intent. Parse final config independently; preserve authored provenance and native OFF.

Every capability entry still targets broker serve, never the dialect executable or bootstrap. Keep the separate hands and server profiles; exact equality includes the plan digest and environment removals, not just an executable name. This row reuses the builder API without changing the source observer.

Closes tasks 33.1 and 33.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Zero/hands/three-server exact positives and independent missing/extra/changed/counterfeit negatives.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U7b — Consume adapter carriage and selected holdings

Read the formerly dead mcp support at each selected candidate and materialize the engine-owned empty/hands/broker contributions from typed holdings; unsupported carriage keeps requires/wants semantics.

Closes tasks 34.1 and 34.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: No union of fallbacks, dead server maps give no authority, unrepresentable cap names refuse.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U7c — Provision protected per-attempt plans

Provision the single sealed plan inventory and private roots before launch, with complete owner identity and disjoint fixed retention shares including permitted fallback slots. Reuse existing launch facts; do not add a duplicate capability inventory or launch state machine. In marks.rs project and clear selected server/tool/discovery identifiers from the same intent consumed by configuration, using existing adapter hands.notice facts. Carry no plan or storage locator into prompt rendering; parse new edge data once into types.

Prepare box facts through U6c4/U6c5's existing consumed API: effective selected hands/Git roots, source identities/digest, mapped writer/privilege facts, closed system-entry/package-entry tree, trusted bootstrap and store/control exclusions. Project the existing typed egress once into network disposition and bind the already judged clearance receipt to the policy digest. Seal these facts for each selected fallback/member/step without serializing live handles; the broker reobserves before lookup. Runtime observes the host above pure core/view and never uses a display admission function. Drift is a hard failure, including for wants; tests mutate each bound fact independently.

Closes tasks 35.1 and 35.2; requirements [SD3](specs/slice-two-delivery/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [CR3](specs/capability-response-retention/spec.md).
Proof: Substitution/missing inventory/pre-start uncertainty refuse exactly; simultaneous slots cannot share quota, oversized allocation refuses before launch, and resume cannot create fresh budget. Bind selected-fact construction and clearing independently: fallback and wanted drop remove stale facts, original requested digest and native/no-MCP behavior stay unchanged, and prompt facts contain no secret-store/ledger locator.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U7d — Deliver checked configurations and selected discovery

Wire the expanded server set and dialect-secret environment removals into each U0-supported builder and final consumption point; unsupported measured carriers refuse. No modifications after checked command creation. Render fixed capability/tool discovery guidance from U7c's typed selected identifiers in the existing capability contract, reusing the measured hands.notice discovery identifier. No provider-name branch, new notice registry or parallel authority.

Configuration continues to carry only broker locator/digest, with no server values or direct bootstrap entry. Keep D5 box/clearance intent in the protected plan and remove every declared binding name from harness/broker inherited environments. The broker's independent pre-secret source/readiness check is still required after exact config proof; prompt discovery carries no mount/store facts.

Closes tasks 36.1, 36.2, 37.1 and 37.2; requirements [MB4](specs/mcp-capability-broker/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD3](specs/slice-two-delivery/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Actual composed cold/resume/replacement commands match independent literal server intent, and mutations refuse at the final serving boundary. Separately bind rendering and selection for Codex deferred discovery, fallback clearing, wanted drop and hostile prose; preserve hands notice, native/no-MCP prompts and requested-effect digest. No complete plan or storage locator reaches the renderer. Independently prove MB4's removal of exported dialect secret names from harness/broker environments, store-only child bindings and exact requires/wants authentication-collision causes at the serving boundary.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U8a — Protect artifact paths in workspace hands

Carry engine-only protected storage facts, overlay the artifacts directory read-only after writable worktree binds, and reject every conflicting writable/overlay alias by canonical owner identity. No new authored HandsSpec key.

Workspace-specific protected mounts remain in hands.rs::box_argv, which consumes U6c3's shared builder primitives; register hands/evidence.rs in that parent. Neither the common source observer nor server profile needs an edit in this row. Server evidence exclusion remains a separate pre-secret proof, not a substitute for workspace protection.

Closes tasks 38.1 and 38.2; requirements [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md).
Proof: Real namespace writes/ancestor renames/aliases/overlap controls cannot change evidence; mount failure refuses, never omits the protection.

Owning tests: `crates/brokkr-protocol/src/hands/tests.rs`, `crates/brokkr-runtime/src/engine/boundary_tests.rs`.

### U8a2 — Protect every managed writer before dispatch

Carry the protected root to every writer before composition and recheck at the common dispatch door. Hold one exclusive canonical-worktree writer lease per run through owned-process settlement, including no-grant runs; protect historical artifacts and refuse unsafe or uncertain concurrent writers. No new authored protection key or daemon.

Carry all managed writers' effective reach in the admitted protection facts, including zero-grant siblings; any writer able to mutate an admitted server source through an alias fails the source-identity check before server lookup. Server boxing does not waive this coordination or protection of historical artifacts. Other worktrees remain parallel only when their admitted write reach is disjoint from these protected sources. Include their mapped identities/privileges in system write-exclusion admission. This coordinates current and concurrent writers; it does not authenticate earlier installed bytes after a past bind was removed (D5).

Implement D6's atomic admission operation and durable reservations in the
already inventoried engine/broker.rs, consumed at engine.rs's common dispatch
and the sequence path. Reuse the protected inventory and existing settlement
facts; the worktree lease alone is insufficient. Keep the host admission lock
short-lived and owner-bound, protect its root from every writer and refuse
unknown participation. Reservation release follows durable owned-process
settlement, never just engine death or advisory unlock.

Closes tasks 38.3 and 38.4; requirements [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md).
Proof: A retaining panel member plus zero-grant attacker and boxed exec cannot replace root/digest; an already-running writer refuses, abandoned ownership cannot outlive cleanup, later no-grant runs preserve old evidence, and separate worktrees remain independent.
For source exclusion, test A-server/B-writer in both start orders, concurrent
starts and engine death with surviving payload, using both program and system
source aliases and a disjoint-worktree positive control. Assert MB3's exact
incoming-dispatch refusal, unchanged protected bytes and no lookup/start on
a refused incoming broker. Independently remove reservation admission and
settlement retention; a same-worktree artifact test proves neither.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8b — Stage retained responses before delivery

Stage only opted-in masked bytes under the sealed disjoint attempt share; reserve 8 MiB before forwarding, charge actual durable bytes and retain completed charges through settlement. Fsync staged content before Terminal/delivery; veto never writes a body. MB4's unsafe-output refusal occurs before staging, regardless of retention disposition.

The boxed child receives no staging/ledger descriptor or path mount; prove it cannot read or manufacture the retained bytes while the broker writes its legitimate response outside. Namespace cleanup removes no staged evidence needed by settlement.

Closes tasks 39.1 and 39.2; requirements [CR1](specs/capability-response-retention/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Four retention outcomes, multi-broker/fallback exhaustion and exact-bound controls, no same-digest quota reset, stage/sync/terminal/delivery failures, and no silent metadata downgrade; an unsafe scalar response yields the exact failed call without staged/published bytes or digest.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U8c — Publish verified content-addressed artifacts

Use owner-rooted no-follow reads, verify staged bytes and atomically publish immutable digest paths under the operated repository; retain handles across checks.

Closes tasks 40.1 and 40.2; requirements [CR2](specs/capability-response-retention/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Existing-good reuse, collision/mismatch, nonregular/symlink/hardlink and concurrent replacement controls, exact missing/corrupt causes.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8d — Fold settled calls through confirmed append

After process settlement, validate the private lifecycle and project one checkpoint per accepted call through commit-confirmed fenced append. Stream from disk with bounded memory; the committed journal and full call payload determine deduplication. Bypass the lossy held-event queue. Validate Closed disposition independently of call completeness; a latched Failed cause overrides harness success without a synthetic call.

Include every new box/source/store/readiness/handoff/exec cause in the existing closed failure projection. A valid zero-call Failed ledger blocks success with its exact cause and no synthetic checkpoint, independently of call completeness. The validator reuses the shared cause enum; no public journal field or second lifecycle is added.

Closes tasks 41.1 and 41.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md).
Proof: Journal lock across repeated attempts, exhaustion, commit-before-ack crash, duplicate/conflict/gap/partial/missing/unexpected/lifecycle faults, and interrupted calls each have exact counts/outcomes and no replay. A capacity-ending ledger yields exactly 4,096 checkpoints and no invented 4,097th; tool/active-call/budget refusals each yield one refused checkpoint. Wrong-version, post-call fatal protocol and capacity-ending sessions preserve their exact failed-attempt causes despite complete ledgers; healthy zero-call and recoverable-error controls can succeed.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8e — Settle and recover every attempt's evidence

Fold only after owned processes settle and before every ordinary/panel/step terminal result, including failed, cancelled, timed-out and engine-restart paths. Reuse existing append patiences and failed/indeterminate transitions; preserve disk evidence on failure.

Use a success-reporting harness after pre-secret box/readiness failure and post-lookup handoff/exec failure. Assert each exact cause, zero accepted-call checkpoints, settled supervisor/bootstrap and restart persistence without retry. Keep these separate from the existing 0/1/4,096-call version/protocol/capacity controls and from cleanup removals.
Include the private exec-status error, invalid tag and EOF followed by no
functioning child at their observed stage; none can acquire a Clean closure merely
because a pipe ended. Writer reservations stay while settlement is uncertain.

Closes tasks 42.1 and 42.2; requirements [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [MB5](specs/mcp-capability-broker/spec.md).
Proof: Live half-record causes no premature corruption finding; settled half-record fails exactly, every terminal route meets the barrier, and restart appends each call once without replaying external work. A deterministic harness deliberately reports success after fatal broker failure; assert CR4's exact 0/1/4,096 call counts and causes, and catch an independent compiling removal of the engine disposition check. Preserve native lost/stranded failure handling too.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`, `crates/brokkr-runtime/src/engine/cleanup_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`.

### U8f — Expose retained evidence in the pure view

Extend the consumed pure call projection with the settled checkpoint's digest/provenance, without stage assembly, path reads or derived grants.

Closes tasks 43.1 and 43.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Exact historical-unrecorded/no-digest/recorded distinctions and shared display values for every settled outcome; no inferred retention veto.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U8g — Open a cited artifact through inspect

Add inspect --capability-call <call_id>; read the selected run's derived call, use shared runtime artifact reader and emit verified bytes/provenance. Keep the thin selection/printing helper in existing readouts.rs beside inspect; runtime owns file verification and view owns provenance, with no new capability_artifact module.

Closes tasks 44.1 and 44.2; requirements [CR5](specs/capability-response-retention/spec.md).
Proof: Complete exact bytes and missing/no-digest/corrupt/path/foreign-run refusal tests; no refetch and no view I/O.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U9a — Prepare whole-plan MCP doctor reporting

Extract capability reporting into the named module and use the shared complete planner for native/MCP metadata; preserve pre-U9 public compile refusal until U9b. Doctor also reports each MCP grant's dialect egress against the bundle's binding minimum, as U5a2 judges it at compile (operator ruling, 2026-10-04).

Reuse U5a2's below-minimum comparison and requires/wants causes. Report static box eligibility and conservative installation requirements separately from unmeasured host source/readiness facts; doctor does not launch a box/server, read secret values or claim readiness. Linux availability and macOS refusal remain honest.

Closes tasks 45.1 and 45.2; requirements [SC5](specs/slice-two-contracts/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Grant versus holding and retention facts with static scope, native denial and exact no-spawn assertions.

Owning tests: `crates/brokkr-cli/src/doctor/capability_tests.rs`.

### U9b — Enable the proved namespace path with guides

Lift only the global MCP compile fence after all prior proofs, activating D3's namespace, gate, strictness, carriage, secret-read, box admission/readiness, evidence and D11 rules. Quiesce older same-worktree engines before enabling managed-writer coordination.

The enabling matrix also traverses all boxed-server seams: singleton system and user-installed package entries with ordinary protected system hard links intact, SD4's measured host budgets, every reach mode and containment direction, resolution ancestors, hard links and bind aliases, store exclusion including empty/missing stores, bounded observation/readiness/handoff, and actual kernel shebang execution. Pin all pre-secret refusals to zero lookups/starts and distinguish later delivery/exec failures. A success-reporting harness cannot hide any zero-call box failure. Real Linux proof includes private tmpfs per server, local/shared network with hands still network-false, safe host loader environment, allowed loading bindings and cancellation at every startup window. macOS proves its existing refusal; no successful box is claimed there.
Repeat MB3's two-worktree lifetime matrix and MB4's complete-frame/EOF
matrix, with the private exec-status controls and exact descriptor/environment
fixture. Quiesce any older writer whose reach overlaps protected sources,
not just writers using the same worktree. All engine/broker observations
count in SD4's measured total startup cost.

Closes tasks 46.1 and 46.2; requirements [GP1](specs/gate-capability-policy/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Real compile/launch/broker/fold/inspect with fake dialect; exact native/MCP gate reads/writes/explicit-office-egress required/wanted outcomes, same-name MCP holding keeps native power OFF, zero-grant sibling and exec cannot alter evidence, unsafe secret reads refuse, and quota/recovery/cold/fallback/member/step cases bind. Include the success-reporting harness after zero-call version failure, post-call fatal protocol and 4,096-call exhaustion, plus scalar-secret refusal on the real retention/inspect path; pin exact causes, counts and absent unsafe bodies.
Add the separate HOME/TMPDIR collisions through real compile and selected launch, asserting the startup cause, zero store lookup/child start, and normal DOCS_TOKEN/private-directory control. Independent removal of reserved-name validation must fail.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-cli/src/doctor/capability_tests.rs`.

Publish grant/veto migration and measured namespace/stdio/empty-restriction limits, secret-read refusal, conservative installed-entry layout and protected source requirements, delayed settled checkpoints, fixed retention budgets, historical evidence protection and same-root run serialization. Explain the dedicated-installation credential/history limits, shared host-loopback/abstract-socket reach, certificate narrowing, measured source/startup budgets, separate hands/server networks, pre-secret box refusals versus post-lookup failures, private tmpfs, valid loading names only at confined exec, durable session failures, the broker's scalar/structural masking refusal and its fixed startup-key collisions; distinguish broker validation from the unchanged shared secret grammar and legacy mask_json semantics.

Closes tasks 47.1 and 47.2; requirements [SD3](specs/slice-two-delivery/spec.md), [SC1](specs/slice-two-contracts/spec.md), [SC2](specs/slice-two-contracts/spec.md), [SC3](specs/slice-two-contracts/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Guides agree with actual compile/report/inspect evidence and explain quiescing old writers; no new realm grant, MCP server or unmeasured support claim is shipped.

Guide edits require coherence with the enabling proof, not behavior-mirroring tests.

Documents/evidence: `docs/guides/agent-library.md`, `docs/guides/provider-adapters.md`, `docs/guides/recipe-authoring.md`, `docs/guides/secrets.md`, `docs/security-model.md`, `docs/status.md`, `docs/reference/cli.md`, `contracts/README.md`.

### U10a — Audit removals, then validate and hand off

Audit requirement/task/test mapping, each compiling removal/restored pass, no new suppression/clone/unused API and frozen bytes against each unit's main. Repair a missing proof in its assigned suite, not by declaring it proved.

Closes tasks 48.1 and 48.2; requirements [SD4](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: All 26 requirement IDs covered; typed refusal text pins once per module; no frozen fixture regeneration or recording gaps.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-runtime/tests/capability_ledger.rs`, `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

Run the complete D10 validation set on the restored final candidate, obtain external exact coverage and Linux/macOS/remote results naming its head; keep missing results pending.

Closes tasks 49.1 and 49.2; requirements [SD4](specs/slice-two-delivery/spec.md), [SD1](specs/slice-two-delivery/spec.md).
Proof: Literal nonzero covered/total equality for lines/branches/functions, pinned compiler agreement, self/verify compiles, measured identities and signed merge-queue delivery; never push from a seat.

Owning tests: `crates/brokkr-runtime/tests/witness_digests.rs`, `crates/brokkr-runtime/src/bundle/compose_tests.rs`.

As the final task, fold the completed change with the dialect's archive
operation, add the seven living-spec provenance entries and validate the
folded candidate. No skip-specs/no-validation shortcut or premature archive
is permitted; final-head evidence must cover the resulting candidate.

Closes task 49.3; requirements [SD1](specs/slice-two-delivery/spec.md),
[SD4](specs/slice-two-delivery/spec.md) and the
[archive instruction](../../../dialects/openspec/archive.md).
Proof: Actual archive directory/date, seven exact folded requirement/scenario
sets and one appended provenance entry each; strict live/archive validation,
local-link audit, retained evidence.md with file/section links after the move,
no visit history in living behavior scenarios, staged/unstaged whitespace checks and committed document
changes. External final-head gates remain pending until observed.

Documents/evidence: all seven `openspec/specs/<capability>/spec.md` paths
listed in Hot files, the archive-created directory for this change and its
commit/evidence record. This row has no additional production files or
behavior-mirroring tests.
