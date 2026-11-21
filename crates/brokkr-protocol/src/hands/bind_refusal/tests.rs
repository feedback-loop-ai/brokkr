//! `bind_refusal.rs`'s own unit tests: each check of a `hands.binds`
//! entry names its own variant, and every variant reads as the text the
//! operator has always seen (#353).

use serde_json::{json, Value};

use super::*;
use crate::hands::{Bind, BindMode, HandsSpec};

/// A read-only bind of `/opt/tool` with `extra` merged over it.
fn entry(extra: Value) -> Value {
    let mut entry = json!({"path": "/opt/tool", "mode": "ro"});
    let fields = entry.as_object_mut().unwrap();
    fields.extend(extra.as_object().unwrap().clone());
    fields.retain(|_, value| !value.is_null());
    entry
}

#[test]
fn each_bind_check_refuses_in_its_own_variant() {
    let refused = |raw: &Value, refusal: BindRefusal<'_>| {
        assert_eq!(Bind::checked(raw), Err(refusal), "{raw}");
    };
    refused(&json!(["/opt/tool"]), BindRefusal::NotObject);
    refused(
        &entry(json!({"size": 3})),
        BindRefusal::UnknownKey { key: "size" },
    );
    refused(&entry(json!({"path": null})), BindRefusal::NoPath);
    refused(&entry(json!({"path": ""})), BindRefusal::NoPath);
    let path = "/opt/tool";
    refused(&entry(json!({"mode": "RO"})), BindRefusal::Mode { path });
    refused(
        &entry(json!({"mask": "token"})),
        BindRefusal::MaskNotArray { path },
    );
    for name in [json!(""), json!("conf/token"), json!(4)] {
        refused(
            &entry(json!({"mask": ["ok", name]})),
            BindRefusal::MaskName { path },
        );
    }
    assert_eq!(
        Bind::checked(&entry(json!({"mode": "overlay", "mask": ["token"]}))),
        Ok(Bind {
            path: path.to_string(),
            mode: BindMode::Overlay,
            mask: vec!["token".to_string()],
        })
    );
}

#[test]
fn every_bind_refusal_reads_as_it_always_has() {
    let path = "~/.cargo";
    for (refusal, text) in [
        (
            BindRefusal::NotObject,
            "hands.binds entry must be an object",
        ),
        (
            BindRefusal::UnknownKey { key: "size" },
            "hands.binds entry has unknown key 'size'; known: path, mode, mask",
        ),
        (
            BindRefusal::NoPath,
            "hands.binds entry needs a non-empty 'path'",
        ),
        (
            BindRefusal::Mode { path },
            "hands.binds entry '~/.cargo' needs mode \"ro\", \"rw\" or \"overlay\"",
        ),
        (
            BindRefusal::MaskNotArray { path },
            "hands.binds entry '~/.cargo' mask must be an array",
        ),
        (
            BindRefusal::MaskName { path },
            "hands.binds entry '~/.cargo' mask names one file each, without a slash",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}

#[test]
fn a_bind_s_unknown_key_names_the_bind_s_vocabulary_not_the_spec_s() {
    let spec = json!({"kind": "workspace", "binds": [entry(json!({"size": 3}))]});
    assert_eq!(
        HandsSpec::parse(&spec),
        Err("hands.binds entry has unknown key 'size'; known: path, mode, mask".to_string())
    );
}
