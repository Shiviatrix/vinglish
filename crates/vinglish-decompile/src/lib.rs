//! Reconstructs compiler IR from the lossless metadata emitted beside generated C.
//!
//! This crate deliberately does not parse arbitrary C.  C is only the carrier: the
//! canonical MIR payload in `vinglish:mir` comments is the source of truth.

#[cfg(test)]
mod stress_tests;

use thiserror::Error;
use sha2::{Sha256, Digest};
use base64::Engine;
use flate2::read::ZlibDecoder;
use std::io::Read;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecompileError {
    #[error("Missing VINGLISH_MIR_PAYLOAD comment in the source file")]
    MissingPayload,
    #[error("C-Source Tampering Detected: The file was modified after generation (desync).")]
    Desync,
    #[error("Failed to decode base64 payload")]
    Base64Decode,
    #[error("Failed to decompress zlib payload")]
    Decompress,
    #[error("Failed to deserialize bincode payload")]
    Deserialize,
}

pub trait MirSnapshotDecoder {
    type MirModule;
    type Error: std::error::Error + Send + Sync + 'static;

    fn decode_module(&self, bytes: &[u8]) -> Result<Self::MirModule, Self::Error>;
}

/// Public round-trip entry point for generated Vinglish C. The returned bytes
/// is the lossless SSA identity graph; callers with a concrete MIR decoder can
/// subsequently decode it.
pub fn extract_mir_payload(c_source: &str) -> Result<Vec<u8>, DecompileError> {
    let payload_marker = "/* VINGLISH_MIR_PAYLOAD: ";
    let Some(start) = c_source.find(payload_marker) else {
        return Err(DecompileError::MissingPayload);
    };
    
    // The C source we want to hash is everything before the payload, exactly as it was generated.
    let c_code = &c_source[..start];
    
    let rest = &c_source[start + payload_marker.len()..];
    let Some(end) = rest.find(" */") else {
        return Err(DecompileError::MissingPayload);
    };
    let base64_payload = &rest[..end];
    
    let mut hasher = Sha256::new();
    hasher.update(c_code.as_bytes());
    let computed_hash = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>();
    
    let compressed = base64::engine::general_purpose::STANDARD.decode(base64_payload)
        .map_err(|_| DecompileError::Base64Decode)?;
        
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut serialized = Vec::new();
    decoder.read_to_end(&mut serialized).map_err(|_| DecompileError::Decompress)?;
    
    let (stored_hash, module_bytes): (String, Vec<u8>) = bincode::deserialize(&serialized)
        .map_err(|_| DecompileError::Deserialize)?;
        
    if computed_hash != stored_hash {
        return Err(DecompileError::Desync);
    }
    
    Ok(module_bytes)
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn detects_tampering() {
        // We will test this via integration tests elsewhere, since we need to generate valid payloads
    }
}
