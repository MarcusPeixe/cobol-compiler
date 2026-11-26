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
fn test_prog004_folha_pgto() {
    test_codegen_golden("prog004_folha_pgto.cob");
}

#[test]
fn test_search() {
    test_codegen_golden("codegen_test_search.cob");
}

#[test]
fn test_inspect() {
    test_codegen_golden("codegen_test_inspect.cob");
}

#[test]
fn test_inspect_multiple() {
    test_codegen_golden("codegen_test_inspect_multiple.cob");
}

#[test]
fn test_inspect_variable() {
    test_codegen_golden("codegen_test_inspect_variable.cob");
}

#[test]
fn test_prog005_strings() {
    test_codegen_golden("prog005_strings.cob");
}

#[test]
fn test_prog006_files() {
    test_codegen_golden("prog006_files.cob");
}

#[test]
fn test_file_status() {
    test_codegen_golden("codegen_test_file_status.cob");
}

#[test]
fn test_file_read() {
    test_codegen_golden("codegen_test_file_read.cob");
}

#[test]
fn test_write_advancing() {
    test_codegen_golden("codegen_test_write_advancing.cob");
}

#[test]
fn test_initialize() {
    test_codegen_golden("codegen_test_initialize.cob");
}

#[test]
fn test_divide_into() {
    test_codegen_golden("codegen_test_divide_into.cob");
}

#[test]
fn test_indexed_file() {
    test_codegen_golden("codegen_test_indexed_file.cob");
}

#[test]
fn test_signed_indexed_key() {
    test_codegen_golden("codegen_test_signed_indexed_key.cob");
}

#[test]
fn test_wide_move() {
    test_codegen_golden("codegen_test_wide_move.cob");
}

#[test]
fn test_wide_add_subtract() {
    test_codegen_golden("codegen_test_wide_add_subtract.cob");
}

#[test]
fn test_wide_expression() {
    test_codegen_golden("codegen_test_wide_expression.cob");
}

#[test]
fn test_wide_editing() {
    test_codegen_golden("codegen_test_wide_editing.cob");
}

#[test]
fn test_prog007_loja() {
    test_codegen_golden("prog007_loja.cob");
}

#[test]
fn test_search_all() {
    test_codegen_golden("codegen_test_search_all.cob");
}

#[test]
fn test_computed_move() {
    test_codegen_golden("codegen_test_computed_move.cob");
}

#[test]
fn test_duplicate_alternate_key() {
    test_codegen_golden("codegen_test_duplicate_alternate_key.cob");
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

#[test]
fn run_prog004_folha_pgto() {
    let Some(output) = run_in_mars("prog004_folha_pgto.cob", "") else {
        return;
    };
    assert!(output.contains("SALARIO(IDX)=22222.22\nDPTO(IDX,1)=A2\n"));
    assert!(output.contains("'WHEN NOME(TID) = MARIA'\nTID=2\n"));
    assert!(output.ends_with(&format!(
        "JOSEA1111111A1A4{}FABIO3333333A2A3\n",
        "\0".repeat(16)
    )));
}

#[test]
fn run_search() {
    let Some(output) = run_in_mars("codegen_test_search.cob", "") else {
        return;
    };
    assert_eq!(output, "FOUND 2 1\nEND 4\n");
}

#[test]
fn run_inspect() {
    let Some(output) = run_in_mars("codegen_test_inspect.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        "LEADING=05\nAFTER-MISSING=00\nFIRST=XYABAA\nREPLACE-LEADING=ZYABAA\nCONVERT=231B11\n"
    );
}

#[test]
fn run_inspect_multiple() {
    let Some(output) = run_in_mars("codegen_test_inspect_multiple.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        "PRIORITY=XYZQAA\nONCE=BCCBBB\nREGION=QAABZZ\nSNAPSHOT=123456\n"
    );
}

#[test]
fn run_inspect_variable() {
    let Some(output) = run_in_mars("codegen_test_inspect_variable.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        "TALLY=2\nAFTER=4\nMULTIPLE=XYXYZA\nSINGLE=XBXBBX\nCONVERT=XYXYYX\n"
    );
}

#[test]
fn run_inspect_invalid() {
    for input in [
        "1\n0\n1\n",
        "1\n5\n1\n",
        "0\n1\n1\n",
        "4\n2\n2\n",
        "1\n2\n1\n",
    ] {
        let Some(output) = run_in_mars("codegen_test_inspect_invalid.cob", input) else {
            return;
        };
        assert_eq!(
            output,
            "Invalid INSPECT operand length or reference modification\n"
        );
    }
}

#[test]
fn run_wide_expression() {
    let Some(output) = run_in_mars("codegen_test_wide_expression.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        "PRODUCT=-001851851827.68\nEXACT=+000000000001.00\n\
         FRACTIONS=+000000000001.00\nSIGNS=+000000000001.00\n\
         QUOTIENT=+000493827156.04\nNEGATIVE-DIVISOR=+000617283944.56\n\
         ALIASED=+001234567890.00\nDivision by zero in wide expression\n"
    );
}

#[test]
fn run_wide_editing() {
    let Some(output) = run_in_mars("codegen_test_wide_editing.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        format!(
            "WIDE=[-{}12345678901234567890.00]!\nNARROW=[{}42.00]!\nLARGE-NARROW=[{}42]!\n",
            " ".repeat(13),
            " ".repeat(32),
            " ".repeat(36)
        )
    );
}

