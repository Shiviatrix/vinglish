# Decompilation

The Vinglish codegen backend embeds a lossless MIR payload inside the generated C source. The `vinglish-decompile` crate extracts and verifies this payload, enabling round-trip between C output and the compiler's internal representation.

---

## Purpose

Allow recovery of the full MIR from generated C source files, enabling debugging, analysis, and verification of generated code.

---

## Design

### Payload Embedding (vinglish-codegen)

`emit_mir_c()` in `vinglish-codegen/src/mir_codegen.rs` appends the payload after all C function definitions:

1. Compute SHA-256 hash of the C source (everything before the payload).
2. Serialize the `MirModule` via `bincode`.
3. Bundle `(sha256_hash, module_bytes)` and serialize with `bincode`.
4. Compress with zlib.
5. Encode with base64.
6. Write as a C comment: `/* VINGLISH_MIR_PAYLOAD: <base64> */`.

### Payload Extraction (vinglish-decompile)

`extract_mir_payload()` in `vinglish-decompile/src/lib.rs`:

1. Find the `VINGLISH_MIR_PAYLOAD` comment marker.
2. Extract the base64 string.
3. Compute SHA-256 of the C source preceding the payload.
4. Decode base64 → decompress zlib → deserialize bincode.
5. Compare the stored hash with the computed hash.
6. Return `DecompileError::Desync` if they differ (tamper detection).
7. Return the raw MIR module bytes on success.

### Error Types

| Error | Meaning |
|---|---|
| `MissingPayload` | No `VINGLISH_MIR_PAYLOAD` comment found |
| `Desync` | C source was modified after generation |
| `Base64Decode` | Base64 decoding failed |
| `Decompress` | Zlib decompression failed |
| `Deserialize` | Bincode deserialization failed |

---

## Dependencies

- `sha2`: SHA-256 hashing
- `flate2`: Zlib compression/decompression
- `base64`: Base64 encoding/decoding
- `bincode`: Binary serialization

---

## Limitations

- The payload is a C comment and is discarded by the C preprocessor. It has zero runtime cost.
- Modifying any byte of the C source before the payload invalidates the hash.

---

## Related Components

- [Reference: Code Generation](../reference/codegen.md)
- [Architecture](architecture.md)
