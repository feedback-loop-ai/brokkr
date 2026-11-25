use std::path::{Path, PathBuf};

use brokkr_runtime::admission::HostError;

use super::seeded;

/// Ask E's seed never refuses `init`: a host file that cannot be placed,
/// or that its seed cannot write, is a note naming why (1193c's review,
/// finding 3).
#[test]
fn seeding_a_host_file_never_refuses_init() {
    let named = |file: &Path| Ok(vec![file.display().to_string()]);
    let placed = seeded(Ok(PathBuf::from("/h/host.json")), named);
    assert_eq!(placed, ["/h/host.json"]);
    let unplaced = seeded(Err(HostError::Unplaced), named);
    assert_eq!(unplaced.len(), 1, "{unplaced:?}");
    assert!(
        unplaced[0].starts_with("No host file was seeded: no host configuration can be found"),
        "{unplaced:?}"
    );
    let failed = seeded(Ok(PathBuf::from("/h/host.json")), |_| {
        Err(anyhow::anyhow!("denied"))
    });
    assert_eq!(
        failed,
        [
            "No host file was seeded: denied. The .brokkr folder is written; declare a host file \
          by hand (contracts/host.v5.schema.json)."
        ]
    );
}
