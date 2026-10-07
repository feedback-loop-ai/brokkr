## Purpose

Serve a realm-held MCP capability through Brokkr's own harness child outside
the seat's namespace, with each dialect server in its own Linux namespace
box, preserving the realm's exclusive authority.

## ADDED Requirements

### Requirement: MB1 the harness launches exactly the engine's brokers

As R1 says, "a granted mcp capability is served by `brokkr broker serve`,
listed in that same engine-written MCP config beside the hands server, one
server per held capability." The engine SHALL derive the set from the selected
site and candidate's sealed holdings. The harness SHALL start the broker;
the broker SHALL start the dialect's real server with piped stdio inside a
separate MB3 box. The broker, bubblewrap supervisor and boxed payload SHALL
remain owned by the attempt under MB5. The engine SHALL not start an
independent unsupervised sibling or ship a third-party capability server.

Each server SHALL be named exactly `cap-<capability>`, with no truncation,
normalization or suffix inferred from model text. Hands remains `brokkr`.
Unrepresentable or colliding identifiers SHALL refuse before launch.
The final check SHALL parse the actual complete command/configuration and
compare executable, arguments, server names, permitted tools and isolation
against independently sealed typed intent, not against a recomposition of
the same output. An authored equal-byte server definition SHALL still refuse.

#### Scenario: Two holdings produce three servers

- **WHEN** a namespace seat holds library-docs and issue-tracker through two admitted MCP dialects
- **THEN** the engine supplies exactly brokkr, cap-library-docs and cap-issue-tracker; each cap entry invokes the current brokkr executable with broker serve and only its own sealed plan; each broker prepares a distinct server box with private HOME/TMPDIR
- **AND** dropping either holding removes that server; an unused grant adds no server

#### Scenario: A counterfeit or missing server refuses

- **WHEN** final assembly adds, removes, duplicates or changes any engine server, including a server with the right name and wrong executable or plan
- **THEN** launch refuses "provider '<provider>' final MCP configuration is not the engine's sealed server set" under the owning site prefix
- **AND** copying valid server argv into an authored command still receives slice one's authored-option refusal, never engine provenance

#### Scenario: A server name cannot be shortened into another holding

- **WHEN** cap plus the exact capability identifier is not representable under the measured adapter's server-name grammar
- **THEN** compilation refuses with cause "MCP server name 'cap-<capability>' is not representable by provider '<provider>'"
- **AND** no truncation, alias or hash-based rename is substituted

### Requirement: MB2 only an actual namespace hands site may hold MCP

R2 says "an mcp grant is admitted only at a site inside a Brokkr box
(boundary `namespace`); every other site refuses it with an exact reason."
After structural grant validity, request/subtraction and office scope,
an otherwise applicable MCP request SHALL refuse outside namespace even
when wanted. Within namespace it SHALL require workspace hands and
network false, a model harness with measured strictness, and supported
adapter MCP carriage. Exec and opaque custom drivers SHALL not gain a
capability channel from being boxed.

Boundary/hands/network eligibility is a hard site refusal under R2.
Native-tool write containment is also mandatory: inability to protect the
plan/ledger/artifacts refuses with "MCP capability '<capability>' cannot
protect broker evidence from this provider's native tools". Provider carriage is ruling 5 compatibility: requires refuses, wants drops
with its exact reason, provided SI2 and native OFF can still hold.
The former realm-wide unbuilt-kind refusal SHALL remain until U9b.
After U9b, valid unused grants stay pinned and inactive; they need no site
or server, consistent with D4. There is no implicit grant or auto-fallback
to another dialect.

After scope and nonempty tools, MB2's hard site checks run before GP1's
class compatibility; GP1 then runs before ordinary provider/connection
compatibility. SI2's independent ambient-isolation check still applies when
a holding drops. New hard-refusal causes, in order, SHALL be:
"MCP capability '<capability>' requires boundary 'namespace'; this site uses
'<boundary>'"; "MCP capability '<capability>' requires workspace hands at
this site"; "MCP capability '<capability>' requires workspace hands with
network false"; "MCP capability '<capability>' requires a model harness with
an engine-owned MCP channel". The evidence-protection check follows those four hard checks. For a dialect
with secrets, a separate measured read-isolation check SHALL then prove that
workspace hands and model-native tools cannot read the bound store or the
child's secret-bearing process state. Failure SHALL hard-refuse with
"MCP capability '<capability>' cannot protect its secret bindings from this provider's native tools". Read-only write confinement and file mode 0600 SHALL
NOT count as read isolation. Secret-free holdings need no secret-read proof. Existing start-time namespace availability
refusals remain in force on both supported hosts.

#### Scenario: Every other boundary refuses by name

- **WHEN** research (office researcher) in private has an applicable library-docs MCP request under harness, open, seatbelt or container
- **THEN** each refuses "seat 'research' (office 'researcher') in realm 'private': MCP capability 'library-docs' requires boundary 'namespace'; this site uses '<boundary>'", substituting the actual boundary
- **AND** both requires and wants obey this refusal; boundary widening is a later decision

#### Scenario: A namespace label alone is insufficient

- **WHEN** that namespace site has no workspace hands, hands with network true, or a boxed exec rather than model harness
- **THEN** it refuses the corresponding complete cause above under the same site prefix, in the stated order
- **AND** a valid model/hands/network-false control remains eligible subject to strictness, carriage and every independent grant rule

#### Scenario: Carriage honors request strength after safety checks

- **WHEN** the otherwise eligible strict provider test-provider declares MCP carriage unsupported with reason "engine MCP loading is unavailable"
- **THEN** requires refuses "seat 'research' (office 'researcher') in realm 'private': requires capability 'library-docs' through dialect 'docs-mcp', but provider 'test-provider' cannot carry MCP (engine MCP loading is unavailable)"
- **AND** wants records "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability 'library-docs' through dialect 'docs-mcp' because provider 'test-provider' cannot carry MCP (engine MCP loading is unavailable)"
- **AND** the dropped candidate starts no broker and independently proves native denial and strict empty/hands-only MCP

#### Scenario: Old exclusion and subtraction still win before site eligibility

- **WHEN** the realm grants only another office, grants empty tools, or the seat subtracts its ask
- **THEN** D4's exact scope/empty-tool/subtraction outcomes remain; no MCP holding or broker is invented
- **AND** invalid grant data remains a realm-wide refusal, even unused; ungranted native equivalents stay OFF

#### Scenario: Nonempty restrictions remain deferred for MCP

- **WHEN** a structurally valid MCP grant has nonempty restrictions and an otherwise eligible site requests it after U9b
- **THEN** requires refuses and wants drops with "MCP nonempty restrictions are deferred to the restriction-transport slice", in GP1's full diagnostic forms
- **AND** an unused valid grant remains pinned/inactive; a child claiming support never bypasses D11

#### Scenario: Native tools cannot forge broker evidence

- **WHEN** an otherwise eligible namespace provider has no proved write confinement for its native tools
- **THEN** the hard refusal is "MCP capability 'library-docs' cannot protect broker evidence from this provider's native tools"
- **AND** neither wants nor a permission prompt replaces the confinement proof

#### Scenario: Read-only native tools cannot qualify a secret-bearing holding

- **WHEN** U0's canary control proves a native tool can read the store or child process environment despite read-only workspace settings
- **THEN** the secret-bearing holding hard-refuses with MB2's secret-protection cause for both requires and wants
- **AND** a secret-free control remains eligible under the other rules; hands and every supported serving shape have independent read-isolation evidence

### Requirement: MB3 the broker offers only granted tools and records refusals

