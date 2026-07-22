//! Reconstructs compiler IR from the lossless metadata emitted beside generated C.
//!
//! This crate deliberately does not parse arbitrary C.  C is only the carrier: the
//! canonical MIR payload in `vinglish:mir` comments is the source of truth.

use std::collections::BTreeMap;
use thiserror::Error;

/// Stable identity of one emitted MIR operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MirInstructionId {
    pub function: u32,
    pub block: u32,
    pub instruction: u32,
}

/// One zero-runtime C comment. `payload` is a canonical, versioned MIR encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirTag {
    pub format_version: u16,
    pub module_fingerprint: String,
    pub id: MirInstructionId,
    pub opcode: String,
    pub payload: String,
}

/// The complete index collected from a generated translation unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconstructionIndex {
    pub records: BTreeMap<MirInstructionId, MirTag>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecompileError {
    #[error("invalid vinglish MIR tag on C line {line}: {reason}")]
    InvalidTag { line: usize, reason: String },
    #[error("duplicate MIR instruction identity {0:?}")]
    DuplicateInstruction(MirInstructionId),
    #[error("metadata format {found} is not supported (expected {expected})")]
    UnsupportedVersion { found: u16, expected: u16 },
    #[error("MIR snapshot decoding failed: {0}")]
    Decode(String),
}

/// Decodes the canonical payloads into the compiler's concrete MIR representation.
///
/// Keeping this trait here prevents the decompiler from guessing symbol-table or
/// generic value-ID layouts. The codegen crate owns the matching encoder.
pub trait MirSnapshotDecoder {
    type MirModule;
    type Error: std::error::Error + Send + Sync + 'static;

    fn decode_module(&self, records: &[MirTag]) -> Result<Self::MirModule, Self::Error>;
}

impl ReconstructionIndex {
    pub const FORMAT_VERSION: u16 = 1;

    /// Read only `/* vinglish:mir ... */` comments. All normal C is ignored.
    pub fn parse_generated_c(c_source: &str) -> Result<Self, DecompileError> {
        let mut index = Self::default();
        for (offset, line) in c_source.lines().enumerate() {
            let line_number = offset + 1;
            let Some(start) = line.find("/* vinglish:mir ") else { continue };
            let rest = &line[start + "/* vinglish:mir ".len()..];
            let Some(body) = rest.strip_suffix(" */") else {
                return Err(DecompileError::InvalidTag { line: line_number, reason: "missing comment terminator".into() });
            };
            let tag = parse_tag(body, line_number)?;
            if tag.format_version != Self::FORMAT_VERSION {
                return Err(DecompileError::UnsupportedVersion { found: tag.format_version, expected: Self::FORMAT_VERSION });
            }
            if index.records.insert(tag.id, tag.clone()).is_some() {
                return Err(DecompileError::DuplicateInstruction(tag.id));
            }
        }
        Ok(index)
    }

    pub fn reconstruct<D: MirSnapshotDecoder>(&self, decoder: &D) -> Result<D::MirModule, DecompileError> {
        let records = self.records.values().cloned().collect::<Vec<_>>();
        decoder.decode_module(&records).map_err(|e| DecompileError::Decode(e.to_string()))
    }
}

/// Public round-trip entry point for generated Vinglish C. The returned index
/// is the lossless SSA identity graph; callers with a concrete MIR decoder can
/// subsequently call `ReconstructionIndex::reconstruct`.
pub fn reconstruct_mir(c_source: &str) -> Result<ReconstructionIndex, DecompileError> {
    ReconstructionIndex::parse_generated_c(c_source)
}

fn parse_tag(body: &str, line: usize) -> Result<MirTag, DecompileError> {
    let mut fields = BTreeMap::new();
    for part in body.split_whitespace() {
        let Some((key, value)) = part.split_once('=') else {
            return Err(DecompileError::InvalidTag { line, reason: format!("expected key=value, got {part:?}") });
        };
        fields.insert(key, value);
    }
    let required = |key: &str| fields.get(key).copied().ok_or_else(|| DecompileError::InvalidTag { line, reason: format!("missing {key}") });
    let number = |key: &str| required(key)?.parse::<u32>().map_err(|_| DecompileError::InvalidTag { line, reason: format!("{key} must be an unsigned integer") });
    let version = required("v")?.parse::<u16>().map_err(|_| DecompileError::InvalidTag { line, reason: "v must be an unsigned integer".into() })?;
    Ok(MirTag {
        format_version: version,
        module_fingerprint: required("module")?.to_owned(),
        id: MirInstructionId { function: number("fn")?, block: number("bb")?, instruction: number("inst")? },
        opcode: required("op")?.to_owned(),
        payload: required("payload")?.to_owned(),
    })
}

/// Format a tag for placement immediately before the C statement it describes.
pub fn emit_c_tag(tag: &MirTag) -> String {
    format!(
        "/* vinglish:mir v={} module={} fn={} bb={} inst={} op={} payload={} */",
        tag.format_version, tag.module_fingerprint, tag.id.function, tag.id.block,
        tag.id.instruction, tag.opcode, tag.payload
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_comments_and_ignores_c() {
        let c = "/* vinglish:mir v=1 module=a1 fn=7 bb=2 inst=3 op=Add payload=deadbeef */\nlong x = 1;";
        let index = ReconstructionIndex::parse_generated_c(c).unwrap();
        assert_eq!(index.records.len(), 1);
        assert_eq!(index.records.values().next().unwrap().opcode, "Add");
    }
}
