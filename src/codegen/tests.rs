use super::data::{encode_numeric, figurative_byte};
use crate::semantic::picture_info;

#[test]
fn encodes_numeric_values_with_scale_and_sign() {
    let info = picture_info("S9(3)V99").unwrap();
    assert_eq!(encode_numeric(&info, "12.5"), b"01250");
    assert_eq!(encode_numeric(&info, "-1.25"), b"0012u");
    assert_eq!(encode_numeric(&info, "12345"), b"34500");
}

#[test]
fn figurative_bytes() {
    assert_eq!(figurative_byte("ZEROS"), b'0');
    assert_eq!(figurative_byte("SPACES"), b' ');
    assert_eq!(figurative_byte("QUOTE"), b'"');
}