The broker SHALL load an engine-created, identity-bound private plan naming
one capability, dialect identity, connection, exact tools, empty restrictions,
retention policy and attempt ownership. It SHALL not read fresh realm grants
or accept authority from tool arguments, server output, recipe files or an
unbound path in the workspace. Plans and ledgers SHALL be outside writable
seat reach; a changed, missing or forged plan SHALL refuse before contacting
the child, with "broker plan is not bound to this attempt".

The 2026-10-05 ruling requires "the broker spawns each MCP server inside
its own Brokkr box (bubblewrap, the machinery of decision 0043)". The broker
SHALL use the hands namespace builder through a consumed shared builder
profile, with an empty root and no implicit workspace, Git, bundle, declared
hands or host-home mount. It SHALL NOT call the current workspace profile
with a dummy workdir: that profile always grants workspace and Git reach.
Decision 0043's "The boundary is never simulated: no `bwrap`, no tool" holds.
Only Linux bubblewrap is admitted; macOS keeps MB2/R2's refusal.

The fixed read-only system set SHALL use the shared
`hands::HOST_TOOLCHAIN_BINDS` with the server-only certificate narrowing below: `/usr/bin`, `/usr/lib`, `/usr/lib64`,
`/usr/include`, `/usr/share`, `/usr/local`, `/usr/libexec`, `/bin`, `/sbin`,
`/lib`, `/lib64`, `/etc/ssl/certs`, `/etc/ca-certificates`, `/etc/alternatives`,
`/etc/ld.so.cache`, `/etc/ld.so.conf`, `/etc/ld.so.conf.d`, where present.
The server profile SHALL replace the table's `/etc/ssl` source with only
`/etc/ssl/certs`; `/etc/ssl/private` and other siblings are never mounted.
This is a consumed projection of the shared table, not a second bind list;
workspace hands retain their existing set. Missing TLS configuration fails
inside the server box without broadening this source. Absent optional system
sources are omitted; they are never replaced by a broader mount. Existing symlink spellings and their resolved mount identities
SHALL be checked. A fresh `/proc` sees only the server PID namespace; `/dev`
is minimal. Identity, hosts and nsswitch files SHALL be generated without host
credentials. Local egress uses a files-only resolver. Shared-network egress
may additionally bind only the checked `/etc/resolv.conf` read-only and use
files/DNS resolution; the whole of `/etc` is never bound. A required trusted
Brokkr bootstrap executable MAY be bound read-only as one file, subject to
the same reach, link and ancestry checks. No control plan, ledger, stage,
artifact root or store SHALL be mounted or inherited as an open descriptor.

The program tree SHALL be determined without running server code or parsing
arguments, shebangs, ELF metadata or loader behavior. Resolve argv[0] once
from an absolute path or the fixed `/usr/local/bin:/usr/bin:/bin` search path;
never search cwd or ambient PATH. Relative paths containing `/` refuse.
Canonicalize the executable, retaining the original resolution chain and
file identity for the pre-mount checks. Use a closed distinction:

- **System entry:** the resolved file's immediate parent is a canonical
  `/usr/bin`, `/usr/sbin`, `/usr/local/bin` or `/usr/local/sbin` directory
  (including `/bin` and `/sbin` aliases), already covered by the fixed system
  binds. Its program tree is that one executable file; there is no inferred
  `/usr` or `/usr/local` package root. The executable still MUST be singly
  linked. Loading support comes solely from the separately admitted system
  set, not an inferred package closure.
- **Package entry:** every other resolved file uses its canonical parent,
  or that parent's parent when the immediate directory is `bin` or `sbin`.
  The entire derived package tree MUST be singly linked, including when
  already covered by a system bind (for example `/usr/lib/docs/bin/server`).
  Bind the root read-only at its canonical absolute path only if the system
  set does not already cover it; never add a broader system mount.

Execute the resolved file. Classification is by this layout, never by uid
or whether a link-count check would pass. A symlink from a system bin into
`/opt/docs/bin/server` is a package entry. A launcher installed directly in
a shared system bin needs all dependencies already in the fixed system set;
otherwise install a dedicated package entry or fail inside the box.
This is a conservative installation-layout rule, not a promise to discover
an arbitrary language's dependencies. A package needing sibling code SHALL
place its installed entry under `<package>/bin/` or `<package>/sbin/` so that
the containing package is bound. A symlinked launcher uses its resolved
installation, never an enclosing host HOME. The broker SHALL neither ascend
further nor add mounts from argv, shebangs, environment values, imports or
runtime errors. A standalone non-system file uses its parent tree; `/`, the host HOME
or an ancestor containing the host HOME SHALL NOT become a package root.
Such a derived root takes the program-tree resolution cause below. A bare runtime is confined by the same rule, not qualified as
an arbitrary program. Anything it needs outside the binds fails INSIDE the
box; there is no unboxed retry or automatic dependency bind.

Seat reach means the canonical workspace and every declared hands bind
root, including `ro`, `rw` and `overlay`, and any additional effective
workspace/Git reach supplied by hands. The fixed read-only system toolchain
is shared infrastructure, not a declaration of seat-controlled reach; an
explicit hands bind overlapping it still counts as a reach root. All server
sources and destinations SHALL be disjoint from these reach roots in both
directions: a broad system/package bind cannot contain a reach root. In
particular the program tree SHALL lie outside every root by canonical path,
and every regular file in it, including the executable, SHALL have link count
exactly one. Directories are not subjected to the regular-file link-count
rule. Symlinks SHALL NOT add implicit binds: targets outside the approved
mount set are absent, and targets in seat reach refuse. No device, socket,
FIFO or other host endpoint SHALL ride inside a program-tree bind.

Canonical path comparison is necessary, not sufficient. Before lookup,
owner-rooted no-follow checks SHALL retain source and ancestry identities,
exclude replacement through any seat reach, and check resolved mount aliases
using host filesystem/mount identity. A bind alias can have another canonical
path and link count one. Uncertain alias exclusion SHALL refuse; a textual
prefix test is not a substitute. Apply this to system binds, the program tree,
bootstrap, the host bubblewrap executable and its resolution ancestry, store
and host-side plan/evidence roots, not just argv[0]. Check
nested mounts rather than assuming a parent read-only bind excludes them.
An admitted namespace SHALL expose every source read-only and no hidden
writable submount. Pin checked sources through namespace establishment; an
identity change SHALL refuse before lookup, never reopen a pathname blindly.
Host operator replacement outside managed-seat reach is not a promised threat
model; seat-mediated replacement is.

