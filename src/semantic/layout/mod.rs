//! Storage layout: byte size and offset of every data item.
//!
//! Every item uses DISPLAY usage (one byte per picture position). Sign (`S`),
//! implied decimal point (`V`) and scaling (`P`) positions occupy no storage.

use crate::parser::{DataDivision, DataItem, Literal};

use super::DataType;
use super::type_system::parse_picture;

/// Size and numeric shape derived from a PICTURE string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureInfo {
    pub data_type: DataType,
    /// Bytes of storage occupied by one value.
    pub size: usize,
    /// Digit positions (numeric and numeric-edited items).
    pub digits: usize,
    /// Digit positions after the (implied or explicit) decimal point.
    pub scale: usize,
    pub signed: bool,
    /// Contains insertion/editing symbols (`Z`, `.`, `,`, `$`, `B`, ...).
    pub edited: bool,
    /// The picture with repetition counts expanded and `S`/`V`/`P` removed
    /// (one character per storage byte for edited items).
    pub pattern: String,
}

impl PictureInfo {
    pub fn integer_digits(&self) -> usize {
        self.digits.saturating_sub(self.scale)
    }
}

/// Parses a PICTURE string. Returns `None` for malformed repetition counts.
pub fn picture_info(picture: &str) -> Option<PictureInfo> {
    let mut symbols: Vec<(char, usize)> = Vec::new();
    let mut chars = picture.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '(' {
            let mut digits = String::new();
            while let Some(next) = chars.peek().copied().filter(char::is_ascii_digit) {
                digits.push(next);
                chars.next();
            }
            let count: usize = digits.parse().ok()?;
            if chars.next() != Some(')') {
                return None;
            }
            let last = symbols.last_mut()?;
            last.1 = last.1.checked_add(count.checked_sub(1)?)?;
        } else {
            symbols.push((ch.to_ascii_uppercase(), 1));
        }
    }

    let mut info = PictureInfo {
        data_type: parse_picture(picture),
        size: 0,
        digits: 0,
        scale: 0,
        signed: false,
        edited: false,
        pattern: String::new(),
    };
    let mut after_point = false;
    for (symbol, count) in symbols {
        if !matches!(symbol, 'S' | 'V' | 'P') {
            info.pattern.extend(std::iter::repeat_n(symbol, count));
        }
        match symbol {
            '9' => {
                info.size += count;
                info.digits += count;
                if after_point {
                    info.scale += count;
                }
            }
            'Z' | '*' => {
                info.size += count;
                info.digits += count;
                info.edited = true;
                if after_point {
                    info.scale += count;
                }
            }
            'A' | 'X' => info.size += count,
            'S' => info.signed = true,
            'V' => after_point = true,
            'P' => {}
            '.' => {
                info.size += count;
                info.edited = true;
                after_point = true;
            }
            '+' | '-' => {
                info.size += count;
                info.edited = true;
                info.signed = true;
            }
            _ => {
                info.size += count;
                info.edited = true;
            }
        }
    }
    Some(info)
}

/// Layout of one data item and, recursively, of its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLayout {
    pub name: String,
    pub level: u8,
    /// Byte offset of the first occurrence, relative to the start of the
    /// enclosing storage area (working storage or file record buffer).
    pub offset: usize,
    /// Size of one occurrence; also the distance between consecutive
    /// occurrences (the stride).
    pub element_size: usize,
    /// Number of occurrences, or `None` when the item has no `OCCURS`.
    pub occurs: Option<usize>,
    /// Total size including all occurrences.
    pub size: usize,
    pub picture: Option<PictureInfo>,
    /// Initial `VALUE`, if any.
    pub value: Option<ItemValue>,
    pub children: Vec<ItemLayout>,
}

/// A `VALUE` clause literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemValue {
    Numeric(String),
    String(String),
    Figurative(String),
}

impl ItemLayout {
    pub fn is_group(&self) -> bool {
        !self.children.is_empty()
    }

    fn find(&self, name: &str) -> Option<&ItemLayout> {
        if self.name.eq_ignore_ascii_case(name) {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLayout {
    pub name: String,
    /// Records of one file share the same buffer, all at offset 0.
    pub records: Vec<ItemLayout>,
    /// Size of the largest record.
    pub record_size: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageLayout {
    pub working_storage: Vec<ItemLayout>,
    pub working_storage_size: usize,
    pub files: Vec<FileLayout>,
}

impl StorageLayout {
    /// Finds a working-storage item by name (case-insensitive).
    pub fn find(&self, name: &str) -> Option<&ItemLayout> {
        self.working_storage.iter().find_map(|item| item.find(name))
    }

    pub fn file(&self, name: &str) -> Option<&FileLayout> {
        self.files
            .iter()
            .find(|file| file.name.eq_ignore_ascii_case(name))
    }

    /// Resolves a possibly qualified name (`A OF B IN C`) to the chain of
    /// items from the 01-level record down to the item, plus the storage
    /// area it lives in (`None` for working storage, `Some(file)` for an FD record).
    pub fn resolve<'a>(&'a self, name: &str) -> Option<Resolved<'a>> {
        let mut words = name.split_whitespace();
        let target = words.next()?;
        let qualifiers: Vec<&str> = words
            .filter(|word| !word.eq_ignore_ascii_case("OF") && !word.eq_ignore_ascii_case("IN"))
            .collect();
        let areas = std::iter::once((None, self.working_storage.as_slice())).chain(
            self.files
                .iter()
                .map(|file| (Some(file.name.as_str()), file.records.as_slice())),
        );
        for (file, roots) in areas {
            for root in roots {
                let mut chain = Vec::new();
                if let Some(path) = find_path(root, target, &qualifiers, &mut chain) {
                    return Some(Resolved { file, path });
                }
            }
        }
        None
    }
}

/// An item found by [`StorageLayout::resolve`].
#[derive(Debug, Clone)]
pub struct Resolved<'a> {
    pub file: Option<&'a str>,
    /// Ancestors from the 01-level item down to the item itself.
    pub path: Vec<&'a ItemLayout>,
}

impl<'a> Resolved<'a> {
    pub fn item(&self) -> &'a ItemLayout {
        self.path[self.path.len() - 1]
    }