#[test]
fn wide_expression_precision_limit() {
    let (_, allowed) = compile_with_diagnostics("codegen_test_wide_precision_limit.cob");
    assert!(!allowed.iter().any(|d| d.is_error()), "{allowed:?}");
    let (_, exceeded) = compile_with_diagnostics("codegen_test_wide_precision_exceeded.cob");
    assert!(
        exceeded.iter().any(|d| {
            d.code == "cobol::codegen::unsupported"
                && d.message
                    .contains("wide expression exceeds 512 digit precision")
        }),
        "{exceeded:?}"
    );
}

#[test]
fn run_prog005_strings() {
    let Some(output) = run_in_mars("prog005_strings.cob", "") else {
        return;
    };
    assert!(output.contains("CONT=03 - FOR ALL 'L'\n"));
    assert!(output.contains("CONT=09 - AFTER INITIAL ' '\n"));
    assert!(output.contains("SAUX=' GOOD-DAY      ' - HOLA-DAY TO GOOD-DAY'\n"));
    assert!(output.contains("SAUX='hello world    ' - CONVERTING'\n"));
    assert!(output.ends_with("SAUX='FFFFFFFFFFFFFFF' - REPLACING \n"));
}

#[test]
fn run_prog006_files() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-prog006-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("prog006_files.cob", "", Some(&dir));
    let file = dir.join("DADOS.TXT");
    let bytes = if output.is_some() {
        Some(std::fs::read(&file).expect("cannot read output file"))
    } else {
        None
    };
    if bytes.is_some() {
        std::fs::remove_file(&file).expect("cannot remove output file");
    }
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert!(output.contains("TRABALHADOR(IDX)=MARIA2222222A2  \n"));
    assert_eq!(
        bytes.expect("Mars produced output"),
        b"00001JOSEA     000001111111A1A4\
          00002MARIA     000002222222A2  \
          00003FABIO     000003333333A2A3"
    );
}

#[test]
fn run_file_status() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-file-status-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("codegen_test_file_status.cob", "", Some(&dir));
    let bytes = if output.is_some() {
        let path = dir.join("STATUS.TXT");
        let bytes = std::fs::read(&path).expect("cannot read output file");
        std::fs::remove_file(&path).expect("cannot remove output file");
        Some(bytes)
    } else {
        None
    };
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert_eq!(
        output,
        "OPEN=00\nOPEN-AGAIN=41\nWRITE=00\nCLOSE=00\nCLOSE-AGAIN=42\nMISSING=30\n"
    );
    assert_eq!(bytes.as_deref(), Some(&b"ABC"[..]));
}

#[test]
fn run_file_read() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-file-read-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("codegen_test_file_read.cob", "", Some(&dir));
    if output.is_some() {
        std::fs::remove_file(dir.join("READ.TXT")).expect("cannot remove output file");
    }
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert_eq!(output, "BAD-WRITE=48\nREAD=ABC\nSTATUS=00\nEND=10\n");
}

#[test]
fn run_initialize() {
    let Some(output) = run_in_mars("codegen_test_initialize.cob", "") else {
        return;
    };
    assert_eq!(output, "ALL=   00Q   00Q\nONE=   00Q   00X\nEDIT=   0\n");
}

#[test]
fn run_divide_into() {
    let Some(output) = run_in_mars("codegen_test_divide_into.cob", "") else {
        return;
    };
    assert_eq!(output, "RESULT=01,04\nSIZE ERROR\nAFTER=04\nGIVING=04\n");
}