The source observation SHALL be bounded to 1,000,000 entries, directory depth
64, 40 symlink hops per resolution and 65,536 mount records per preparation,
within the same absolute 30 s startup deadline; these counts are the memory
bound, and no separate byte budget applies. Cyclic, over-limit or unsupported
identity data SHALL take "MCP server box filesystem identity is not protected",
never skip an entry. An entry the broker cannot read, outside the program tree
and bootstrap, is admitted only when its owner, mode and access ACL, observed
without read access to its contents, prove it root-owned, without group or
other write, and writable by no managed writer; any other unreadable entry SHALL
take that cause. Program-tree and bootstrap files must be readable.
Mount identity SHALL include the filesystem device and inode, mount root and
relative subpath from Linux mountinfo; mount ID or canonical spelling alone
cannot exclude a bind alias. Unexplained overlay/remote-filesystem identity
SHALL refuse. The checked handles SHALL be the actual mount sources (Linux
bubblewrap `--ro-bind-fd`), not re-opened path strings. A launcher lacking
that facility takes "MCP server box is unavailable". Readiness SHALL verify
all effective mounts, including nested read-only state, against this intent.
Program-tree and bootstrap files retain their unconditional single-link
rule, regardless of ownership. Only a regular system support/launcher file
outside that tree may have multiple links, and only with this complete
kernel write-exclusion proof: its filesystem owner differs from every managed
writer's mapped uid; group and other write bits are clear; no extended access
ACL is present; and no managed write surface can obtain the owner's identity,
change those permissions/ACLs, bypass DAC, or remount the backing filesystem.
Observe identities in the source filesystem's user namespace, including
idmapped mounts; an unmapped/overflow uid is not an owner proof. Bind these
credential/privilege facts to the plan and enforce them for the full attempt
lifetime, alongside whole-chain and mount-alias protection. Unknown mappings,
ACL semantics, privilege confinement or any failed condition take
"MCP server box filesystem identity is not protected" before lookup. A
read-only bind, root ownership alone or `fs.protected_hardlinks=1` alone
SHALL NOT qualify. Write permission and ownership belong to the inode, so a
protected file cannot become writable merely through an unenumerated hard
link; this avoids scanning the whole host for its names. A user-owned linked
library cannot take this exception. No exception applies to program files,
known reach overlap, store/evidence exclusion or writable mount aliases. Managed native write surfaces SHALL also exclude creation of new
writable aliases to these sources during the attempt; an unproved surface
refuses the filesystem-identity cause. Read-only mounts alone prove no such
host guarantee. Source inspection retains root handles and the active
ancestor stack, not one live FD per traversed file. It SHALL NOT cache a
previous preparation's verdict or skip a subtree to meet a budget.

Source exclusion SHALL also govern later managed writers, including zero-grant
runs in other worktrees. Admission of a writer and reservation of protected
sources SHALL be ordered atomically: neither start order nor simultaneous
starts may admit a writable alias beside a live server. Preserve reservations
until owned-process settlement, including after engine death; advisory-lock
release alone is not settlement. A conflicting or unprovable combination
SHALL refuse before the incoming dispatch with
"MCP server box filesystem identity is not protected"; when the incoming
party is the broker this is before lookup. An already admitted server SHALL
NOT be protected merely by detecting the write on its next call. Disjoint
worktrees may proceed when their complete write reach is proved disjoint.

Installation is a trust prerequisite, not a historical authenticity check.
The operator SHALL provision a trusted dedicated package tree. An otherwise
admissible broad tree such as `~/.cargo` derived from `~/.cargo/bin/server`
can expose unrelated `credentials.toml` to the trusted server. Store exclusion
protects the selected store only; masking knows only declared values. Admission
SHALL NOT claim discovery or masking of arbitrary installation credentials.
Current/concurrent managed-reach checks do not authenticate bytes planted by
a completed seat whose earlier bind has since been removed. Trusted
installation provenance and remediation of such earlier writes remain the
operator's responsibility; a version assertion or source-set metadata digest
supplies neither.

Admission order SHALL be plan/inventory/digest binding, MB2 eligibility and
existing compatibility checks, valid/reserved binding names, executable and
package resolution, reach/link/ancestry/mount checks, MB4 store exclusion,
then empty-box establishment. All those refusals SHALL precede secret lookup
and dialect-server execution, for requires and wants alike. Metadata-only
store identity checks are not secret-value lookup. Establish the actual box
with a trusted waiting bootstrap and verify its ready state, complete mounts,
private directories and network before resolving bindings. Readiness SHALL
be a single closed typed message of at most 4 KiB on a separate inherited control pipe, bound to the plan digest, source-set digest,
child identity and actual namespace identities. A marker/environment bit,
MCP stdout, duplicate message or another child cannot supply readiness.
The bootstrap SHALL wait on private control pipes and SHALL NOT run the dialect executable
before admission and injection complete. No secret is required to prove a
box can stand. Missing dependencies discovered only during server execution
are later execution/protocol failures, not pre-secret admission failures.

The following exact typed causes SHALL be used, with bounded site/holding
context and no raw child text. If several checks fail, the order above wins;
within filesystem checks use reach, hard links, then remaining identity.

| Refusal | Exact cause |
| --- | --- |
| Missing/forged/mismatched plan or inventory | `broker plan is not bound to this attempt` (kept) |
| Executable resolves into seat-writable reach | `MCP server launch resolves inside seat-writable reach` (kept; checked before other tree overlap) |
| Binding name collides with a fixed environment key | `MCP server startup inputs are not protected from seat writes` (kept for this collision; no arbitrary-startup qualification) |
| Executable missing/unresolvable, relative slash path, or root cannot be determined without widening | `MCP server box program tree cannot be resolved` (new) |
| Any source/destination overlaps a reach root, including a readable or overlay root | `MCP server box bind overlaps seat reach` (new) |
| A regular program-tree or bootstrap file has more than one link | `MCP server box program tree contains a multiply-linked file` (new) |
| Unsafe/changed ancestry, special file, mount alias, writable nested mount, or incomplete identity proof | `MCP server box filesystem identity is not protected` (new) |
| Linux box launcher unavailable after the normal start-time availability check | `MCP server box is unavailable` (new) |
| Namespace/mount/private-directory/network establishment or ready handshake fails | `MCP server box could not be established` (new) |

MB4 owns the distinct store-reach and store-in-box causes. Source admission,
readiness, handoff and initialization share one absolute 30 s startup bound
and the attempt's remaining deadline; no helper can wait indefinitely before
lookup. A refused
box has zero secret lookups and zero dialect-server starts (a waiting trusted
bootstrap is not the dialect server). Once an owned ledger is opened, a fatal
box refusal SHALL close it Failed with that cause under CR3. An unbound plan
creates no invented ledger. Post-admission failure to exec the boxed server,
including an absent loader/interpreter, SHALL use the distinct typed cause
`MCP server could not start inside its box`, close Failed with zero calls,
and never widen mounts or retry on the host.
The bootstrap SHALL report handoff rejection or exec failure on one private
close-on-exec status pipe, with at most one fixed tag from a closed typed
failure enum: HandoffFailed maps to MB4's handoff cause, ExecFailed to the
boxed-exec cause. Neither stderr nor an MCP frame decides that status.
Successful exec closes the descriptor. After the sender completes its frame
and closes, EOF without a failure tag permits bounded MCP initialization,
not a successful-session claim. Before that sender completion, channel loss
or invalid status takes the handoff cause; afterward an invalid tag takes
the boxed-exec cause. A valid HandoffFailed tag still takes the handoff cause
after sender completion. EOF cannot prove whether a bootstrap died before
exec: without a functioning MCP child, the existing protocol/timeout cause
applies. No missing status invents success or a host retry, and no status
descriptor survives server exec.

