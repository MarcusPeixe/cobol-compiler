use crate::parser::{DataDivision, DataItem, WorkingStorageSection};
use crate::source::{FileId, Span};

use super::{compute_layout, picture_info};

fn span() -> Span {
    Span::new(FileId(0), 0, 1)
}

fn item(level: u8, name: &str, picture: Option<&str>, occurs: Option<&str>) -> DataItem {
    DataItem {
        span: span(),
        level,
        name: name.to_string(),
        picture: picture.map(str::to_string),
        value: None,
        occurs: occurs.map(str::to_string),
        indexed_by: Vec::new(),
        children: Vec::new(),
    }
}

fn division(items: Vec<DataItem>) -> DataDivision {
    DataDivision {
        span: span(),
        file_section: None,
        working_storage_section: Some(WorkingStorageSection {
            span: span(),
            data_items: items,
        }),
    }
}

#[test]
fn picture_sizes_and_scale() {
    let info = picture_info("S9(3)V99").unwrap();
    assert_eq!(
        (info.size, info.digits, info.scale, info.signed, info.edited),
        (5, 5, 2, true, false)
    );
    assert_eq!(info.integer_digits(), 3);
    assert_eq!(picture_info("X(20)").unwrap().size, 20);
    assert_eq!(picture_info("PP99").unwrap().size, 2);
}

#[test]
fn edited_pictures_count_every_position() {
    let info = picture_info("-ZZ9.99").unwrap();
    assert_eq!((info.size, info.digits, info.scale), (7, 5, 2));
    assert!(info.edited && info.signed);
    assert!(picture_info("9(").is_none());
}

#[test]
fn groups_sum_children_and_occurs_multiplies() {
    let mut table = item(2, "ENTRY", None, Some("3"));
    table.children = vec![
        item(3, "ID", Some("9(2)"), None),
        item(3, "NAME", Some("X(5)"), Some("2")),
    ];
    let mut root = item(1, "ROOT", None, None);
    root.children = vec![item(2, "HEAD", Some("X(4)"), None), table];
    let layout = compute_layout(Some(&division(vec![
        root,
        item(77, "COUNTER", Some("9(3)"), None),
    ])));

    let entry = layout.find("entry").unwrap();
    assert_eq!((entry.offset, entry.element_size, entry.size), (4, 12, 36));
    assert_eq!(entry.occurs, Some(3));
    let name = layout.find("NAME").unwrap();
    assert_eq!((name.offset, name.element_size, name.size), (6, 5, 10));
    assert_eq!(layout.find("ROOT").unwrap().size, 40);
    assert_eq!(layout.find("COUNTER").unwrap().offset, 40);
    assert_eq!(layout.working_storage_size, 43);
}

#[test]
fn resolves_qualified_names_and_table_chain() {
    let mut a = item(1, "GROUP-A", None, None);
    a.children = vec![item(5, "NAME", Some("X(5)"), None)];
    let mut b = item(1, "GROUP-B", None, Some("2"));
    b.children = vec![item(5, "NAME", Some("9(3)"), None)];
    let layout = compute_layout(Some(&division(vec![a, b])));

    let in_a = layout.resolve("NAME OF GROUP-A").unwrap();
    assert_eq!(in_a.item().size, 5);
    let in_b = layout.resolve("name in group-b").unwrap();
    assert_eq!((in_b.item().offset, in_b.tables().len()), (5, 1));
    assert_eq!(layout.resolve("NAME").unwrap().item().size, 5);
    assert!(layout.resolve("NAME OF GROUP-C").is_none());
}