#[test]
fn run_indexed_file() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-indexed-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("codegen_test_indexed_file.cob", "", Some(&dir));
    let bytes = if output.is_some() {
        let path = dir.join("INDEX.DAT");
        let bytes = std::fs::read(&path).expect("cannot read indexed file");
        std::fs::remove_file(&path).expect("cannot remove indexed file");
        Some(bytes)
    } else {
        None
    };
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert_eq!(
        output,
        "DUP-ID=22\nDUP-NAME=22\nFOUND=02DEF\nSTART=00\n\
         NEXT=02DEF\nNEXT=03GHI\nEND=10\nLESS=02DEF\n\
         MISSING=23\nALTERNATE=01ABC\n"
    );
    assert_eq!(bytes.as_deref(), Some(&b"01ABC02DEF03GHI"[..]));
}

#[test]
fn run_signed_indexed_key() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-signed-indexed-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("codegen_test_signed_indexed_key.cob", "", Some(&dir));
    if output.is_some() {
        std::fs::remove_file(dir.join("SIGNED.DAT")).expect("cannot remove indexed file");
    }
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert_eq!(
        output,
        "ALT-DUP=02\nZERO-DUP=22\nORDER=-10\nORDER=-02\nORDER=+00\n\
         ORDER=+01\nEND=10\nGREATER=+00\nLESS-EQUAL=-02\n\
         ALT=-02:02\nALT-NEXT=+01:00\n"
    );
}

#[test]
fn run_write_advancing() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-advancing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("codegen_test_write_advancing.cob", "", Some(&dir));
    let bytes = if output.is_some() {
        let path = dir.join("ADVANCE.DAT");
        let bytes = std::fs::read(&path).expect("cannot read output file");
        std::fs::remove_file(&path).expect("cannot remove output file");
        Some(bytes)
    } else {
        None
    };
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert_eq!(output, "STATUS=00\nINVALID=30\nINVALID-BEFORE=30\n");
    assert_eq!(bytes.as_deref(), Some(&b"\n\nONE1TWO2\x0c\nEND3ZERO"[..]));
}

#[test]
fn run_duplicate_alternate_key() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-duplicate-key-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let output = run_in_mars_in_dir("codegen_test_duplicate_alternate_key.cob", "", Some(&dir));
    let bytes = if output.is_some() {
        let path = dir.join("INDEX.DAT");
        let bytes = std::fs::read(&path).expect("cannot read indexed file");
        std::fs::remove_file(&path).expect("cannot remove indexed file");
        Some(bytes)
    } else {
        None
    };
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    assert_eq!(
        output,
        "FIRST-WRITE=00\nDUP-WRITE=02\nTHIRD-WRITE=02\nDUP-PRIMARY=22\n\
         KEY=03ABC:02\nNEXT=01ABC:02\nNEXT=04ABC:00\nNEXT=02DEF:00\n\
         END=10\nSTART=03ABC:02\nREOPEN-WRITE=02\n"
    );
    assert_eq!(bytes.as_deref(), Some(&b"03ABC01ABC02DEF04ABC05ABC"[..]));
}

#[test]
fn run_wide_move() {
    let Some(output) = run_in_mars(
        "codegen_test_wide_move.cob",
        "9876543210\n12345678.91\n-8765432109\n",
    ) else {
        return;
    };
    assert_eq!(
        output,
        "LARGE=1234567890\nDECIMAL=34567890.00\n\
         TRUNCATED=0034567890\nSIGNED=-001234567890\n\
         EDITED-NEGATIVE=-1234567890.00\nEDITED-LARGE= 9876543210.00\n\
         UNSIGNED=1234567890\n\
         LARGE-EQUAL\nNEGATIVE-LESS\nDECIMAL-EQUAL\nZERO-EQUAL\n\
         INCREMENT=2147483648\nOVERFLOW\nPRESERVED=9999999999\n\
         INPUT=9876543210,12345678.91,-8765432109\n"
    );
}

#[test]
fn run_wide_add_subtract() {
    let Some(output) = run_in_mars("codegen_test_wide_add_subtract.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        "ADD=+100000000000.01\nSUBTRACT=-000000000000.99\n\
         NEGATIVE=-000000000001.01\nCOMPUTE=+123456789011.31\n\
         ZERO=+000000000000.00\nPRODUCT=-003086419725.30\n\
         QUOTIENT=-000493827156.04\nREMAINDER=-000000000000.02\nDIVIDE-ZERO\n\
         QUOTIENT-PRESERVED=-000493827156.04\n\
         QUOTIENT-SMALL-DIVISOR=+000000010000.00\n\
         INTO=+000000000001.00,+000000000004.00\n\
         PRODUCT-NARROW=+009999800001.00\nPRODUCT-OVERFLOW\n\
         PRODUCT-PRESERVED=+009999800001.00\n\
         OVERFLOW\nPRESERVED=+999999999999.99\n\
         SMALL-OVERFLOW\nSMALL=55\n"
    );
}