Only MCP initialization and the measured protocol operations necessary for
listing/calling granted tools SHALL be proxied. Tool listing SHALL expose
exactly the allowed subset, preserving schemas as data; pagination SHALL not
leak excluded tools. A call outside that set SHALL be refused before forwarding
with "tool '<tool>' is not granted to capability '<capability>'". Dynamic
catalog changes SHALL not widen it. A granted tool missing from the child's
complete listing SHALL refuse "MCP server does not offer granted tool
'<tool>'"; no partial substitute toolset is advertised as the full holding. Unrelated resource/prompt operations or
server requests for sampling, roots, credentials or additional model actions
SHALL not create another authority channel; unadmitted methods refuse
"MCP method is not admitted by this broker". The admitted protocol is MCP 2025-06-18 over newline-delimited JSON-RPC 2.0.
Initialize, initialized notification, ping, tools/list, tools/call and local
cancellation handling are the only admitted operations; no server-initiated
request is forwarded to the harness. Unknown protocol versions refuse "MCP
protocol version is not supported by this broker". Bounds are 1 MiB per
request, 8 MiB per response/canonical artifact, JSON depth 64, 1,024 catalog
tools over at most 32 pages, one forwarded call at a time per broker, 30 s
initialize/list and 120 s per call (all also bounded by the attempt deadline).
Cancellation grants 2 s for child settlement before existing attempt cleanup.
An over-limit input returns "MCP request exceeds the broker limit". A tools/call
request with an invalid tool identifier or parameter shape refuses "MCP tool
call request is invalid" before acceptance; the SC4 tool-name byte bound uses
the request-limit cause. Valid bounded calls, including tool/active-call/budget
denials, follow CC2/CR3's durable acceptance rule. Catalog
count/page/cursor failures return "MCP tool catalog exceeds the broker limit";
a second concurrent call returns "MCP broker already has an active call".
Responses exceeding bounds return the response-limit cause below; no truncated
response is delivered as complete. These are slice limits, not configurable
new grant keys. Responses SHALL match the exact typed JSON-RPC request ID,
method and session phase; string and numeric IDs are distinct. Unknown, wrong,
duplicate or late responses SHALL refuse "MCP server protocol is invalid" with a
typed correlation failure. Timeouts are absolute deadlines unaffected by ping,
progress or notification traffic. Cancellation/timeout ends an uncertain child
session; its external action SHALL NOT be replayed. Non-response frames share
a fixed 1 MiB cumulative byte budget per initialization/list or call operation;
exhaustion SHALL return "MCP server response exceeds the broker limit".
Fatal initialization, version, protocol, response-limit or timeout endings
SHALL latch a typed session failure in CR3's private Closed disposition,
even before the first call or after every accepted call has terminated.
Neither orderly EOF nor a successful harness result SHALL clear that fault;
CR4 owns attempt settlement. A normal tool error or recorded local denial
alone SHALL remain a call outcome, not a fatal session failure.

#### Scenario: Filtering is enforced at both list and call

- **WHEN** the fake child offers lookup and admin but the realm permits only lookup
- **THEN** listing returns exactly lookup and a direct admin call returns "tool 'admin' is not granted to capability 'library-docs'"
- **AND** the child's call log contains no admin invocation, while the broker ledger records the refused attempt without marking admin granted

#### Scenario: A result cannot grant a tool or a method

- **WHEN** child output advertises a new tool or requests sampling, or a model supplies a replacement dialect/plan in a call
- **THEN** no authority changes; an ungranted call or unadmitted method returns the corresponding exact refusal
- **AND** the original admitted call still has its checkpoint and the returned content remains DATA

#### Scenario: Version and protocol errors do not degrade to passthrough

- **WHEN** the child initialize response reports a server version different from the dialect's version
- **THEN** the broker refuses "MCP server version does not match the dialect's pinned version" before offering a tool
- **AND** malformed protocol, output-limit and timeout failures return respectively "MCP server protocol is invalid", "MCP server response exceeds the broker limit" and "MCP server call timed out", with typed ledger outcomes and no raw stderr echoed
- **AND** the literal protocol/limit contract above is tested at each boundary; no fallback to unbounded proxying is admitted

#### Scenario: Protocol and resource bounds refuse exactly

- **WHEN** a caller selects another protocol version, exceeds the request/depth bound, or calls concurrently while one call is active
- **THEN** it receives respectively the unsupported-version, request-limit or active-call cause above, without forwarding that call
- **AND** catalog overflow, more than 32 pages or a repeated cursor returns the catalog-limit cause; exact-limit positive controls still work
- **AND** invalid tool vocabulary or parameter shape refuses "MCP tool call request is invalid" before acceptance; a valid bounded concurrent call has one recorded refused outcome and no child forwarding

#### Scenario: Installed program trees run without workspace mounts

- **WHEN** an admitted user installation has entry `/opt/docs/bin/server` or a symlink resolving to that entry, with singly linked files outside all seat reach
- **THEN** `/opt/docs` is the read-only package tree, the resolved server executes inside its own empty-root box, and its sibling modules remain available without a loader or shebang proof
- **AND** the workspace, declared hands binds, Git metadata, host HOME, store and broker evidence are absent; legitimate tool calls still pass through the broker
- **AND** an entry directly in `/opt/docs` uses that same package root; `/usr/bin/server` is a singleton system entry, while `/usr/lib/docs/bin/server` has package root `/usr/lib/docs`; neither adds a broader system mount
- **AND** the singleton entry and every package regular file still refuse the program-tree hard-link cause when multiply linked, even if root-owned and not writable

#### Scenario: Runtime arguments do not widen the bind set

- **WHEN** a dialect starts a system Python or Node runtime with a script outside all approved binds, or a loader/RUNPATH/import refers outside those binds
- **THEN** access fails inside the box without exposing the missing host bytes; missing executable loader/interpreter yields "MCP server could not start inside its box", while a running server that fails initialization yields the existing bounded protocol/timeout cause
- **AND** an equivalent installed package whose dependencies are in its read-only tree can work; no argument/shebang/loader analyzer or host retry supplies authority

#### Scenario: Resolution failures and workspace executables refuse before lookup

- **WHEN** argv[0] is a relative slash path or cannot resolve under the fixed search path, or its package root would be `/`, the host HOME or an ancestor of that HOME
- **THEN** launch refuses "MCP server box program tree cannot be resolved" with zero secret lookups and dialect-server starts
- **AND** an executable resolving into seat-writable reach instead receives the kept "MCP server launch resolves inside seat-writable reach" cause; a server version assertion never admits it

#### Scenario: Reach includes every declared bind and containing mounts

- **WHEN** a program tree or system source overlaps the workspace or a declared hands `ro`, `rw` or `overlay` bind, whether it contains that root or lies inside it
- **THEN** launch refuses "MCP server box bind overlaps seat reach" before lookup or dialect-server start
- **AND** separate controls cover each bind mode, a symlink target in reach and a workspace beneath `/usr/local`; no broad system mount is an exception

#### Scenario: A hard link cannot hide a writable program alias

- **WHEN** the program executable or a sibling module has link count greater than one, including a second link in seat reach
- **THEN** launch refuses "MCP server box program tree contains a multiply-linked file" before lookup or dialect-server start
- **AND** a singly linked protected package passes this check; directory link counts alone do not cause the regular-file refusal

#### Scenario: Whole-chain identity and mount aliases remain admission checks

- **WHEN** a resolution directory or package ancestor can be replaced through seat reach, a special file or writable submount is present, or a bind alias cannot be excluded by filesystem/mount identity
- **THEN** launch refuses "MCP server box filesystem identity is not protected" with zero lookups and dialect-server starts
- **AND** replacing a checked source between observation and mount cannot substitute another tree; owner-bound plan failure retains "broker plan is not bound to this attempt"

#### Scenario: Later writers cannot invalidate an admitted source

- **WHEN** worktree A has an admitted live server and a zero-grant writer from B requests a writable alias to its program or system support source
- **THEN** B refuses "MCP server box filesystem identity is not protected" before dispatch, and A's source bytes remain unchanged throughout its lifetime
- **AND** in reverse order A refuses with that cause before lookup or server start; simultaneous starts admit at most one conflicting party, while a disjoint B can run
- **AND** killing A's engine does not free its reservation while an owned bootstrap/server may survive; uncertain settlement refuses the incoming conflict rather than trusting lock release

