//! The isolation every box opens with (decision 0065 slice two, U6c3 and
//! U8j), workspace and server alike, before its profile's own mounts.

/// The isolation flags [`super::Namespace::open`] puts in every box's argv,
/// in order. First a user namespace of the box's own in which no further
/// user namespace can be made (U8j; SI1, MB4): a nested `unshare -U`, a
/// nested bubblewrap or a mount namespace made to unmask the store meets
/// the box's own limit (ENOSPC) on every host, never only the host's
/// restriction, and bubblewrap asserts the limit holds before the command
/// runs. A bubblewrap that cannot disable them (before 0.8) refuses the
/// option, so the box does not start. Then the box's own pid, IPC, UTS and
/// cgroup namespaces, and no capability.
pub(super) const ISOLATION: [&str; 9] = [
    "--unshare-user",
    "--disable-userns",
    "--assert-userns-disabled",
    "--unshare-pid",
    "--unshare-ipc",
    "--unshare-uts",
    "--unshare-cgroup-try",
    "--cap-drop",
    "ALL",
];
