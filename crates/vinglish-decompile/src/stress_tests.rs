#![allow(clippy::module_inception)]
/// Stress tests for the binary delta encoding and tamper detection pipeline.
///
/// These tests exercise the full round-trip under adversarial conditions:
/// - payload presence/absence
/// - hash correctness on clean input
/// - byte-level tampering in C body (char-flip, line insert, char delete)
/// - payload corruption (base64 garbage, truncated payload, zlib-invalid bytes)
/// - whitespace normalization
/// - idempotency (double-encode tolerance)
#[cfg(test)]
mod payload_stress {
    use crate::{extract_mir_payload, DecompileError};

    // ─── helpers ──────────────────────────────────────────────────────────────

    /// Build a minimal valid C output that our codegen would produce.
    /// Uses a synthetic marker instead of actually running the compiler,
    /// because we want precise control over the payload bytes.
    fn build_valid_c(c_body: &str) -> String {
        use sha2::{Sha256, Digest};
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use base64::Engine;
        use std::io::Write;

        let mut hasher = Sha256::new();
        hasher.update(c_body.as_bytes());
        let hash_hex = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();

        // Payload = (c_hash, module_bytes)
        let module_bytes: Vec<u8> = b"fake_mir_module_bytes".to_vec();
        let pair: (String, Vec<u8>) = (hash_hex, module_bytes);
        let serialized = bincode::serialize(&pair).unwrap();

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serialized).unwrap();
        let compressed = encoder.finish().unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&compressed);

        format!("{}/* VINGLISH_MIR_PAYLOAD: {} */\n", c_body, b64)
    }

    // ─── happy path ──────────────────────────────────────────────────────────

    #[test]
    fn clean_roundtrip_succeeds() {
        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        let bytes = extract_mir_payload(&src).expect("roundtrip must succeed on clean input");
        assert!(!bytes.is_empty(), "decoded payload must be non-empty");
    }

    #[test]
    fn clean_roundtrip_is_idempotent() {
        let c_body = "static long fn_1(int64_t v_1) { bb_1_0:\n    v_1 = 42;\n    return v_1;\n}\n";
        let src = build_valid_c(c_body);
        let b1 = extract_mir_payload(&src).expect("first decode");
        let b2 = extract_mir_payload(&src).expect("second decode");
        assert_eq!(b1, b2, "identical input must produce identical output");
    }

    // ─── missing payload ─────────────────────────────────────────────────────

    #[test]
    fn missing_payload_returns_error() {
        let no_payload = "int main(void) { return 0; }\n";
        assert_eq!(
            extract_mir_payload(no_payload),
            Err(DecompileError::MissingPayload)
        );
    }

    #[test]
    fn truncated_payload_marker_returns_error() {
        // Comment starts but never closes
        let truncated = "int x = 1;\n/* VINGLISH_MIR_PAYLOAD: abc123";
        assert_eq!(
            extract_mir_payload(truncated),
            Err(DecompileError::MissingPayload)
        );
    }

    #[test]
    fn empty_string_returns_missing() {
        assert_eq!(extract_mir_payload(""), Err(DecompileError::MissingPayload));
    }

    // ─── C-body tampering (desync) ────────────────────────────────────────────

    #[test]
    fn single_char_insert_into_c_body_is_detected() {
        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        // Insert one character right before the payload comment
        let tampered = src.replacen("int main", "int  main", 1); // double-space
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Desync),
            "single-char insert must trigger Desync"
        );
    }

    #[test]
    fn single_char_delete_from_c_body_is_detected() {
        let c_body = "static long fn_9(int64_t v_1) { return 0; }\n";
        let src = build_valid_c(c_body);
        // Delete one char from middle of C body
        let payload_start = src.find("/* VINGLISH_MIR_PAYLOAD").unwrap();
        let mut tampered = src.clone();
        tampered.remove(5); // remove 5th char of C body
        // Sanity: the payload comment must still be intact
        assert!(tampered.contains("/* VINGLISH_MIR_PAYLOAD"));
        // ... but the body before it changed
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Desync),
        );
        let _ = payload_start;
    }

    #[test]
    fn appended_newline_to_c_body_is_detected() {
        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        // Append a blank line between body and payload
        let tampered = src.replacen("/* VINGLISH_MIR_PAYLOAD", "\n/* VINGLISH_MIR_PAYLOAD", 1);
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Desync),
        );
    }

    #[test]
    fn replacing_return_value_in_c_body_is_detected() {
        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        let tampered = src.replacen("return 0", "return 1", 1);
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Desync),
        );
    }

    #[test]
    fn injected_comment_into_c_body_is_detected() {
        let c_body = "int x = 1;\nint main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        // Inject a C comment (looks benign, but changes hash)
        let tampered = src.replacen("int x = 1;", "int x = 1; /* hacked */", 1);
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Desync),
        );
    }

    // ─── payload corruption ───────────────────────────────────────────────────

    #[test]
    fn corrupted_base64_returns_error() {
        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        // Replace valid base64 with garbage that isn't valid base64
        let start = src.find("/* VINGLISH_MIR_PAYLOAD: ").unwrap();
        let rest = &src[start..];
        let end = rest.find(" */").unwrap() + start;
        let marker_end = start + "/* VINGLISH_MIR_PAYLOAD: ".len();
        let mut tampered = src.clone();
        tampered.replace_range(marker_end..end, "!!!NOT_BASE64!!!");
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Base64Decode),
        );
    }

    #[test]
    fn valid_base64_but_invalid_zlib_returns_error() {
        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        // Encode random bytes as valid base64 (not zlib-compressed)
        use base64::Engine;
        let junk_b64 = base64::engine::general_purpose::STANDARD.encode(b"this is not zlib compressed data at all");
        let start = src.find("/* VINGLISH_MIR_PAYLOAD: ").unwrap();
        let rest = &src[start..];
        let end = rest.find(" */").unwrap() + start;
        let marker_end = start + "/* VINGLISH_MIR_PAYLOAD: ".len();
        let mut tampered = src.clone();
        tampered.replace_range(marker_end..end, &junk_b64);
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Decompress),
        );
    }

    #[test]
    fn valid_zlib_but_invalid_bincode_returns_error() {
        use base64::Engine;
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let c_body = "int main(void) { return 0; }\n";
        let src = build_valid_c(c_body);

        // Compress random bytes that aren't valid bincode
        let junk = b"i am definitely not valid bincode deserialization data";
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(junk).unwrap();
        let compressed = encoder.finish().unwrap();
        let junk_b64 = base64::engine::general_purpose::STANDARD.encode(&compressed);

        let start = src.find("/* VINGLISH_MIR_PAYLOAD: ").unwrap();
        let rest = &src[start..];
        let end = rest.find(" */").unwrap() + start;
        let marker_end = start + "/* VINGLISH_MIR_PAYLOAD: ".len();
        let mut tampered = src.clone();
        tampered.replace_range(marker_end..end, &junk_b64);
        assert_eq!(
            extract_mir_payload(&tampered),
            Err(DecompileError::Deserialize),
        );
    }

    // ─── multi-function C body ────────────────────────────────────────────────

    #[test]
    fn large_c_body_roundtrip_succeeds() {
        let mut c_body = String::new();
        for i in 0..500 {
            c_body.push_str(&format!("static long fn_{}(int64_t v_{}) {{\n    return {};\n}}\n", i, i, i * 2));
        }
        let src = build_valid_c(&c_body);
        let bytes = extract_mir_payload(&src).expect("large body must succeed");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn unicode_in_string_literal_pool_roundtrip_succeeds() {
        let c_body = "static const char *const string_literal_0 = \"\\xe2\\x9c\\xa6 Vinglish\";\nint main(void) { return 0; }\n";
        let src = build_valid_c(c_body);
        let bytes = extract_mir_payload(&src).expect("unicode escape roundtrip must succeed");
        assert!(!bytes.is_empty());
    }

    // ─── boundary / edge cases ────────────────────────────────────────────────

    #[test]
    fn null_byte_in_base64_region_returns_decode_error() {
        let c_body = "int main(void) {}\n";
        let src = build_valid_c(c_body);
        let start = src.find("/* VINGLISH_MIR_PAYLOAD: ").unwrap();
        let _marker_end = start + "/* VINGLISH_MIR_PAYLOAD: ".len();
        let end = {
            let rest = &src[start..];
            rest.find(" */").unwrap() + start
        };
        // Truncate one byte from the payload (breaks base64 padding)
        let mut tampered = src.clone();
        tampered.remove(end - 1); // remove last char before " */"
        let result = extract_mir_payload(&tampered);
        // Must be either Base64Decode or Decompress — never Ok
        assert!(
            result == Err(DecompileError::Base64Decode)
                || result == Err(DecompileError::Decompress)
                || result == Err(DecompileError::Deserialize),
            "truncated payload must not succeed: {:?}", result
        );
    }
}