#### Scenario: Box establishment precedes secret resolution

- **WHEN** Linux loses its checked bubblewrap launcher, or namespace/mount/private-tmpfs/network setup or its ready handshake fails
- **THEN** the broker refuses respectively "MCP server box is unavailable" or "MCP server box could not be established" before any secret lookup or dialect-server start
- **AND** failed readiness by its fixed deadline terminates the waiting helper under MB5; there is no ordinary host-process fallback

#### Scenario: Source protection includes system loading aliases and bounded observation

- **WHEN** a multiply-linked system support file fails any kernel write-exclusion condition above, mountinfo exposes a different-path alias into seat reach, a source walk exceeds any bound above, or a managed native surface can create a writable source alias
- **THEN** launch refuses "MCP server box filesystem identity is not protected" with zero lookups and server starts; a multiply-linked regular file within the program tree retains the more specific program-tree cause
- **AND** controls cover each bound independently, a nlink-one bind alias and a hard-linked user-owned library under `/usr/local`; a complete protected source set passes without interpreting imports

#### Scenario: Ordinary protected system hard links do not reject a package

- **WHEN** a singly linked `/opt/docs/bin/server` package or singleton system entry uses system binds containing ordinary multiply-linked support files whose mapped owner, modes, absent access ACL and managed privilege confinement establish the complete kernel write-exclusion proof
- **THEN** those support files pass source admission without renaming, unlinking, copying or pruning the installed system; the program-tree single-link rule still applies independently
- **AND** independently introducing owner equality, group/other write, an access ACL, unresolved uid mapping or a permission-bypassing native surface refuses "MCP server box filesystem identity is not protected" with zero lookups/starts; a root-owned hard-linked program file instead takes the program-tree cause

#### Scenario: Public certificates do not bind private TLS material

- **WHEN** the host has readable `/etc/ssl/certs` beside an unreadable `/etc/ssl/private`
- **THEN** only the certificates subtree enters the server source set; private material is neither traversed nor mounted, and its presence cannot fail that source walk
- **AND** an unreadable entry inside an actual admitted source refuses "MCP server box filesystem identity is not protected" before lookup unless its owner, mode and access ACL prove it root-owned and writable by no managed writer; no skipped entry becomes an admitted source

#### Scenario: A root-only system file is admitted only when provably untouchable

- **WHEN** an admitted system support source holds a regular file the broker cannot read, such as a root-only configuration file or helper
- **THEN** the walk records it without reading its contents and admits it only when its owner is root, its mode grants no group or other write, and its access ACL grants no managed writer write access
- **AND** the same unreadable file with a non-root owner, a group or other write bit, or a writer ACL entry refuses "MCP server box filesystem identity is not protected" with zero lookups/starts, and an unreadable program-tree or bootstrap file always refuses

#### Scenario: Installation reach is not credential discovery or historical authentication

- **WHEN** an otherwise admissible package tree contains an unrelated credential canary or bytes installed before the current managed reach was established
- **THEN** tests record that the trusted server can read those bound bytes; they assert no generic credential redaction or historical authenticity from admission
- **AND** a separate current or concurrent seat-writable alias still refuses the owning reach/identity cause before lookup, and selected-store exposure still takes MB4's exact store cause

#### Scenario: Readiness is evidence from the owned empty namespace

- **WHEN** readiness is truncated, duplicated, over 4 KiB, from another child, has a wrong plan/source digest or reports mismatched mounts or namespaces
- **THEN** launch refuses "MCP server box could not be established" before lookup and terminates the waiting helper
- **AND** a marker-only or MCP-stdout message cannot authorize delivery; losing descriptor-mount support takes "MCP server box is unavailable" with no pathname fallback

#### Scenario: Kernel execution is confined rather than predicted

- **WHEN** an installed script has a shebang the kernel rejects, or names an interpreter not mounted inside the box
- **THEN** the boxed exec fails with "MCP server could not start inside its box" and a zero-call Failed session; Brokkr neither emulates the shebang nor retries outside the box
- **AND** a valid shebang whose interpreter and program tree are bound executes under that kernel's normal semantics inside the admitted box

#### Scenario: Exec status is separate from protocol output

- **WHEN** the confined exec syscall fails for an absent interpreter or loader
- **THEN** the broker receives ExecFailed and records "MCP server could not start inside its box" with zero calls, regardless of misleading or empty stderr
- **AND** HandoffFailed instead records "MCP server secret environment could not be delivered", including when sent after sender completion; an invalid tag after sender completion takes the boxed-exec cause
- **AND** successful exec closes the status pipe and permits only MCP initialization, whose failure keeps its existing protocol/timeout cause; bootstrap death before sender completion takes the handoff cause, while EOF afterward without a functioning child cannot certify success
- **AND** a healthy child inherits only its intended stdio

#### Scenario: Secret bindings cannot replace protected startup directories

- **WHEN** separate bound plans declare HOME or TMPDIR and the corresponding store value would point the protected installed fake server at a seat-writable startup marker
- **THEN** each refuses "MCP server startup inputs are not protected from seat writes" before any store lookup or child start
- **AND** a currently benign value and rotation to a different value without changing dialect bytes receive the same name-based refusal
- **AND** a DOCS_TOKEN control reaches the child through the shared injector while the child observes the exact engine-private HOME and TMPDIR; each collision has an independent removal proof

#### Scenario: Correlation and deadlines survive hostile protocol traffic

- **WHEN** a child sends a wrong-type ID, duplicate/late response or response for the wrong phase
- **THEN** the broker ends that session with the exact protocol cause, records the unresolved call without a digest, and forwards no mismatched result
- **AND** continuous pings or progress cannot extend the fixed deadline; notification-budget exhaustion has the exact response-limit cause, and an uncertain cancelled action is never retried

### Requirement: MB4 only the real server receives broker secret bindings

Per R1, "never in argv, never in the harness's environment": the broker SHALL
resolve only dialect-declared names from the operator's decision 0012 store
only after MB3 box admission, immediately before boxed server exec. Dialect argv SHALL execute directly, without interpolation
or a shell added by Brokkr. The actual child environment alone receives
values through U6a's existing single plaintext injector and the confined
handoff below, without adding a second accessor call. The harness, hands box,
manifest, broker plan and argv SHALL contain no resolved value. Dialect secret
names SHALL be removed from the inherited harness and broker environment;
they are resolved afresh only for the real child. A measured harness auth
variable colliding with such a name SHALL refuse the holding with cause
"MCP secret binding '<name>' collides with harness authentication", using
GP1's required/optional form. A secret-store path inside workspace hands'
readable reach SHALL refuse launch with "MCP secret store is reachable by
workspace hands"; owner-bound alias checks SHALL not trust lexical paths.

Missing, disallowed, unreadable or invalid bindings SHALL refuse child spawn
using decision 0012's typed causes and safe name-only diagnostics. Dialect
egress SHALL meet the operator's existing binding minimum under 0036;
a capability class or gate office does not grant secret clearance. The
compatibility cause is exactly "MCP dialect '<dialect>' has egress '<class>'
below binding minimum '<minimum>'", in GP1's required/optional forms.
The actual server environment SHALL be cleared and contain exactly its
declared bindings plus these builder-owned fixed entries: `PATH` =
`/usr/local/bin:/usr/bin:/bin`, `HOME` = `/runtime/home`, `TMPDIR` = `/tmp`,
`USER` = `runner`, `LOGNAME` = `runner`, `LANG` = `C.UTF-8`, `LC_ALL` =
`C.UTF-8`, and `BROKKR_HANDS_BOX` = `1`. HOME and TMPDIR SHALL be distinct
fresh private tmpfs mounts per server lifetime, with cwd `/runtime/home`.
No inherited git, updater, cache, credential, locale or loader environment
is copied. Fixed entries and their reserved-name check have one builder
owner. Shared decision-0012 name validation runs first; otherwise-valid
collisions such as HOME/TMPDIR take MB3's kept startup-input cause before
lookup, regardless of current or rotated values.

