use super::super::{pnpm_flow_map, pnpm_package};
use super::*;

#[test]
fn each_resolution_check_refuses_in_its_own_variant() {
    assert_eq!(
        pnpm_flow_map("integrity: {x: y}"),
        Err(ResolutionRefusal::Nested)
    );
    // The unkeyed field is named as the split yields it, padding kept.
    assert_eq!(
        pnpm_flow_map("tarball: x, integrity"),
        Err(ResolutionRefusal::Unkeyed {
            field: " integrity"
        })
    );
    assert_eq!(
        pnpm_flow_map("shasum: x"),
        Err(ResolutionRefusal::UnknownKey { key: "shasum" })
    );
    // `tarball: ` is an empty scalar, which is not a value.
    assert_eq!(
        pnpm_flow_map("tarball: "),
        Err(ResolutionRefusal::Value { key: "tarball" })
    );
    assert_eq!(
        pnpm_flow_map(" integrity:x "),
        Err(ResolutionRefusal::Unseparated {
            key: "integrity",
            field: "integrity:x"
        })
    );
    assert_eq!(
        pnpm_flow_map("integrity: a[b"),
        Err(ResolutionRefusal::Punctuation {
            key: "integrity",
            value: "a[b"
        })
    );
    assert_eq!(
        pnpm_flow_map("integrity: true"),
        Err(ResolutionRefusal::Typed {
            key: "integrity",
            text: "true",
            kind: "a boolean"
        })
    );
}

#[test]
fn each_package_key_check_refuses_in_its_own_variant() {
    assert_eq!(
        pnpm_package("a b@1"),
        Err(PackageKeyRefusal::Scalar {
            key: "a b@1",
            reason: "carries whitespace"
        })
    );
    assert_eq!(
        pnpm_package("left-pad"),
        Err(PackageKeyRefusal::NoAt { key: "left-pad" })
    );
    assert_eq!(
        pnpm_package("left-pad@"),
        Err(PackageKeyRefusal::NoVersion { key: "left-pad@" })
    );
    assert_eq!(
        pnpm_package("@scope@1"),
        Err(PackageKeyRefusal::NotAName {
            key: "@scope@1",
            name: "@scope"
        })
    );
}

#[test]
fn every_pnpm_leaf_refusal_reads_as_it_always_has() {
    let malformed = "a malformed resolution flow map";
    let resolution = [
        (ResolutionRefusal::Nested, malformed),
        (ResolutionRefusal::Unkeyed { field: "f" }, malformed),
        (ResolutionRefusal::UnknownKey { key: "k" }, malformed),
        (ResolutionRefusal::Value { key: "k" }, malformed),
        (
            ResolutionRefusal::Unseparated {
                key: "integrity",
                field: "integrity:x",
            },
            "the resolution field 'integrity' lacks ': ' separation: 'integrity:x'",
        ),
        (
            ResolutionRefusal::Punctuation {
                key: "tarball",
                value: "a[b",
            },
            "the resolution field 'tarball' carries unsupported flow syntax: 'a[b'",
        ),
        (
            ResolutionRefusal::Typed {
                key: "integrity",
                text: "true",
                kind: "a boolean",
            },
            "the resolution field 'integrity' is the plain scalar 'true', \
             which is a boolean and not a string",
        ),
    ];
    for (refusal, text) in resolution {
        assert_eq!(refusal.to_string(), text);
    }
    let package = [
        (
            PackageKeyRefusal::Scalar {
                key: "a b@1",
                reason: "carries whitespace",
            },
            "'a b@1': the key carries whitespace",
        ),
        (
            PackageKeyRefusal::NoAt { key: "x" },
            "'x': no '@' after the first character",
        ),
        (
            PackageKeyRefusal::NoVersion { key: "x@" },
            "'x@': empty version",
        ),
        (
            PackageKeyRefusal::NotAName {
                key: "@s@1",
                name: "@s",
            },
            "'@s@1': '@s' is not a package name",
        ),
    ];
    for (refusal, text) in package {
        assert_eq!(refusal.to_string(), text);
    }
}
