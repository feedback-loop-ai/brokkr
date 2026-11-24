use super::*;

#[test]
fn base64url_round_trips_rfc4648_vectors_non_ascii_and_url_symbols() {
    for (raw, expected) in [
        (&b""[..], ""),
        (b"f", "Zg"),
        (b"fo", "Zm8"),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg"),
        (b"fooba", "Zm9vYmE"),
        (b"foobar", "Zm9vYmFy"),
        (&[0xfb, 0xff], "-_8"),
    ] {
        let encoded = encode_base64url(raw);
        assert_eq!(encoded, expected);
        assert_eq!(decode_url_nopad(&encoded), Ok(raw.to_vec()));
    }
    let non_ascii = "/home/valentin/work\u{00e9}/\u{00be}\u{00be}\u{00be}\u{00bf}\u{00bf}\u{00bf}/\u{65e5}\u{672c}\u{8a9e}";
    let encoded = encode_base64url(non_ascii.as_bytes());
    assert!(encoded.contains('-'), "expected - in {encoded}");
    assert!(encoded.contains('_'), "expected _ in {encoded}");
    assert_eq!(decode_base64url(&encoded), Ok(non_ascii.to_string()));
}

#[test]
fn base64url_refuses_padding_bad_alphabet_bad_length_trailing_bits_and_non_utf8() {
    let padded = "Zg==";
    assert_eq!(decode_base64url(padded), Err(Base64UrlError::Padding));
    assert_eq!(decode_base64url("Zm8="), Err(Base64UrlError::Padding));
    assert_eq!(decode_base64url("Z=g"), Err(Base64UrlError::Padding));
    assert_eq!(
        Base64UrlError::Padding.to_string(),
        "contains '=' padding, which unpadded base64url forbids"
    );

    for bad in ["+/8", "Zm9v\n", "Zm9v ", "Zm9v!", "Zm9v\u{00e9}"] {
        assert_eq!(
            decode_base64url(bad),
            Err(Base64UrlError::InvalidAlphabet),
            "{bad:?}"
        );
    }
    assert_eq!(
        Base64UrlError::InvalidAlphabet.to_string(),
        "contains a byte outside the base64url alphabet"
    );

    for bad in ["A", "Zm9vA"] {
        assert_eq!(
            decode_base64url(bad),
            Err(Base64UrlError::InvalidLength),
            "{bad:?}"
        );
    }
    assert_eq!(
        Base64UrlError::InvalidLength.to_string(),
        "has an invalid base64url length"
    );

    // 2-char tail: low 4 bits of second sextet non-zero (B = 1).
    assert_eq!(
        decode_base64url("AB"),
        Err(Base64UrlError::NonZeroTrailingBits)
    );
    // 3-char tail: low 2 bits of third sextet non-zero (B = 1).
    assert_eq!(
        decode_base64url("AAB"),
        Err(Base64UrlError::NonZeroTrailingBits)
    );
    assert_eq!(
        Base64UrlError::NonZeroTrailingBits.to_string(),
        "has non-zero trailing base64url bits"
    );

    let invalid_utf8 = encode_base64url(&[0xff, 0xfe]);
    assert_eq!(
        decode_base64url(&invalid_utf8),
        Err(Base64UrlError::NotUtf8)
    );
    assert_eq!(
        Base64UrlError::NotUtf8.to_string(),
        "decoded bytes are not valid UTF-8"
    );

    // No refusal text ever echoes the rejected value.
    let sentinel = "SECRET_CANARY_VALUE";
    for (input, expected) in [
        (format!("{sentinel}="), Base64UrlError::Padding),
        (format!("{sentinel}!"), Base64UrlError::InvalidAlphabet),
        (format!("{sentinel}AA"), Base64UrlError::InvalidLength),
    ] {
        let error = decode_base64url(&input).unwrap_err();
        assert_eq!(error, expected);
        assert!(!error.to_string().contains(sentinel), "{error}");
    }
}