A declared code-loading name permitted by decision 0012, such as
`PYTHONUSERBASE`, `CLASSPATH`, `LUA_PATH`, `GEM_HOME` or
`PHPRC`, SHALL NOT be rejected merely for affecting code loading. It exists
only in the boxed server environment: host path targets are approved read-only
binds or absent; private tmpfs is initially empty and only that server can
populate it. Existing `PATH`, `IFS`, `LD_PRELOAD`, `LD_LIBRARY_PATH` and
`BROKKR_` refusals remain. `_JAVA_OPTIONS` is already invalid under the
shared `[A-Z][A-Z0-9_]*` grammar and remains refused, not newly admitted.
No growing broker-specific loader denylist or
semantic inspection of secret values replaces the box. Operator-selected
server code remains trusted; arbitrary transformations of secrets retain
0012's stated limit.

The broker SHALL read the store outside the server box with the existing
name, clearance, mode and read-isolation checks, plus the new owner-bound
identity checks. The current pathname metadata/read pair is not an identity
proof: the eventual read SHALL consume the admitted no-follow file descriptor,
with the same parser and mode checks, not resolve the path again. If the store
does not exist, even for a secret-free dialect, this slice SHALL refuse
"MCP server box filesystem identity is not protected" before lookup; an empty
protected operator store is the supported secret-free control. No store is
created by admission and no value is read merely to classify a failure. The store SHALL never
be mounted, even through a system/program subtree, nested mount, symlink or
bind alias. Metadata-only checks SHALL compare the store's owner-rooted
identity against both hands-readable and server-readable reach; canonical
spelling alone is insufficient. Hands exposure keeps "MCP secret store is
reachable by workspace hands"; server exposure has the new exact cause
"MCP secret store would be mounted in the server box". When alias or ancestry
exclusion cannot be proved, MB3's filesystem-identity cause applies. These
checks precede value lookup, even when the server declares no secrets.
Store file descriptors SHALL be closed before server exec and never passed
into its namespace. Plans, ledgers and staging stay with broker/engine outside
that namespace as CR2 requires.

Secret values SHALL NOT be passed in bubblewrap `--setenv`/`--args`, process
argv, a temporary file, or the host launcher's environment. An already-ready
trusted in-box bootstrap SHALL receive a bounded, private, anonymous pipe
handoff of only declared bindings, close that channel and all unrelated
descriptors, and execute the server with its final environment. The handoff
SHALL reuse U6a's `secret::bind_environment` as the sole plaintext injector;
`machine_proof.rs` SHALL still find exactly one production `expose_for_spawn`
call site, inside that function. Any environment serialization needed for
the pipe stays inside the shared secret module and consumes that injector's
prepared environment, never a second plaintext accessor or encoder of Secret.
The host bubblewrap and bootstrap loader start with only their fixed safe
environment; code-loading bindings become environment entries only for the
final exec inside the established box. The bootstrap accepts no fresh mounts,
argv or authority from the pipe. The handoff is one length-delimited frame
of at most 1 MiB, at most 256 bindings and depth 4, with exact declared-name
equality, no duplicate keys, unknown fields, NUL or trailing frame. Values
are not truncated. The sender SHALL close its write end after the one frame;
the receiver SHALL consume the complete frame and then establish EOF before
exec. Any trailing byte, including an incomplete second frame, refuses. EOF
before the frame completes is truncation; EOF immediately after the complete
frame is the required terminator. A complete frame whose sender keeps the
pipe open waits only until the shared deadline. Sending and receiving share
the absolute startup deadline and are interruptible; invalid framing,
excess, premature EOF or timeout takes the handoff cause below. Temporary plaintext buffers stay inside the secret
boundary without Debug/logging and are best-effort wiped after use. A broken/invalid handoff SHALL refuse with
"MCP server secret environment could not be delivered" and end the session;
this is a post-lookup failure with zero server starts, not a box admission
refusal. No result/log/artifact exposes the handoff bytes.

Network SHALL be a closed projection of the pinned dialect egress:
`local` unshares the network namespace with no host loopback or external
network; `contracted` and `uncontracted` share the host network, as declared.
Shared network includes host loopback and Linux abstract Unix sockets;
filesystem mount exclusion does not isolate those services. This is the
ruling's declared shared-network reach, not a destination allowlist or D11
restriction enforcement. Operator-selected servers and the host services
they can reach remain trusted under that declaration.
Absent dialect egress still means `uncontracted`. R2 independently keeps
workspace hands at network false. U5a2's binding-minimum comparison and
GP1's gate office/class checks run unchanged before launch; shared network
does not upgrade clearance, and a higher-clearance harness grants none.

Known literals and common encodings SHALL
be masked before any response, stderr, ledger, artifact or diagnostic leaves
the broker. Masking SHALL preserve protocol structure and work across stream
chunk boundaries. Before staging or delivery, the broker SHALL check bounded
prepared output, including correlation/structural fields, for known literals
or common encodings that string/key masking cannot safely remove. If removing
an occurrence would change a non-string scalar or required protocol/data shape,
it SHALL refuse with "MCP response cannot be safely masked", without the value.
It SHALL neither coerce a number to text, substitute another scalar nor deliver
or persist the unsafe response. An already forwarded call SHALL have a failed
Terminal with that safe cause and no response digest, not a pre-forward refused
outcome. When safe correlation is impossible, no unsafe frame is emitted and
the session closes failed with that cause. Legacy mask_json scalar semantics
remain unchanged; this is an additional broker output-edge check using the
shared encoding definitions. Exact-number preservation is a separate check.

#### Scenario: Secret values stop at the child environment

- **WHEN** the fake MCP server needs DOCS_TOKEN and the operator store supplies it
- **THEN** the boxed child receives DOCS_TOKEN; captured harness/broker/bubblewrap argv, harness and host launcher environments, plans, journal and artifacts contain none of its raw or decision-0012 encoded values
- **AND** an applicable argv secret-reference connection refuses under SC1 before spawning, with "MCP connection argv cannot contain secret references; declare environment bindings in secrets"

#### Scenario: Undeclared or missing secrets do not fall back to ambient values

- **WHEN** DOCS_TOKEN is absent from the store but exists in the harness's ambient environment
- **THEN** child spawn receives the existing missing-binding refusal; no ambient value satisfies it
- **AND** a dialect declaring a denied injection name receives decision 0012's exact denied-name refusal before any child starts

#### Scenario: Dialect secrets meet their own route clearance

- **WHEN** a secret-bearing MCP dialect has uncontracted egress and the operator's binding minimum is contracted
- **THEN** requires refuses and wants drops with "MCP dialect 'docs-mcp' has egress 'uncontracted' below binding minimum 'contracted'"
- **AND** the harness's higher route clearance does not authorize the dialect's bindings

#### Scenario: Ambient credentials and exposed stores are not broker channels

- **WHEN** DOCS_TOKEN is exported by the operator as well as stored, and a broker holding declares that name
- **THEN** the harness and broker inherit no DOCS_TOKEN; only the real child receives the store's binding
- **AND** a measured auth-name collision receives the exact collision compatibility cause, while a store inside hands reach receives the exact store-path launch refusal above

