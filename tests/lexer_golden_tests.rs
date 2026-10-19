use std::path::{Path, PathBuf};

use cobol_compiler::diagnostics::Diagnostic;
use cobol_compiler::diagnostics::Severity;
use cobol_compiler::lexer::lex;
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::source::{ReaderOptions, SourceMap};

const EXAMPLE_FILES: [&str; 7] = [
    "prog001_divisoes.cob",
    "prog002_operacoes.cob",
    "prog003_calculadora.cob",
    "prog004_folha_pgto.cob",
    "prog005_strings.cob",
    "prog006_files.cob",
    "prog007_loja.cob",
];

#[test]
fn professor_examples_match_lexer_goldens() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut mismatches = Vec::new();

    for file_name in EXAMPLE_FILES {
        let relative_path = Path::new("cobol-examples").join(file_name);
        let source_path = root.join(&relative_path);
        let result = lex_example(&root, &relative_path, &source_path);

        match result {
            Ok(actual) => {
                let golden_path = root
                    .join("tests/lexer-goldens")
                    .join(format!("{file_name}.tokens"));
                match std::fs::read_to_string(&golden_path) {
                    Ok(expected) if expected == actual => {}
                    Ok(_) => mismatches.push(file_name.to_string()),
                    Err(error) => {
                        mismatches.push(format!("{file_name} (cannot read golden: {error})"))
                    }
                }
            }
            Err(error) => mismatches.push(format!("{file_name} ({error})")),
        }
    }

    assert!(
        mismatches.is_empty(),
        "lexer output differed for:\n{}",
        mismatches.join("\n")
    );
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
