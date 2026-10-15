//! The server box as provisioning observes it (decision 0065 slice two,
//! U7c0b; MB3): write exclusion judged against the writers the plan seals,
//! taken as confined, beside the kernel's overflow uid, never the
//! observing process chain; each other condition of the proof holding as
//! it does at admission.

use std::os::unix::fs::MetadataExt;

use super::super::namespace::sources::host::*;
use super::super::*;
use super::server::{unservable, Host as Fixture};
use crate::broker::{Network, Reach, Refusal};

/// A status a writer that is not confined reports: uid 1000, no
/// capability, `no_new_privs` unset, as an ordinary engine or test runs.
const UNCONFINED: &str = "Name:\tbrokkr\nUid:\t1000\t1000\t1000\t1000\n\
                          CapPrm:\t0000000000000000\nCapEff:\t0000000000000000\n\
                          CapAmb:\t0000000000000000\nNoNewPrivs:\t0\n";

/// A root-owned support file no one else may write, with no ACL: what a
/// multiply-linked system file such as `/usr/bin/coreutils` reports.
const ROOTED: Owned = Owned {
    uid: 0,
    mode: 0o100_755,
    acl: Acl::Absent,
};

#[test]
fn provisioning_credentials_are_the_sealed_writers_confined_beside_the_overflow_uid() {
    let sealed = provisioned(Some("65534\n"), &[1002, 1000, 1002]);
    let expected = Credentials {
        uids: vec![1000, 1002],
        overflow: 65534,
        confinement: Confinement::Proved,
    };
    assert_eq!(sealed, expected);
    // No writer sealed, or an overflow uid that does not read, leaves no
    // writer known, exactly as an unread observation does.
    let unknown = credited(None, &[]);
    assert_eq!(
        unknown,
        Credentials {
            uids: Vec::new(),
            overflow: u32::MAX,
            confinement: Confinement::Unproved,
        }
    );
    assert_eq!(provisioned(Some("65534\n"), &[]), unknown);
    for overflow in [None, Some("65534"), Some("x\n"), Some("")] {
        assert_eq!(
            (overflow, provisioned(overflow, &[1000])),
            (overflow, unknown.clone())
        );
    }
}

#[test]
fn sealed_confined_writers_admit_a_root_owned_linked_file_the_unconfined_chain_refuses() {
    let chain = credited(Some("65534\n"), &[Some(UNCONFINED)]);
    assert_eq!(chain.confinement, Confinement::Unproved);
    assert!(!excluded(&ROOTED, &chain.sealed(&[1000])));
    assert!(!untouchable(&ROOTED, &chain.sealed(&[1000])));
    let sealed = provisioned(Some("65534\n"), &[1000]);
    assert!(excluded(&ROOTED, &sealed));
    assert!(untouchable(&ROOTED, &sealed));
    // Every other condition still refuses, as at admission, where the
    // chain observed confined joins the sealed writers.
    let fenced = UNCONFINED.replace("NoNewPrivs:\t0", "NoNewPrivs:\t1");
    let admission = credited(Some("65534\n"), &[Some(&fenced)]).sealed(&[1000]);
    let file = |uid, mode, acl| Owned { uid, mode, acl };
    let refused = [
        (
            "a sealed writer owns it",
            file(1000, 0o100_755, Acl::Absent),
            &sealed,
        ),
        (
            "group may write it",
            file(0, 0o100_775, Acl::Absent),
            &sealed,
        ),
        (
            "other may write it",
            file(0, 0o100_757, Acl::Absent),
            &sealed,
        ),
        ("an access ACL", file(0, 0o100_755, Acl::Present), &sealed),
        ("an unread ACL", file(0, 0o100_755, Acl::Unknown), &sealed),
        (
            "an unmapped owner",
            file(65534, 0o100_755, Acl::Absent),
            &sealed,
        ),
    ];
    for (case, file, writers) in refused {
        assert_eq!((case, excluded(&file, writers)), (case, false));
        assert_eq!((case, excluded(&file, &admission)), (case, false));
        assert_eq!((case, untouchable(&file, writers)), (case, false));
    }
    let overflowing = provisioned(Some("65534\n"), &[1000, 65534]);
    assert!(!excluded(&ROOTED, &overflowing));
    for unknown in [
        provisioned(Some("65534\n"), &[]),
        provisioned(None, &[1000]),
    ] {
        assert!(!excluded(&ROOTED, &unknown));
        assert!(!untouchable(&ROOTED, &unknown));
    }
}

/// A multiply-linked regular file directly in `/usr/bin` or `/usr/sbin`,
/// other than `except`, where this host has one.
fn linked_system_file(except: &Path) -> Option<PathBuf> {
    let entries = ["/usr/bin", "/usr/sbin"]
        .into_iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.filter_map(|entry| Some(entry.ok()?.path())));
    let mut linked = entries.filter(|path| {
        let held = path.symlink_metadata();
        held.is_ok_and(|held| held.is_file() & (held.nlink() > 1)) & (path != except)
    });
    linked.next()
}

/// Why this run cannot contrast the two observations, if it cannot: in a
/// seat's hands box no box stands; as root, the sealed writer owns the
/// system set; under a confined chain, `prepare` admits too; and with no
/// multiply-linked system file, nothing tells them apart.
fn uncontrasted(sh: &Path) -> Option<&'static str> {
    if rustix::process::geteuid().is_root() {
        return Some("euid 0 owns the system set, so its sealed writer owns every file");
    }
    if Host::live().credentials.confinement == Confinement::Proved {
        return Some("this process chain is confined, so prepare admits linked files too");
    }
    let none = linked_system_file(sh).is_none();
    none.then_some("no multiply-linked file lies in /usr/bin or /usr/sbin here")
}

#[test]
fn over_a_real_linked_system_file_provisioning_admits_where_the_unconfined_prepare_refuses() {
    if unservable() {
        return;
    }
    let fixture = Fixture::new();
    let sh = ServerProgram::resolve("sh", &fixture.home).unwrap();
    if let Some(reason) = uncontrasted(sh.executable()) {
        return skip_boundary_proof(boundary_evidence_required(), reason);
    }
    let bootstrap = std::env::current_exe().unwrap();
    let euid = [rustix::process::geteuid().as_raw()];
    let reach = Reach {
        writable: vec![fixture.path("work")],
        readable: Vec::new(),
    };
    let profile = ServerProfile {
        reach: &reach,
        network: &Network::Isolated,
        bootstrap: &bootstrap,
        writers: &euid,
    };
    assert_eq!(
        ServerBox::prepare(&sh, &profile).err(),
        Some(Refusal::Identity)
    );
    let provisioned = ServerBox::provision(&sh, &profile).unwrap();
    let confined = ServerBox::prepare_with(&sh, &profile, &super::sources::confined()).unwrap();
    assert_eq!(&provisioned, confined.sources());
    assert!(provisioned.entries > 0, "{provisioned:?}");
    // No writer sealed proves no linked file protected.
    let unsealed = ServerProfile {
        writers: &[],
        ..profile
    };
    assert_eq!(ServerBox::provision(&sh, &unsealed), Err(Refusal::Identity));
}