#### Scenario: A store under a read-only bind is still exposed

- **WHEN** the selected store lies under a proposed system/program bind, or the same store inode is exposed through a nested bind mount or another readable alias
- **THEN** launch refuses "MCP secret store would be mounted in the server box" before any value lookup or server start, even for a secret-free dialect
- **AND** a store alias visible to hands instead receives the kept hands-reach cause first; unresolved alias identity receives MB3's filesystem-identity cause
- **AND** an inaccessible owner-bound store outside both reach sets supplies declared values while its path and open descriptors remain absent from the box

#### Scenario: Store identity is admitted even without declared values

- **WHEN** a secret-free dialect names a missing store, or a store/ancestor is replaced between admission and delivery
- **THEN** launch refuses "MCP server box filesystem identity is not protected" before value lookup, and creates neither a store nor a server
- **AND** an existing protected empty store outside all mounts allows the secret-free control with zero value lookups; a declared binding is read from the already admitted descriptor with the unchanged mode/parser/name diagnostics

#### Scenario: The environment handoff has one bounded meaning

- **WHEN** the post-readiness handoff exceeds 1 MiB or 256 bindings or depth 4, repeats a name, changes the declared name set, includes unknown fields or NUL, truncates, stalls or sends a trailing frame
- **THEN** it fails "MCP server secret environment could not be delivered" with zero server starts and a Failed session; exact-bound valid frames retain every value
- **AND** independent controls bind each limit and framing check, and the server receives only its intended stdio after all control, store and mount-source descriptors close

#### Scenario: A complete binding frame still waits for EOF

- **WHEN** the sender supplies one valid complete binding frame but leaves its write end open
- **THEN** the bootstrap starts no dialect server; expiry returns "MCP server secret environment could not be delivered" and closes Failed with zero calls
- **AND** closing immediately after that frame permits confined exec; adding any byte or a second frame before closing instead gives the same handoff refusal, even when the first frame was already decoded
- **AND** each case records its actual lookup count; no source, store, ledger or control descriptor remains readable through the server's `/proc/self/fd`

#### Scenario: Declared loading variables never reach a host loader

- **WHEN** a valid declared PYTHONUSERBASE, CLASSPATH, LUA_PATH, GEM_HOME or PHPRC points to code in seat reach
- **THEN** the value reaches only the boxed server, which cannot read that target; no binding appears in the host bubblewrap or bootstrap loader environment
- **AND** an approved read-only target inside the program/system binds is usable under that runtime's semantics; an undeclared ambient variable is absent
- **AND** existing denied names still take decision 0012's name-only cause; `_JAVA_OPTIONS` refuses "secret name '_JAVA_OPTIONS' does not match [A-Z][A-Z0-9_]*"; fixed HOME/TMPDIR collisions still refuse before lookup under MB3

#### Scenario: Fresh private directories and exact environment are per server

- **WHEN** two otherwise identical admitted brokers start with DOCS_TOKEN and no other declared binding
- **THEN** each child observes exactly MB4's eight fixed entries plus DOCS_TOKEN, private tmpfs HOME and TMPDIR, and cwd `/runtime/home`
- **AND** a marker written by one server in its HOME or TMPDIR is invisible to the other and to a replacement attempt; host home/tmp markers are absent

#### Scenario: Failed environment delivery does not start a server

- **WHEN** the private binding handoff fails after the box is ready and values have been resolved
- **THEN** the broker reports "MCP server secret environment could not be delivered", closes Failed with zero accepted calls and starts no dialect server
- **AND** pipe bytes never become argv, a file, a log or a host loader environment; cleanup closes both ends and removes the box

#### Scenario: Egress chooses only the server network

- **WHEN** otherwise eligible dialects declare local, contracted, uncontracted or omit egress
- **THEN** their server boxes have respectively isolated, shared, shared and shared network; the local control cannot reach a host-loopback sentinel or an external endpoint
- **AND** networked positive controls can reach the test endpoint, host-loopback sentinel and Linux abstract Unix socket sentinel without changing the hands box's network-false state; local controls cannot reach any of them, and shared-network DNS uses only its checked resolver input
- **AND** U5a2 still refuses/drops a below-minimum route with its existing exact cause before lookup; GP1 still denies unlisted gate egress

#### Scenario: Known encodings are masked before persistence and delivery

- **WHEN** the fake child returns a bound literal, its common encodings or a split-stream occurrence in safely redactable result text/keys or stderr
- **THEN** every delivered/persisted surface contains the declared redaction instead, and the retained digest is over the masked bytes
- **AND** a real compiling removal of masking makes the full leak-scan test fail; an arbitrary transformed secret remains outside 0012's claimed guarantee

#### Scenario: A numeric secret cannot pass through structured output

- **WHEN** the fake child returns the digits-only canary 876543210 as a bare number in structuredContent, with effective retention either true or false
- **THEN** the broker returns "MCP response cannot be safely masked" without the value, records that forwarded call as failed with no digest, and stages/delivers none of the unsafe response
- **AND** the same canary as text is redacted, an unrelated exact numeric value is preserved, and escaped/encoded strings are masked; masking-created duplicate keys retain the protocol refusal
- **AND** the refusal itself passes the leak scan, and independently removing the scalar check makes the canary assertion fail; numeric precision tests remain separate

#### Scenario: A protocol field cannot be repaired into a different response

- **WHEN** a known secret occurs in a correlation or structural field that cannot be redacted while preserving its admitted meaning
- **THEN** the broker emits no unsafe frame and closes with failed disposition and cause "MCP response cannot be safely masked"
- **AND** accepted calls keep their verified outcomes or become interrupted if no terminal was durable; no raw identifier is copied into diagnostics or evidence

### Requirement: MB5 attempt cleanup covers broker and child

The broker, bubblewrap supervisor, waiting bootstrap, server and descendants
SHALL remain owned by the harness attempt's process tree and host process
group. The server profile SHALL NOT inherit hands' `--new-session`, call
`setsid`, or create a broker-owned detached group. Reuse #403 ownership and
settlement; retain `--die-with-parent`, PID/IPC/UTS isolation, supported cgroup
namespace isolation and all capabilities dropped from the common builder.
Cancellation, timeout, normal shutdown and failed box establishment or
initialization SHALL close pipes and settle that owned tree before folding.
Server stdin/stdout SHALL be piped exclusively through the broker's MB3
filter and MB4 masker, with bounded stderr drainage. No daemon, direct harness
to server pipe, or separate untracked engine group is introduced. HOME/TMPDIR
tmpfs disappear with namespace teardown; host bootstrap scratch, if required,
uses the existing Session lifetime/reaper only after owner/ancestry checks.
Persistent plans/ledgers/staging SHALL NOT use delete-on-drop Session scratch.
Each new child failure SHALL leave an attributable ledger outcome even when
its result cannot be delivered: call evidence when a call was accepted, and
CR3's failed session disposition for a fatal ending, including zero-call
box setup or initialization failure. The engine SHALL judge that disposition under CR4
before accepting a harness success; child stderr or process exit alone does
not prove the harness reported failure. Existing #403 residuals SHALL remain named;
a process-group assertion SHALL not claim cgroup guarantees.

#### Scenario: Cancellation leaves no owned child running

- **WHEN** a fake server blocks and the attempt is cancelled or times out
- **THEN** the broker, bubblewrap supervisor, bootstrap and owned fake child settle under the same attempt cleanup, private tmpfs disappear, and engine folding records the interrupted call
- **AND** a failed initialization cleans up the child without exposing a tool; real Linux evidence is required for namespace enablement

