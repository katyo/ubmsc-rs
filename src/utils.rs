use crate::{log, Result};
use pretty_hex::PrettyHex;

pub fn checksum(init: Option<u8>, data: impl AsRef<[u8]>) -> u8 {
    data.as_ref()
        .iter()
        .fold(init.unwrap_or(0), |acc, val| acc.wrapping_add(*val))
}

pub fn ascii_to_string(ascii: impl AsRef<[u8]>) -> Result<String> {
    Ok(core::str::from_utf8(ascii.as_ref())?
        .trim_matches(|c| c == '\0')
        .into())
}

pub fn ascii_to_string_safe(field: &str, ascii: impl AsRef<[u8]>) -> String {
    let ascii = ascii.as_ref();
    ascii_to_string(ascii)
        .map_err(|error| {
            log::warn!("Bad string at '{field}': {error:?}");
            log::trace!("{:?}", ascii.hex_dump());
        })
        .unwrap_or_default()
}

pub fn i16le_to_value(raw: &[u8; 2], mul: f32) -> f32 {
    i16::from_le_bytes(*raw) as f32 * mul
}

pub fn i16les_to_values<const N: usize>(raw: &[[u8; 2]; N], mul: f32) -> Vec<f32> {
    raw.iter()
        .filter(|raw| *raw != &[0; 2])
        .map(|raw| i16le_to_value(raw, mul))
        .collect()
}

pub fn i32le_to_value(raw: &[u8; 4], mul: f32) -> f32 {
    i32::from_le_bytes(*raw) as f32 * mul
}

pub fn u32le_to_value(raw: &[u8; 4], mul: f32) -> f32 {
    u32::from_le_bytes(*raw) as f32 * mul
}

pub fn u32le_to_count(raw: &[u8; 4]) -> usize {
    u32::from_le_bytes(*raw) as _
}
