//! The one source of the startup denial-control targets, shared by the helper
//! executable that performs the attacks and the host-independent check that
//! refuses a profile covering a target (decision 0046 slice II, design D3).
//!
//! The helper includes this file with `#[path]` so its attack functions and
//! the check read the identical constants; a host-independent test pins that
//! equality. The targets are host paths, never payload-writable state.

/// The credential files the credential-read denial control opens. Each is
/// listed in its direct spelling; the check adds the `/private` spelling
/// macOS resolves it to through its one host-independent map.
pub const CREDENTIAL_READ_DENIAL_TARGETS: &[&str] = &["/etc/passwd", "/etc/hosts"];

/// The single path the data-volume credential-read denial control opens. It is
/// the `/System/Volumes/Data` spelling of `/etc/passwd`, which macOS firmlinks.
/// The candidate must deny it exactly as it denies the direct spelling.
pub const DATA_VOLUME_CREDENTIAL_READ_DENIAL_TARGET: &str =
    "/System/Volumes/Data/private/etc/passwd";

/// The path the host-write denial control opens. It is already under
/// `/private`, its resolved spelling.
pub const HOST_WRITE_DENIAL_TARGET: &str = "/private/tmp/brokkr-probe-denial-write";
