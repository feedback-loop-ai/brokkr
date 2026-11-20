//! A shape's status and its two enablement rules: each check refuses in
//! its own variant, the admitted shapes pass, and every refusal's text is
//! pinned once, word for word.

use serde_json::{json, Map, Value};

use super::{enablement, status, ResumeRefusal};
use crate::agents::{ResumeEvidence, ResumeIdentity, ResumeStatus};

const WHAT: &str = "adapter 'claude' 'resume' 'session'";

fn raw(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("a test shape is an object, not {other}"),
    }
}

fn measured() -> ResumeIdentity {
    ResumeIdentity::Measured {
        version: "2.1.0".to_owned(),
        applies_to: "2.1.0".to_owned(),
        wrapper_digest: None,
    }
}

fn unknown() -> ResumeIdentity {
    ResumeIdentity::Unknown {
        reason: "nobody has identified the CLI".to_owned(),
    }
}

fn complete() -> ResumeEvidence {
    ResumeEvidence {
        interface: Some("interface.md".to_owned()),
        restrictions: Some("restrictions.md".to_owned()),
        root: Some("root.md".to_owned()),
        accounting: Some("accounting.md".to_owned()),
    }
}

#[test]
fn each_status_token_reads_as_its_status() {
    for (token, expected) in [
        ("unmeasured", ResumeStatus::Unmeasured),
        ("unsupported", ResumeStatus::Unsupported),
        ("supported", ResumeStatus::Supported),
    ] {
        assert_eq!(status(&raw(json!({"status": token})), WHAT), Ok(expected));
    }
}

#[test]
fn an_unknown_missing_or_non_string_status_is_refused_as_status() {
    for shape in [
        json!({"status": "enabled"}),
        json!({}),
        json!({"status": 1}),
    ] {
        assert_eq!(
            status(&raw(shape), WHAT),
            Err(ResumeRefusal::Status { what: WHAT })
        );
    }
}

#[test]
fn a_supported_shape_missing_any_one_evidence_reference_is_incomplete() {
    let gaps: [fn(&mut ResumeEvidence); 4] = [
        |evidence| evidence.interface = None,
        |evidence| evidence.restrictions = None,
        |evidence| evidence.root = None,
        |evidence| evidence.accounting = None,
    ];
    for gap in gaps {
        let mut evidence = complete();
        gap(&mut evidence);
        assert_eq!(
            enablement(ResumeStatus::Supported, &measured(), &evidence, None, WHAT),
            Err(ResumeRefusal::IncompleteEvidence { what: WHAT })
        );
    }
}

#[test]
fn an_unknown_identity_is_refused_before_incomplete_evidence() {
    assert_eq!(
        enablement(
            ResumeStatus::Supported,
            &unknown(),
            &ResumeEvidence::default(),
            None,
            WHAT
        ),
        Err(ResumeRefusal::UnknownIdentity { what: WHAT })
    );
}

#[test]
fn an_unsupported_shape_with_no_reason_is_refused() {
    assert_eq!(
        enablement(
            ResumeStatus::Unsupported,
            &unknown(),
            &ResumeEvidence::default(),
            None,
            WHAT
        ),
        Err(ResumeRefusal::NoReason { what: WHAT })
    );
}

#[test]
fn the_shapes_the_two_rules_admit_are_admitted() {
    let none = ResumeEvidence::default();
    assert_eq!(
        enablement(
            ResumeStatus::Unsupported,
            &unknown(),
            &none,
            Some("we looked"),
            WHAT
        ),
        Ok(())
    );
    assert_eq!(
        enablement(ResumeStatus::Unmeasured, &unknown(), &none, None, WHAT),
        Ok(())
    );
    assert_eq!(
        enablement(
            ResumeStatus::Supported,
            &measured(),
            &complete(),
            None,
            WHAT
        ),
        Ok(())
    );
}

#[test]
fn every_resume_refusal_reads_as_it_always_has() {
    let texts = [
        ResumeRefusal::Status { what: WHAT },
        ResumeRefusal::UnknownIdentity { what: WHAT },
        ResumeRefusal::IncompleteEvidence { what: WHAT },
        ResumeRefusal::NoReason { what: WHAT },
    ]
    .map(|refusal| refusal.to_string());
    assert_eq!(
        texts,
        [
            "adapter 'claude' 'resume' 'session' needs 'status': one of unmeasured, \
             unsupported, supported",
            "adapter 'claude' 'resume' 'session' is 'supported' with an unknown identity; \
             only a MEASURED identity can support enablement, because the per-invocation \
             version check needs a pinned version to compare the observed one against",
            "adapter 'claude' 'resume' 'session' is 'supported' and must name all four \
             evidence references \u{2014} interface, restrictions, root, accounting; one of \
             them does not imply the others",
            "adapter 'claude' 'resume' 'session' is 'unsupported' and needs its measured \
             'reason'; \"we looked and it cannot\" is a different statement from \
             \"nobody looked\"",
        ]
    );
}
