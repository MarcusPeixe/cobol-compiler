use super::*;

#[test]
fn line_col_is_one_based() {
    let mut map = SourceMap::default();
    let id = map.add_file("a", "ab\ncd\n\nx".to_string());
    assert_eq!(map.line_col(id, 0), (1, 1));
    assert_eq!(map.line_col(id, 1), (1, 2));
    assert_eq!(map.line_col(id, 3), (2, 1));
    assert_eq!(map.line_col(id, 4), (2, 2));
    assert_eq!(map.line_col(id, 6), (3, 1));
    assert_eq!(map.line_col(id, 7), (4, 1));
}

#[test]
fn line_col_counts_characters_not_bytes() {
    let mut map = SourceMap::default();
    let id = map.add_file("a", "ção x".to_string());
    assert_eq!(map.line_col(id, "ção ".len()), (1, 5));
}

#[test]
fn logical_text_maps_multibyte_chars() {
    let mut t = LogicalText::default();
    t.push_char('ç', FileId(0), 10);
    assert_eq!(t.text.len(), 2);
    assert_eq!(t.origins[0].offset, 10);
    assert_eq!(t.origins[1].offset, 11);
    assert_eq!(t.span(0, 2), Span::new(FileId(0), 10, 12));
}

#[test]
fn synthetic_chars_reuse_origin() {
    let mut t = LogicalText::default();
    let o = Origin {
        file: FileId(1),
        offset: 5,
    };
    t.push_synthetic('\n', o);
    assert_eq!(t.origins, vec![o]);
}

#[test]
fn extend_from_copies_range() {
    let mut a = LogicalText::default();
    a.push_char('x', FileId(0), 0);
    a.push_char('y', FileId(0), 1);
    let mut b = LogicalText::default();
    b.extend_from(&a, 1..2);
    assert_eq!(b.text, "y");
    assert_eq!(b.origins[0].offset, 1);
}
