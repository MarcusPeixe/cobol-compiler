//! Static data: the initial image of working storage and directive formatting.

use crate::semantic::{DataType, ItemLayout, ItemValue, PictureInfo, StorageLayout};

/// Builds the initial bytes of working storage: zeros in numeric items,
/// spaces elsewhere, then `VALUE` clauses, with `OCCURS` tables replicated.
pub fn initial_image(layout: &StorageLayout) -> Vec<u8> {
    let mut image = vec![b' '; layout.working_storage_size];
    for item in &layout.working_storage {
        init_item(item, &mut image);
    }
    image
}

fn init_item(item: &ItemLayout, image: &mut [u8]) {
    let start = item.offset;
    let end = start + item.element_size;
    if item.children.is_empty() {
        let info = item.picture.as_ref();
        let numeric = info.is_some_and(|info| info.data_type == DataType::Numeric && !info.edited);
        let fill = if numeric { b'0' } else { b' ' };
        image[start..end].fill(fill);
        if let Some(value) = &item.value {
            let bytes = value_bytes(value, info, item.element_size);
            image[start..end].copy_from_slice(&bytes);
        }
    } else {
        for child in &item.children {
            init_item(child, image);
        }
    }
    for occurrence in 1..item.occurs.unwrap_or(1) {
        image.copy_within(start..end, start + occurrence * item.element_size);
    }
}

fn value_bytes(value: &ItemValue, info: Option<&PictureInfo>, size: usize) -> Vec<u8> {
    let numeric = info.filter(|info| info.data_type == DataType::Numeric && !info.edited);
    let mut bytes = match (value, numeric) {
        (ItemValue::Numeric(text), Some(info)) => encode_numeric(info, text),
        (ItemValue::Figurative(word), Some(info)) if is_zero(word) => encode_numeric(info, "0"),
        (ItemValue::Figurative(word), _) => vec![figurative_byte(word); size],
        (ItemValue::String(text) | ItemValue::Numeric(text), _) => text.as_bytes().to_vec(),
    };
    bytes.resize(size, b' ');
    bytes
}

fn is_zero(word: &str) -> bool {
    matches!(word, "ZERO" | "ZEROS" | "ZEROES")
}

/// The byte a figurative constant stands for.
pub fn figurative_byte(word: &str) -> u8 {
    match word {
        "ZERO" | "ZEROS" | "ZEROES" => b'0',
        "HIGH-VALUE" | "HIGH-VALUES" => 0xFF,
        "LOW-VALUE" | "LOW-VALUES" | "NULL" | "NULLS" => 0,
        "QUOTE" | "QUOTES" => b'"',
        _ => b' ',
    }
}

/// Encodes a numeric literal (`-12.5`) as the DISPLAY digits of a field.
/// Digits beyond the field are truncated; a negative value in a signed field
/// overpunches the last digit.
pub fn encode_numeric(info: &PictureInfo, text: &str) -> Vec<u8> {
    let negative = text.starts_with('-');
    let unsigned = text.trim_start_matches(['+', '-']);
    let (int_part, frac_part) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let int_digits = info.integer_digits();
    let mut digits: Vec<u8> = Vec::with_capacity(info.digits);
    let int_bytes = int_part.as_bytes();
    let kept = &int_bytes[int_bytes.len().saturating_sub(int_digits)..];
    digits.extend(std::iter::repeat_n(b'0', int_digits - kept.len()));
    digits.extend_from_slice(kept);
    let frac = frac_part.as_bytes();
    for position in 0..info.scale {
        digits.push(frac.get(position).copied().unwrap_or(b'0'));
    }
    if negative
        && info.signed
        && let Some(last) = digits.last_mut()
    {
        *last += 0x40;
    }
    digits
}

/// Formats bytes as the cheapest data directives, e.g. `.ascii "ab"`.
pub fn data_directives(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let value = bytes[index];
        let run = bytes[index..].iter().take_while(|&&b| b == value).count();
        if run >= 4 {
            if value == 0 {
                out.push_str(&format!("    .space {run}\n"));
            } else {
                out.push_str(&format!("    .byte {value}:{run}\n"));
            }
            index += run;
            continue;
        }
        let mut end = index;
        while end < bytes.len() && end - index < 64 {
            let next = bytes[end];
            if bytes[end..].iter().take_while(|&&b| b == next).count() >= 4 {
                break;
            }
            end += 1;
        }
        out.push_str(&literal_directive(&bytes[index..end], false));
        index = end;
    }
    out
}

/// `.ascii`/`.asciiz` when the bytes are printable, otherwise `.byte`.
pub fn literal_directive(bytes: &[u8], terminate: bool) -> String {
    if bytes.iter().all(|b| (0x20..0x7F).contains(b)) {
        let mut escaped = String::new();
        for &b in bytes {
            match b {
                b'"' => escaped.push_str("\\\""),
                b'\\' => escaped.push_str("\\\\"),
                _ => escaped.push(b as char),
            }
        }
        let directive = if terminate { ".asciiz" } else { ".ascii" };
        return format!("    {directive} \"{escaped}\"\n");
    }
    let mut values: Vec<String> = bytes.iter().map(u8::to_string).collect();
    if terminate {
        values.push("0".to_string());
    }
    format!("    .byte {}\n", values.join(", "))
}
