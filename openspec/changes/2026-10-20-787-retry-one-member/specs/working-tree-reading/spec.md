## ADDED Requirements

### Requirement: The working tree is read once, by git's exit and its words

The working tree of the operated repository SHALL be read by one reader, the home of `git_dirty`, which runs `git status --porcelain` there and reads one of four readings: `clean` when git exited zero and printed nothing; `dirty` when git exited zero and printed something; `refused` when git exited other than zero, whatever it printed; `unrun` when git could not be run there, a directory that does not exist included. The reader SHALL read nothing else: not git's words on its error stream, and not the files the repository ignores, which git does not report. The ship gate, every panel attempt's `tree` record and the carry's observations at the command and at the drive SHALL read through this one reader and SHALL each decide over its reading by their own rule; no second reader of the tree SHALL be added.

#### Scenario: Four readings

- **WHEN** the reader runs in a clean repository, in a repository with an uncommitted edit, in a repository with a commit whose index file git cannot read, and on a directory that does not exist
- **THEN** it reads `clean`, `dirty`, `refused` and `unrun`, in that order

#### Scenario: A directory that is no repository

- **WHEN** the reader runs in a directory that is no git repository, where `git status --porcelain` exits 128 and prints nothing
- **THEN** it reads `refused`, and what it decides is each gate's own

#### Scenario: An ignored file

- **WHEN** a file the repository's `.gitignore` names is edited and left so
- **THEN** the reader reads `clean`, since git reports nothing of it

### Requirement: The ship gate decides `dirty_worktrees` over the reading

`dirty_worktrees` SHALL be `true` on `dirty`; `true` on `unrun`, as today; `false` on `clean`; and on `refused`, `true` when the repository answers a head, `git rev-parse HEAD` succeeding as `git_head` reads it, and `false` when it answers none. So a directory that is no repository ships as it does today, a host with no git stops as it does today, and a repository with a commit that git refuses to read stops at `SHIP-DIRTY`, where today it reaches `done`. `ship_inputs` SHALL NOT change: `git_dirty` keeps its name and its boolean, deciding over the reading and the head.

#### Scenario: A run over no repository ships as before

- **WHEN** a run is operated in a bare directory with no git repository, as the house's machine proofs run, and reaches ship
- **THEN** `dirty_worktrees` is `false`, neither `SHIP-DIRTY` nor `SHIPPED-DIRTY` fires, and the run reaches `done` with its decision payloads byte for byte as before this change

#### Scenario: A repository git refuses to read stops at ship

- **WHEN** a run is operated in a repository with a commit whose index file is unreadable, so that `git rev-parse HEAD` answers and `git status --porcelain` exits 128 printing nothing, and reaches ship
- **THEN** `dirty_worktrees` is `true` and `SHIP-DIRTY` stops the run, where before this change it read `false`

#### Scenario: A host with no git

- **WHEN** git cannot be run in the operated directory
- **THEN** `dirty_worktrees` is `true`, as before this change

#### Scenario: A repository git reads

- **WHEN** a run is operated in a clean repository, and in one with an uncommitted edit
- **THEN** `dirty_worktrees` is `false` and `true` respectively, as before this change

#### Scenario: An empty repository

- **WHEN** the operated repository has no commit, so that `git rev-parse HEAD` fails and `git status --porcelain` exits zero
- **THEN** `dirty_worktrees` follows what git printed, and the head is not asked
