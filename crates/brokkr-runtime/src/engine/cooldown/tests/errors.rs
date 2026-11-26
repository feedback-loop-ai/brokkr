use super::super::CooldownError;

/// Each `CooldownError` variant pins its `Display` once: the operator's
/// text is the variant's, never a string built at the park.
#[test]
fn a_cooldown_error_names_an_unreadable_host() {
    let error = CooldownError::HostUnreadable {
        path: "/tmp/host.json".into(),
        kind: std::io::ErrorKind::PermissionDenied,
    };
    assert_eq!(
        error.to_string(),
        "the host configuration at /tmp/host.json cannot be read: permission denied"
    );
}

#[test]
fn a_cooldown_error_names_an_invalid_host() {
    let source = crate::admission::HostError::Unplaced;
    let error = CooldownError::HostInvalid { source };
    assert_eq!(
        error.to_string(),
        "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME names an absolute \
         directory"
    );
}

#[test]
fn a_cooldown_error_names_an_unreadable_last_wait() {
    let error = CooldownError::LastWaitUnreadable(brokkr_core::ProviderWaitError::Field {
        seq: 4,
        field: "provider".into(),
    });
    assert_eq!(
        error.to_string(),
        "event 4: the waits entry does not read: provider"
    );
}

#[test]
fn a_cooldown_error_names_too_many_waits() {
    let error = CooldownError::TooManyWaits { seq: 5, count: 2 };
    assert_eq!(
        error.to_string(),
        "event 5: a single seat carries at most one wait, not 2"
    );
}

#[test]
fn a_cooldown_error_names_an_unreadable_earlier_wait() {
    let error = CooldownError::EarlierWaitUnreadable {
        seq: 3,
        source: brokkr_core::ProviderWaitError::Field {
            seq: 3,
            field: "seconds".into(),
        },
    };
    assert_eq!(
        error.to_string(),
        "an earlier wait does not read: event 3: the waits entry does not read: seconds"
    );
}

#[test]
fn a_cooldown_error_names_an_unreadable_refusal() {
    let error = CooldownError::RefusedUnreadable(brokkr_core::ProviderRefusalError::Field {
        seq: 6,
        field: "cause".into(),
    });
    assert_eq!(
        error.to_string(),
        "the failed event's refusal does not read: event 6: the refused record does not read: cause"
    );
}
