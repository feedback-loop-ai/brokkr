# Tasks: crossing enactment

All tasks are checked: this change FOLDS behavior already built and
proved by Phase 2 slices (ii)–(vi) into the living truth, and adds no
new behavior. Each task names the requirement it serves.

## 1. The word (`realm-crossing`: the vocabulary is forge.realms/v5)

- [x] 1.1 Confirm `contracts/realms.v5.schema.json` types `publishes` and `consumes` as arrays of the ruled shapes and refuses a written `null`.
- [x] 1.2 Confirm `brokkr-core::realms` judges each malformed map by the map's text alone, with no I/O.
- [x] 1.3 Confirm the self-consumption refusal and the older-label refusal are pure and named.

## 2. The loader (`realm-crossing`: resolution and verification at world load)

- [x] 2.1 Confirm the loader reads each published file from the publishing realm's own tree and compares each consumer pin to the raw bytes.
- [x] 2.2 Confirm a moved pin is carried as data naming the consuming realm, the crossing, the publisher, both digests and the file.
- [x] 2.3 Confirm an unreadable publication is the publisher's failure and its consumers' pins are unchecked, never matching.

## 3. The refusal (`realm-crossing`: the verbs refuse a moved crossing)

- [x] 3.1 Confirm `run`, `rerun`, `resume` and `compile` refuse before a seat spawns or a prompt exists, in the loader's own words.
- [x] 3.2 Confirm `resume` re-reads the disk as a fence over the world it rehydrated from its run manifest.

## 4. The record (`realm-crossing`: run-manifest/v10 records the observation)

- [x] 4.1 Confirm the observed digests serialize as a sibling of the map pin and the key is absent when nothing is published.
- [x] 4.2 Confirm a crossing moves no bundle digest.

## 5. The readouts (`realm-crossing`: doctor, realms and muninn read the one report)

- [x] 5.1 Confirm `brokkr doctor` reports every crossing and refuses nothing.
- [x] 5.2 Confirm `brokkr realms` renders publications, consumptions and the three pin states in text and `--json`, and stays read-only.
- [x] 5.3 Confirm `brokkr muninn run` carries the report into the dossier and a moved pin is a citable finding under the consuming realm.
- [x] 5.4 Confirm no read surface opens a crossing file, hashes a byte or compares a pin.
- [x] 5.5 Confirm a moved refusal names the publisher's declared repository-relative path and never the host location it resolved to, and that Muninn's crossing evidence states declared paths only.
- [x] 5.6 Confirm the dossier states entries rather than counts, snapshots the crossing evidence into the record, and yields a crossing dossier for a mapped world with no journal yet.
- [x] 5.7 Confirm the record's snapshot keeps the crossing evidence a proposal stood on after the map changes.

## 6. The fold

- [x] 6.1 `openspec validate crossing-enactment --strict --no-interactive`.
- [x] 6.2 Archive through the dialect and append the capability's provenance line.
- [x] 6.3 `openspec validate --archived --strict --no-interactive`.