#### Scenario: Startup and normal exit settle the same owned box

- **WHEN** the box fails readiness, the server fails initialization, the server exits mid-call, or normal MCP shutdown completes
- **THEN** every case closes the broker pipes and settles the supervised box; no server or bootstrap remains outside the attempt's ownership
- **AND** established failed sessions keep their exact MB3/MB4 cause and call count under CR3/CR4, while a normal zero-call session can close Clean
- **AND** real Linux process and mount evidence, including cancellation during the pre-secret wait, is required; a process-group assertion alone supplies no cgroup guarantee

## Decisions

R1's harness child is adopted because engine.rs:4434 and hands.rs:1119 show
that ownership today; an engine-spawned sibling would require new supervision
and transport. 0077 records the difference from 0065's literal
"launched by the engine".

R2's explicit refusal takes precedence over a wants drop for a site that is
not safely boxed; ordinary unsupported carriage still follows ruling 5.
R2 still names 0043's workspace tool-call box. The new ruling adds an
independent server box and leaves the broker/harness outside both; it builds
no whole-harness box or 0072 boundary.

Native read-only access is not secrecy (hands.rs:1–13, decision 0043),
process.rs:173–178 otherwise inherits the workdir, and a method result needs
typed request correlation (0071 rulings 3, 8, 9). U0 may prove a harness
unsupported; no whole-harness box, installer or attestation service substitutes
for proof. The protected operator installation remains trusted code.

Nonempty restrictions remain deferred under D11, even if a child claims
support. Empty restrictions still come only from the realm. URL transport,
native retention and wider boundaries require later commissioned work.

process.rs:499–520 consumes driver status, so child exit cannot substitute
for CR4's independent session judgment. secret.rs:637–668 leaves numeric
scalars unchanged; the secrets guide documents that residual. R1's "masks
secrets in its own output" requires a refusal when redaction cannot preserve
shape. Reject inheriting that residual, coercing scalars or relying on
precision: 876543210 is exactly representable. Keep text masking, shared
encodings and the independent precision check (0071 rulings 3, 5, 8, 9).

The shared injector at secret.rs:122 overwrites environment entries, while
shared name validation permits HOME/TMPDIR. Keep fixed-key checks before
lookup, even for benign values: rotation does not change dialect identity.
Reject a global grammar change or second injector. A host bubblewrap process
with declared loading variables could run code before confinement, so its
environment carries no bindings; the anonymous handoff is gated by actual
box readiness (0071 rulings 3, 5, 8–10). Boxing does not make host-side
store/native-read or evidence checks unnecessary.

The conservative installed-entry layout above deliberately bounds the tree
without inventing v1 fields or predicting a language's loader. Reject automatic
root discovery by running package managers or expanding mounts after failure.
User-installed packages can use a protected `<package>/bin/server`; unusual
layouts may fail safely inside the box until separately commissioned support.
A bare interpreter no longer needs a language-specific program proof: the
same mounted filesystem bounds everything it can read. It grants no tool
until the existing initialization/version/catalog checks pass.

The seven supplied HIGH findings are reconciled individually. These are
design dispositions, not claims that held code was repaired or measured.

| Finding | Removed by construction | Check that remains before lookup |
| --- | --- | --- |
| H1 resolution identity skipped directory protection | Once pinned mounts stand, seat paths cannot replace boxed resolution inputs. Boxing alone does not establish those mounts' origin. | Retain the executable's resolution chain and all source ancestry identities through mount readiness; reach/identity refusals cover replacement and uncertainty. |
| H2 implicit ELF loader/RUNPATH and Python sibling loading | Yes, loading sees only approved read-only system/program binds or fails inside the empty root. | Check the whole program tree and all mounted sources, including nested mounts and aliases. No ELF/import closure proof remains. |
| H3 bare runtime without protected program | Yes, the runtime has the same bounded filesystem even without a separately recognized program argument; an absent script cannot expose host bytes. | Executable/tree admission and initialization/version/catalog checks remain; arguments never authorize new mounts. |
| H4 ever more loading environment names | Yes for seat tampering inside the admitted box, including valid declared loading names; no inherited loading variables and none in a host loader environment. | Keep shared name denials, builder-key collisions, cleared environment, pre-lookup readiness and confined handoff. No expanded language denylist. |
| H5 owner/private whole-chain protection | Child HOME/TMPDIR/cwd are fresh private tmpfs by construction. Host plan/ledger/store/bootstrap ancestry is outside that protection. | Owner-rooted no-follow whole-chain and alias checks remain for every host control/source root; Session lifetime alone is insufficient. |
| H6 store identity hidden by bind mounts | Not automatically. A read-only bind can still expose the store. | Prove store identity absent from hands-readable and server mounts, including nested/bind aliases; uncertainty refuses before lookup. Keep U0 native process/store read isolation. |
| H7 shebang proof differed from host execution | Yes. The actual kernel interprets the shebang only inside the admitted box; an absent or invalid interpreter fails there. | Protected executable/tree and normal typed boxed-exec/protocol failure handling remain. No shebang parser or host fallback. |

Reuse the hands builder's common namespace/system-mount machinery with a
separate typed server profile; copying `box_argv` or passing a fake worktree
would duplicate policy or leak reach. Existing hands behavior keeps its own
consumer during extraction (0071 rulings 2, 4–6, 10). Network is derived once
from typed egress; no environment, provider or model flag can override it.
D11 restrictions, masking, durable failure, filtering and U9b's enablement
fence remain independent obligations (0071 rulings 3, 8, 9).

The missing-store case is deliberately refused even without bindings: an
unobserved inode is no proof that a future store cannot appear inside a bind.
An empty protected store is a bounded compatibility cost. Source traversal
and handoff bounds above are execution limits, not edits to frozen v1.
Descriptor mounts and descriptor-bound store reads close observation/use
replacement; accepting a pathname recheck would leave H1/H5/H6 unresolved.
The returned correctness finding C1 is adopted: blanket system-link refusal
and deriving `/usr` for a `/usr/bin` entry rejected ordinary Linux sources.
Separate singleton system entries from package trees, and replace that blanket
refusal only for system support with the complete kernel write-exclusion
proof. Reject the suggested uid-only relaxation and any exception to the
commission's program-tree single-link rule (0071 rulings 3, 8, 9).
A metadata survey also found unreadable `/etc/ssl/private`; narrow the server
certificate source instead of silently skipping unreadable bound content.
C2's budget is owned by SD4 and U6c5, then repeated end to end by U6f/U9b;
limits remain fail closed. S1 and S3 are adopted as the installation trust
limits above. S2 is adopted as the explicit shared-network service reach;
reject a new destination filter or loopback ban contrary to the ruling.
These choices retain user-installed protected packages without claiming all
layouts or histories qualify (0071 rulings 3, 5, 8–10).

Lifetime exclusion cannot be inferred from a snapshot of current writers or
from leases keyed only by worktree. Order source reservation against every
managed dispatch and preserve it through settlement; reject a shape whose
coordination cannot prove that exclusion. The new scenario makes the existing
attempt-lifetime promise observable in both start orders (0071 rulings 3, 8–10).
Complete-frame EOF and a private close-on-exec failure pipe preserve the existing
handoff and boxed-exec causes without reading server prose. Its two fixed tags
retain receiver-side handoff errors after the sender has closed; EOF itself
cannot distinguish a dead bootstrap from successful exec, so initialization
remains mandatory. Neither
adds negotiation, authority or a public lifecycle (rulings 3, 5, 8–10).