#[test]
fn run_prog007_loja() {
    let dir = std::env::temp_dir().join(format!(
        "cobol-codegen-prog007-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&dir).expect("cannot create test directory");
    let key = dir.join("PROG10-001.KEY");
    let data = dir.join("PROG10-001.DAT");
    std::fs::write(&key, b"2147483647").expect("cannot create key file");
    let existing = b"2147483647OLD       000000000100000000000100OLD                 ";
    std::fs::write(&data, existing).expect("cannot create data file");
    let input = "1\nPEN\n12.34\n2\nA PENCIL\n4\n2147483648\n5\n";
    let output = run_in_mars_in_dir("prog007_loja.cob", input, Some(&dir));
    let first_key_bytes = std::fs::read(&key).expect("cannot read key file");
    let second_input = "1\nPAPER\n1.25\n3\nA PAGE\n4\n2147483649\n5\n";
    let second_output = run_in_mars_in_dir("prog007_loja.cob", second_input, Some(&dir));
    let bytes = std::fs::read(&data).expect("cannot read data file");
    let key_bytes = std::fs::read(&key).expect("cannot read key file");
    std::fs::remove_file(&data).expect("cannot remove data file");
    std::fs::remove_file(&key).expect("cannot remove key file");
    std::fs::remove_dir(&dir).expect("cannot remove test directory");
    let Some(output) = output else { return };
    let Some(second_output) = second_output else {
        return;
    };
    assert!(output.contains("PRODUTO ARMAZENADO"), "{output}");
    assert!(output.contains("WS-ULTIMO-PID --> 2147483648"), "{output}");
    assert!(output.contains("NOME: PEN"), "{output}");
    assert!(output.contains("VALOR: 0000000012.34"), "{output}");
    assert!(output.contains("DESCRICAO: A PENCIL"), "{output}");
    assert!(!output.contains("ERRO AO REGISTRO"), "{output}");
    assert!(output.ends_with("BYE\n"), "{output}");
    assert!(second_output.contains("WS-ULTIMO-PID --> 2147483649"));
    assert!(second_output.contains("NOME: PAPER"));
    assert!(second_output.contains("PRODUTO ARMAZENADO"));
    assert!(!second_output.contains("ERRO AO REGISTRO"));
    assert_eq!(
        bytes,
        [
            &existing[..],
            &b"2147483648PEN       000000001234000000000200A PENCIL            "[..],
            &b"2147483649PAPER     000000000125000000000300A PAGE              "[..]
        ]
        .concat()
    );
    assert_eq!(first_key_bytes, b"2147483648");
    assert_eq!(key_bytes, b"2147483649");
}

#[test]
fn run_search_all() {
    let Some(output) = run_in_mars("codegen_test_search_all.cob", "") else {
        return;
    };
    assert_eq!(output, "FOUND=2\nMISSING\n");
}

#[test]
fn run_computed_move() {
    let Some(output) = run_in_mars("codegen_test_computed_move.cob", "") else {
        return;
    };
    assert_eq!(
        output,
        "ADD=7           \nDIVIDE=3.5000      \nNEGATIVE=-3          \n"
    );
}

fn compile(file_name: &str) -> String {
    let (assembly, diagnostics) = compile_with_diagnostics(file_name);
    assert!(
        !diagnostics.iter().any(|d| d.is_error()),
        "unexpected errors: {diagnostics:?}"
    );
    assembly
}

fn compile_with_diagnostics(
    file_name: &str,
) -> (String, Vec<cobol_compiler::diagnostics::Diagnostic>) {
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
    (assembly, diagnostics)
}

fn test_codegen_golden(file_name: &str) {
    insta::assert_snapshot!(file_name, compile(file_name));
}

/// Runs the compiled program under `Mars nc`. Returns `None` (skipping the
/// test) when Mars is not installed.
fn run_in_mars(file_name: &str, input: &str) -> Option<String> {
    run_in_mars_in_dir(file_name, input, None)
}

fn run_in_mars_in_dir(
    file_name: &str,
    input: &str,
    directory: Option<&std::path::Path>,
) -> Option<String> {
    let assembly = compile(file_name);
    let path = std::env::temp_dir().join(format!(
        "cobol-compiler-{}-{}.asm",
        std::process::id(),
        file_name
    ));
    std::fs::write(&path, assembly).expect("cannot write assembly");
    let mut command = Command::new("Mars");
    command
        .arg("nc")
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let child = command.spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::remove_file(&path).expect("cannot remove assembly");
            eprintln!("skipping: Mars not found on PATH");
            return None;
        }
        Err(error) => panic!("cannot start Mars: {error}"),
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
