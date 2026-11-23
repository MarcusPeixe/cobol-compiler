use std::path::{Path, PathBuf};

use cobol_compiler::diagnostics::Diagnostic;
use cobol_compiler::diagnostics::Severity;
use cobol_compiler::lexer::lex;
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::source::{ReaderOptions, SourceMap};

#[test]
fn test_prog001_divisoes() {
    test_lexer_golden("prog001_divisoes.cob");
}

#[test]
fn test_prog002_operacoes() {
    test_lexer_golden("prog002_operacoes.cob");
}

#[test]
fn test_prog003_calculadora() {
    test_lexer_golden("prog003_calculadora.cob");
}

#[test]
fn test_prog004_folha_pgto() {
    test_lexer_golden("prog004_folha_pgto.cob");
}

#[test]
fn test_prog005_strings() {
    test_lexer_golden("prog005_strings.cob");
}

#[test]
fn test_prog006_files() {
    test_lexer_golden("prog006_files.cob");
}

#[test]
fn test_prog007_loja() {
    test_lexer_golden("prog007_loja.cob");
}

fn test_lexer_golden(file_name: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let relative_path = Path::new("cobol-examples").join(file_name);
    let source_path = root.join(&relative_path);

    let actual = lex_example(&root, &relative_path, &source_path).expect("lexing should succeed");

    insta::assert_snapshot!(file_name, actual);
}

fn lex_example(root: &Path, relative_path: &Path, source_path: &Path) -> Result<String, String> {
    let source = std::fs::read_to_string(source_path)
        .map_err(|error| format!("cannot read source: {error}"))?;
    let mut source_map = SourceMap::default();
    let root_file = source_map.add_file(relative_path.display().to_string(), source);
    let loader = FsLoader {
        search_dirs: vec![root.join(relative_path.parent().unwrap_or(Path::new(".")))],
    };

    let (logical_text, mut diagnostics) = preprocess(
        &mut source_map,
        root_file,
        ReaderOptions::default(),
        &loader,
    );
    let (tokens, lexer_diagnostics) = lex(&logical_text);
    diagnostics.extend(lexer_diagnostics);

    let errors: Vec<&Diagnostic> = diagnostics.iter().filter(|d| d.is_error()).collect();
    if !errors.is_empty() {
        return Err(format!(
            "lexing failed: {}",
            errors
                .iter()
                .map(|diagnostic| format_diagnostic(diagnostic, &source_map))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    let mut output = String::new();
    for token in tokens {
        let file = source_map.file(token.span.file);
        let (line, column) = source_map.line_col(token.span.file, token.span.start);
        output.push_str(&format!(
            "{}:{line:03}:{column:03}  {}\n",
            file.name, token.kind
        ));
    }
    output.push_str("\nDIAGNOSTICS:\n");
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
    {
        output.push_str(&format_diagnostic(diagnostic, &source_map));
        output.push('\n');
    }
    Ok(output)
}

fn format_diagnostic(diagnostic: &Diagnostic, source_map: &SourceMap) -> String {
    let location = diagnostic.span.map_or_else(
        || "<unknown>".to_string(),
        |span| {
            let file = &source_map.file(span.file).name;
            let (line, column) = source_map.line_col(span.file, span.start);
            format!("{file}:{line:03}:{column:03}")
        },
    );
    let severity = match diagnostic.severity {
        Severity::Info => "INFO",
        Severity::Warning => "WARNING",
        Severity::Error => "ERROR",
        Severity::Severe => "SEVERE",
    };
    format!(
        "{severity} {} {location} {}",
        diagnostic.code, diagnostic.message
    )
}
