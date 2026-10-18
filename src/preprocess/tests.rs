use super::*;
use std::collections::HashMap;

struct MemLoader(HashMap<&'static str, &'static str>);

impl CopybookLoader for MemLoader {
    fn load(&self, name: &str) -> Result<Copybook, LoadError> {
        let key = name.to_uppercase();
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&key))
            .map(|(k, v)| Copybook {
                id: k.to_string(),
                name: format!("{k}.cpy"),
                text: v.to_string(),
            })
            .ok_or_else(|| LoadError::NotFound(name.to_string()))
    }
}

fn run(
    src: &str,
    books: &[(&'static str, &'static str)],
) -> (String, Vec<Diagnostic>, SourceMap, LogicalText) {
    let mut map = SourceMap::default();
    let root = map.add_file("main.cob", src.to_string());
    let loader = MemLoader(books.iter().copied().collect());
    let (text, diags) = preprocess(&mut map, root, ReaderOptions::default(), &loader);
    (text.text.clone(), diags, map, text)
}

fn words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

#[test]
fn no_copy_is_identity() {
    let (t, d, ..) = run("       STOP RUN.\n", &[]);
    assert!(d.is_empty());
    assert_eq!(words(&t), ["STOP", "RUN."]);
}

#[test]
fn simple_copy_is_expanded() {
    let (t, d, ..) = run(
        "       DATA DIVISION.\n       COPY REC.\n       PROCEDURE DIVISION.\n",
        &[("REC", "       01 A PIC X.\n")],
    );
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(
        words(&t),
        [
            "DATA",
            "DIVISION.",
            "01",
            "A",
            "PIC",
            "X.",
            "PROCEDURE",
            "DIVISION."
        ]
    );
}

#[test]
fn copy_is_case_insensitive_and_accepts_quoted_names_and_library() {
    for stmt in [
        "copy rec.",
        "COPY 'REC'.",
        "COPY \"rec\".",
        "COPY REC OF LIB.",
        "COPY REC IN LIB .",
    ] {
        let (t, d, ..) = run(&format!("       {stmt}\n"), &[("REC", "       01 A.\n")]);
        assert!(d.is_empty(), "{stmt}: {d:?}");
        assert_eq!(words(&t), ["01", "A."], "{stmt}");
    }
}

#[test]
fn copy_spanning_lines() {
    let (t, d, ..) = run(
        "       COPY\n          REC\n          .\n",
        &[("REC", "       01 A.\n")],
    );
    assert!(d.is_empty());
    assert_eq!(words(&t), ["01", "A."]);
}

#[test]
fn nested_copy() {
    let (t, d, ..) = run(
        "       COPY A.\n",
        &[
            ("A", "       01 X.\n       COPY B.\n"),
            ("B", "       01 Y.\n"),
        ],
    );
    assert!(d.is_empty(), "{d:?}");
    assert_eq!(words(&t), ["01", "X.", "01", "Y."]);
}

#[test]
fn same_copybook_twice_is_allowed() {
    let (t, d, ..) = run(
        "       COPY A.\n       COPY A.\n",
        &[("A", "       01 X.\n")],
    );
    assert!(d.is_empty());
    assert_eq!(words(&t), ["01", "X.", "01", "X."]);
}

#[test]
fn copy_inside_literal_and_as_part_of_word_is_ignored() {
    let (t, d, ..) = run(
        "       DISPLAY 'COPY REC.' COPYRIGHT-X.\n",
        &[("REC", "       01 A.\n")],
    );
    assert!(d.is_empty());
    assert_eq!(words(&t), ["DISPLAY", "'COPY", "REC.'", "COPYRIGHT-X."]);
}

#[test]
fn copy_in_comment_is_ignored() {
    let (t, d, ..) = run(
        "      * COPY REC.\n       STOP RUN.\n",
        &[("REC", "       01 A.\n")],
    );
    assert!(d.is_empty());
    assert_eq!(words(&t), ["STOP", "RUN."]);
}

#[test]
fn missing_copybook() {
    let (_, d, ..) = run("       COPY NOPE.\n", &[]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::preprocess::copybook_error");
    assert!(d[0].message.contains("NOPE"));
    assert!(d[0].help.is_some());
}

#[test]
fn recursive_copy_is_detected() {
    let (_, d, ..) = run(
        "       COPY A.\n",
        &[("A", "       COPY B.\n"), ("B", "       COPY A.\n")],
    );
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::preprocess::recursive_copy");
}

#[test]
fn self_recursive_copy_is_detected() {
    let (_, d, ..) = run("       COPY A.\n", &[("A", "       COPY A.\n")]);
    assert_eq!(
        d.iter()
            .filter(|d| d.code == "cobol::preprocess::recursive_copy")
            .count(),
        1
    );
}

#[test]
fn missing_name() {
    let (_, d, ..) = run("       COPY.\n", &[]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::preprocess::missing_name");
}

#[test]
fn missing_period() {
    let (t, d, ..) = run(
        "       COPY REC\n       STOP RUN.\n",
        &[("REC", "       01 A.\n")],
    );
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::preprocess::missing_period");
    assert!(t.contains("STOP"));
}

#[test]
fn replacing_is_unsupported_and_skipped() {
    let (t, d, ..) = run(
        "       COPY REC REPLACING ==A== BY ==B==.\n       STOP RUN.\n",
        &[("REC", "       01 A.\n")],
    );
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::preprocess::replacing_unsupported");
    assert_eq!(words(&t), ["STOP", "RUN."]);
}

#[test]
fn spans_of_copied_text_point_into_the_copybook() {
    let (_, d, map, text) = run(
        "       COPY REC.\n",
        &[("REC", "000100 01 FIELD-A PIC X.\n")],
    );
    assert!(d.is_empty());
    let i = text.text.find("FIELD-A").unwrap();
    let span = text.span(i, i + 7);
    assert_eq!(map.file(span.file).name, "REC.cpy");
    assert_eq!(&map.file(span.file).text[span.start..span.end], "FIELD-A");
}

#[test]
fn errors_inside_copybooks_are_reported() {
    let (_, d, map, _) = run("       COPY REC.\n", &[("REC", "      X BAD\n")]);
    assert_eq!(d.len(), 1);
    assert_eq!(map.file(d[0].span.unwrap().file).name, "REC.cpy");
}

#[test]
fn fs_loader_rejects_path_traversal() {
    let loader = FsLoader {
        search_dirs: vec![PathBuf::from(".")],
    };
    for name in ["../x", "a/b", "", "a\\b"] {
        assert!(
            matches!(loader.load(name), Err(LoadError::InvalidName(_))),
            "{name}"
        );
    }
}

#[test]
fn fs_loader_finds_files_with_extensions_and_case() {
    let dir = std::env::temp_dir().join(format!("cobol-fs-loader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("rec.cpy"), "       01 A.\n").unwrap();
    let loader = FsLoader {
        search_dirs: vec![dir.clone()],
    };
    let book = loader.load("REC").unwrap();
    assert_eq!(book.text, "       01 A.\n");
    assert!(matches!(loader.load("OTHER"), Err(LoadError::NotFound(_))));
    std::fs::remove_dir_all(&dir).unwrap();
}
