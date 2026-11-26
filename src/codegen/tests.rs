use crate::diagnostics::Diagnostic;
use crate::lexer::lex;
use crate::parser::parse;
use crate::preprocess::{FsLoader, preprocess};
use crate::semantic::{AnalyzedProgram, analyze};
use crate::source::{ReaderOptions, SourceMap};

use super::{c_string, generate};

fn analyzed_fixture(name: &str) -> AnalyzedProgram {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("cobol-examples")
        .join(name);
    let text = std::fs::read_to_string(path).expect("fixture should be readable");
    let mut source_map = SourceMap::default();
    let file = source_map.add_file(name, text);
    let loader = FsLoader {
        search_dirs: Vec::new(),
    };
    let (logical, mut diagnostics) =
        preprocess(&mut source_map, file, ReaderOptions::default(), &loader);
    let (tokens, lex_diagnostics) = lex(&logical);
    diagnostics.extend(lex_diagnostics);
    let (program, parse_diagnostics) = parse(tokens);
    diagnostics.extend(parse_diagnostics);
    let (analyzed, semantic_diagnostics) = analyze(&program);
    diagnostics.extend(semantic_diagnostics);
    assert!(
        !diagnostics.iter().any(Diagnostic::is_error),
        "{diagnostics:?}"
    );
    analyzed
}

#[test]
fn escapes_c_strings_without_changing_their_bytes() {
    assert_eq!(c_string("a\"\\\0é9"), "\"a\\\"\\\\\\000\\303\\2519\"");
    assert_eq!(c_string("??/??="), "\"\\?\\?/\\?\\?=\"");
}

#[test]
fn rejects_unsupported_statements_with_a_source_span() {
    let program = analyzed_fixture("codegen_test_unsupported.cob");
    let diagnostics = generate(&program).expect_err("INITIALIZE is not supported");
    assert!(diagnostics.iter().any(
        |diagnostic| diagnostic.code == "cobol::codegen::unsupported" && diagnostic.span.is_some()
    ));
}
