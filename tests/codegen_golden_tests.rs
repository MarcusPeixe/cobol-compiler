use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use cobol_compiler::codegen::generate;
use cobol_compiler::lexer::lex;
use cobol_compiler::parser::parse;
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::semantic::analyze;
use cobol_compiler::source::{ReaderOptions, SourceMap};

#[test]
fn test_control_flow() {
    test_codegen_golden("codegen_test_control_flow.cob");
}

#[test]
fn test_prog001_divisoes() {
    test_codegen_golden("prog001_divisoes.cob");
}

#[test]
fn test_prog002_operacoes() {
    test_codegen_golden("prog002_operacoes.cob");
}

#[test]
fn test_prog003_calculadora() {
    test_codegen_golden("prog003_calculadora.cob");
}

#[test]
fn run_control_flow() {
    let output = run_in_mars("codegen_test_control_flow.cob", "");
    assert_eq!(
        output.as_deref(),
        Some("TOTAL=0015\nSHOW COBOL   \nSHOW COBOL   \nTOTAL=0039\nNEG=-005\nTWO\nCOB\nBIG\n")
    );
}

#[test]
fn run_prog001_divisoes() {
    let output = run_in_mars("prog001_divisoes.cob", "");
    assert_eq!(output.as_deref(), Some("Hello world\nEnio Basso - 2\n"));
}

#[test]
fn run_prog002_operacoes() {
    let Some(output) = run_in_mars("prog002_operacoes.cob", "") else {
        return;
    };
    assert!(output.contains("DIVIDE A BY 2 GIVING B REMAINDER C .-->\nA= 81 , B= 40 , C= 01\n"));
    assert!(output.ends_with("ON SIZE ERROR -->99\n"));
}

#[test]
fn run_prog003_calculadora() {
    let input = "1\ns\n10\n3.5\n1\np\n2\n3\n1\nd\n7\n2\n1\nu\n3\n10\n2\n";
    let Some(output) = run_in_mars("prog003_calculadora.cob", input) else {
        return;
    };
    let results: Vec<&str> = output
        .lines()
        .filter(|line| line.starts_with("Resultado:"))
        .collect();
    assert_eq!(
        results,
        [
            "Resultado:          13.50",
            "Resultado:           8.00",
            "Resultado:           3.50",
            "Resultado:           7.00"
        ]
    );
}

fn compile(file_name: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(root.join("cobol-examples").join(file_name))
        .expect("cannot read source");
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("test.cob", source);
    let loader = FsLoader {
        search_dirs: Vec::new(),
    };
    let (logical_text, mut diagnostics) =
        preprocess(&mut source_map, file_id, ReaderOptions::default(), &loader);
    let (tokens, lexer_diagnostics) = lex(&logical_text);
    diagnostics.extend(lexer_diagnostics);
    let (program, parser_diagnostics) = parse(tokens);
    diagnostics.extend(parser_diagnostics);
    let (analyzed, semantic_diagnostics) = analyze(&program);
    diagnostics.extend(semantic_diagnostics);
    let (assembly, codegen_diagnostics) = generate(&analyzed);
    diagnostics.extend(codegen_diagnostics);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {diagnostics:?}"
    );
    assembly
}

fn test_codegen_golden(file_name: &str) {
    insta::assert_snapshot!(file_name, compile(file_name));
}

/// Runs the compiled program under `Mars nc`. Returns `None` (skipping the
/// test) when Mars is not installed.
fn run_in_mars(file_name: &str, input: &str) -> Option<String> {
    let assembly = compile(file_name);
    let path = std::env::temp_dir().join(format!(
        "cobol-compiler-{}-{}.asm",
        std::process::id(),
        file_name
    ));
    std::fs::write(&path, assembly).expect("cannot write assembly");
    let child = Command::new("Mars")
        .arg("nc")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(mut child) = child else {
        let _ = std::fs::remove_file(&path);
        eprintln!("skipping: Mars not found on PATH");
        return None;
    };
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("cannot write stdin");
    let result = child.wait_with_output().expect("Mars did not finish");
    let _ = std::fs::remove_file(&path);
    assert!(
        result.status.success(),
        "Mars failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    // Mars prints one extra newline when the program terminates.
    let stdout = String::from_utf8_lossy(&result.stdout);
    Some(stdout.strip_suffix('\n').unwrap_or(&stdout).to_string())
}