    /// Items on the path that have `OCCURS`, outermost first.
    pub fn tables(&self) -> Vec<&'a ItemLayout> {
        self.path
            .iter()
            .copied()
            .filter(|item| item.occurs.is_some())
            .collect()
    }
}

fn find_path<'a>(
    item: &'a ItemLayout,
    target: &str,
    qualifiers: &[&str],
    chain: &mut Vec<&'a ItemLayout>,
) -> Option<Vec<&'a ItemLayout>> {
    chain.push(item);
    let mut found = None;
    if item.name.eq_ignore_ascii_case(target) && qualifies(chain, qualifiers) {
        found = Some(chain.clone());
    } else {
        for child in &item.children {
            found = find_path(child, target, qualifiers, chain);
            if found.is_some() {
                break;
            }
        }
    }
    chain.pop();
    found
}

/// Qualifiers must appear among the ancestors in order, innermost first.
fn qualifies(chain: &[&ItemLayout], qualifiers: &[&str]) -> bool {
    let mut ancestors = chain[..chain.len() - 1].iter().rev();
    qualifiers.iter().all(|qualifier| {
        ancestors
            .by_ref()
            .any(|ancestor| ancestor.name.eq_ignore_ascii_case(qualifier))
    })
}

pub fn compute_layout(data_division: Option<&DataDivision>) -> StorageLayout {
    let mut layout = StorageLayout::default();
    let Some(data_division) = data_division else {
        return layout;
    };
    if let Some(section) = &data_division.file_section {
        for file in &section.file_descriptions {
            let records: Vec<ItemLayout> = file
                .records
                .iter()
                .map(|record| layout_item(record, 0))
                .collect();
            let record_size = records.iter().map(|record| record.size).max().unwrap_or(0);
            layout.files.push(FileLayout {
                name: file.file_name.to_ascii_uppercase(),
                records,
                record_size,
            });
        }
    }
    if let Some(section) = &data_division.working_storage_section {
        let mut offset = 0;
        for item in &section.data_items {
            let item_layout = layout_item(item, offset);
            offset += item_layout.size;
            layout.working_storage.push(item_layout);
        }
        layout.working_storage_size = offset;
    }
    layout
}

fn layout_item(item: &DataItem, offset: usize) -> ItemLayout {
    let picture = item.picture.as_deref().and_then(picture_info);
    let mut children = Vec::new();
    let mut child_offset = offset;
    for child in &item.children {
        let child_layout = layout_item(child, child_offset);
        child_offset += child_layout.size;
        children.push(child_layout);
    }
    let element_size = if children.is_empty() {
        picture.as_ref().map_or(0, |info| info.size)
    } else {
        child_offset - offset
    };
    let occurs = item
        .occurs
        .as_deref()
        .and_then(|count| count.parse::<usize>().ok());
    let size = element_size * occurs.unwrap_or(1);
    ItemLayout {
        name: item.name.to_ascii_uppercase(),
        level: item.level,
        offset,
        element_size,
        occurs,
        size,
        picture,
        value: item.value.as_ref().map(|literal| match literal {
            Literal::Numeric(text, _) => ItemValue::Numeric(text.clone()),
            Literal::String(text, _) => ItemValue::String(text.clone()),
            Literal::FigurativeConstant(word, _) => {
                ItemValue::Figurative(word.to_ascii_uppercase())
            }
        }),
        children,
    }
}

/// Plain-text dump of the layout, shared by the CLI and the golden tests.
pub fn format_layout(layout: &StorageLayout) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "WORKING-STORAGE ({} bytes)\n",
        layout.working_storage_size
    ));
    for item in &layout.working_storage {
        format_item(&mut output, item, 1);
    }
    for file in &layout.files {
        output.push_str(&format!(
            "FILE {} (record {} bytes)\n",
            file.name, file.record_size
        ));
        for record in &file.records {
            format_item(&mut output, record, 1);
        }
    }
    output
}

fn format_item(output: &mut String, item: &ItemLayout, depth: usize) {
    let indent = "  ".repeat(depth);
    output.push_str(&format!(
        "{indent}{} @{} size={}",
        item.name, item.offset, item.size
    ));
    if let Some(count) = item.occurs {
        output.push_str(&format!(" occurs={count} stride={}", item.element_size));
    }
    output.push('\n');
    for child in &item.children {
        format_item(output, child, depth + 1);
    }
}

#[cfg(test)]
mod tests;
