use super::{NameRefusal, ReferenceRefusal, ValueRefusal};
use crate::secret::{
    checked_name, checked_refs, checked_value, scan_secret_refs, validate_name, validate_value,
};

/// `{{secret:<name>` with `tail` after it, built so no reference array
/// elsewhere is restated.
fn reference(name: &str, tail: &str) -> String {
    format!("{{{{secret:{name}{tail}")
}

#[test]
fn each_name_check_refuses_in_its_own_variant() {
    for name in ["lower", "9BAD", "A-B"] {
        assert_eq!(checked_name(name), Err(NameRefusal::Grammar { name }));
    }
    for name in ["PATH", "LD_PRELOAD", "BROKKR_", "BROKKR_X"] {
        assert_eq!(checked_name(name), Err(NameRefusal::Denylisted { name }));
    }
    assert_eq!(checked_name("GH_TOKEN"), Ok(()));
}

#[test]
fn each_value_check_refuses_in_its_own_variant() {
    assert_eq!(checked_value(""), Err(ValueRefusal::Empty));
    for value in ["two\nlines", "two\rlines"] {
        assert_eq!(checked_value(value), Err(ValueRefusal::MultiLine));
    }
    assert_eq!(checked_value("nul\0byte"), Err(ValueRefusal::Nul));
    assert_eq!(checked_value("abc"), Err(ValueRefusal::TooShort));
    assert_eq!(
        checked_value("abcd"),
        Ok(Some(
            "warning: secret value is shorter than 8 bytes; short values mask aggressively"
                .to_string()
        ))
    );
    assert_eq!(checked_value("12345678"), Ok(None));
}

#[test]
fn each_reference_check_refuses_in_its_own_variant() {
    for space in [" ", "\t"] {
        let text = format!("{{{{{space}secret:NAME}}}}");
        assert_eq!(checked_refs(&text), Err(ReferenceRefusal::Whitespace));
    }
    for (text, quoted) in [
        (reference("gh", "}}"), "{{secret:gh"),
        (reference("", "}}"), "{{secret:}}"),
        (reference("9NAME", "}}"), "{{secret:9NAME}}"),
        (reference("", ""), "{{secret:"),
    ] {
        assert_eq!(
            checked_refs(&text),
            Err(ReferenceRefusal::Name { quoted }),
            "{text}"
        );
    }
    for (text, name) in [
        (reference("NAME", ""), "NAME"),
        (reference("GH_TOKENs", "}}"), "GH_TOKEN"),
    ] {
        assert_eq!(
            checked_refs(&text),
            Err(ReferenceRefusal::Unclosed { name }),
            "{text}"
        );
    }
}

#[test]
fn the_checks_run_in_their_order() {
    assert_eq!(
        checked_name("BROKKR_lower"),
        Err(NameRefusal::Grammar {
            name: "BROKKR_lower"
        })
    );
    assert_eq!(checked_value("\n"), Err(ValueRefusal::MultiLine));
    assert_eq!(checked_value("a\0"), Err(ValueRefusal::Nul));
    assert_eq!(checked_value("\n\0abcd"), Err(ValueRefusal::MultiLine));
    let spaced = "{{ secret:x}}";
    assert_eq!(checked_refs(spaced), Err(ReferenceRefusal::Whitespace));
    let second = format!("{} {spaced}", reference("A", "}}"));
    assert_eq!(checked_refs(&second), Err(ReferenceRefusal::Whitespace));
    let first = format!("{} {spaced}", reference("a", "}}"));
    assert_eq!(
        checked_refs(&first),
        Err(ReferenceRefusal::Name {
            quoted: "{{secret:a}"
        })
    );
}

#[test]
fn well_formed_references_name_each_in_order() {
    assert_eq!(
        checked_refs(&reference("GH_TOKEN", "}}")),
        Ok(vec!["GH_TOKEN".to_string()])
    );
    let two = format!("{}{}", reference("A", "}}"), reference("B_2", "}}"));
    assert_eq!(
        checked_refs(&two),
        Ok(vec!["A".to_string(), "B_2".to_string()])
    );
    for text in ["secret:NAME", "https://x/secret:thing"] {
        assert_eq!(checked_refs(text), Ok(Vec::new()), "{text}");
    }
}

/// Every refusal reads, through the public functions, as the text main
/// has always printed, written out by hand: Whitespace's four braces
/// included.
#[test]
fn every_secret_refusal_reads_as_it_always_has() {
    for (name, text) in [
        (
            "lower",
            "secret name 'lower' does not match [A-Z][A-Z0-9_]*",
        ),
        (
            "PATH",
            "secret name 'PATH' is denylisted (PATH, IFS, LD_PRELOAD, \
             LD_LIBRARY_PATH, and the BROKKR_ prefix are never bindable)",
        ),
    ] {
        assert_eq!(validate_name(name), Err(text.to_string()));
    }
    for (value, text) in [
        ("", "secret value is empty"),
        (
            "a\nb",
            "secret value must be a single line (the store is line-oriented)",
        ),
        ("a\0b", "secret value must not contain NUL"),
        (
            "abc",
            "secret value is shorter than 4 bytes; masking a tiny value \
             shreds the journal's evidence trail",
        ),
    ] {
        assert_eq!(validate_value(value), Err(text.to_string()));
    }
    for (template, text) in [
        (
            "{{ secret:NAME}}",
            "malformed secret reference near '{{ secret:': whitespace \
             is not allowed inside a reference",
        ),
        (
            "{{secret:gh}}",
            "malformed secret reference '{{secret:gh': expected \
             {{secret:NAME}} with NAME matching [A-Z][A-Z0-9_]*",
        ),
        (
            "{{secret:GH_TOKENs}}",
            "malformed secret reference '{{secret:GH_TOKEN': missing the closing \
             '}}' (or the name continues with characters outside [A-Z0-9_])",
        ),
    ] {
        assert_eq!(scan_secret_refs(template), Err(text.to_string()));
    }
}

/// A refused name's quote ends two bytes past the name, rounded up to the
/// next character boundary, so a multibyte character there is quoted
/// whole instead of panicking the scanner.
#[test]
fn a_multibyte_character_after_a_refused_name_is_quoted_whole() {
    for (text, quoted) in [
        (reference("€", "}}"), "{{secret:€"),
        (reference("a€", ""), "{{secret:a€"),
        (reference("9€", "}}"), "{{secret:9€"),
        (reference("é", "}}"), "{{secret:é"),
    ] {
        assert_eq!(
            checked_refs(&text),
            Err(ReferenceRefusal::Name { quoted }),
            "{text}"
        );
    }
    assert_eq!(
        scan_secret_refs("{{secret:€}}"),
        Err("malformed secret reference '{{secret:€': expected \
             {{secret:NAME}} with NAME matching [A-Z][A-Z0-9_]*"
            .to_string())
    );
}
