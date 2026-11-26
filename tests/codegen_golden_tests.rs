use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        loop {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("cobol-codegen-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create test directory {}: {error}", path.display()),
            }
        }
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("cannot remove test directory {}: {error}", self.0.display());
        }
    }
}

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cobol-examples")
}

fn compiler() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cobol-compiler"))
}

fn generated_c(name: &str) -> String {
    let output = compiler()
        .arg("codegen")
        .arg(examples_dir().join(name))
        .output()
        .expect("cannot run COBOL compiler");
    assert_success(&output);
    String::from_utf8(output.stdout).expect("generated C should be UTF-8")
}

fn compile_c(directory: &TestDirectory, source: &str) -> PathBuf {
    let source_path = directory.path().join("program.c");
    let executable = directory.path().join("program");
    fs::write(&source_path, source).expect("cannot write generated C");
    let output = Command::new("cc")
        .args([
            "-std=c11",
            "-pedantic-errors",
            "-Wall",
            "-Wextra",
            "-Werror",
        ])
        .arg(&source_path)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("C compiler `cc` is required for codegen tests");
    assert_success(&output);
    executable
}

fn run(executable: &Path, directory: &TestDirectory, input: &[u8]) -> Output {
    let mut child = Command::new(executable)
        .current_dir(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cannot run generated program");
    child
        .stdin
        .take()
        .expect("missing program stdin")
        .write_all(input)
        .expect("cannot send program input");
    child.wait_with_output().expect("cannot wait for program")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        escaped(&output.stdout),
        escaped(&output.stderr)
    );
}

fn escaped(bytes: &[u8]) -> String {
    let mut result = String::new();
    for &byte in bytes {
        match byte {
            b'\n' => result.push('\n'),
            b'\\' => result.push_str("\\\\"),
            32..=126 => result.push(char::from(byte)),
            _ => write!(result, "\\x{byte:02x}").expect("writing to a string cannot fail"),
        }
    }
    result
}

fn snapshot_example(name: &str, source: &str, results: &[(&str, &[u8])]) {
    let mut snapshot = format!("GENERATED C:\n{source}");
    for (label, bytes) in results {
        writeln!(snapshot, "\n{label}:").expect("writing to a string cannot fail");
        snapshot.push_str(&escaped(bytes));
        snapshot.push('\n');
    }
    insta::assert_snapshot!(name, snapshot);
}

#[test]
fn test_prog001_divisoes() {
    let source = generated_c("prog001_divisoes.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(output.stdout, b"Hello world\nEnio Basso - 2\n");
    snapshot_example("prog001_divisoes", &source, &[("STDOUT", &output.stdout)]);
}

#[test]
fn test_prog002_operacoes() {
    let source = generated_c("prog002_operacoes.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert!(escaped(&output.stdout).contains("A= 81 , B= 40 , C= 01"));
    assert!(escaped(&output.stdout).contains("ON SIZE ERROR -->99"));
    snapshot_example("prog002_operacoes", &source, &[("STDOUT", &output.stdout)]);
}

#[test]
fn test_prog003_calculadora() {
    let source = generated_c("prog003_calculadora.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(
        &executable,
        &directory,
        b"1\nS\n1.25\n2.50\n1\nU\n1.25\n2.50\n1\nM\n-1.25\n2\n\
          1\nD\n5\n2\n1\nP\n2\n0.5\n1\nP\n2\n-1\n2\n",
    );
    assert_success(&output);
    let text = escaped(&output.stdout);
    for value in ["3.75", "1.25", "-         2.50", "2.50", "1.41", "0.50"] {
        assert!(text.contains(value), "missing result {value}: {text}");
    }
    snapshot_example(
        "prog003_calculadora",
        &source,
        &[("STDOUT", &output.stdout)],
    );
}

#[test]
fn test_prog004_folha_pgto() {
    let source = generated_c("prog004_folha_pgto.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert!(output.stdout.contains(&0));
    assert!(escaped(&output.stdout).contains("TRABALHADOR(TID)=MARIA"));
    snapshot_example("prog004_folha_pgto", &source, &[("STDOUT", &output.stdout)]);
}

#[test]
fn test_prog005_strings() {
    let source = generated_c("prog005_strings.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    let text = escaped(&output.stdout);
    assert!(text.contains("CONT=15 - FOR CHARACTERS"));
    assert!(text.contains("SAUX='FFFFFFFFFFFFFFF'"));
    snapshot_example("prog005_strings", &source, &[("STDOUT", &output.stdout)]);
}

#[test]
fn test_prog006_files() {
    let source = generated_c("prog006_files.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    let file = fs::read(directory.path().join("DADOS.TXT")).expect("missing sequential file");
    assert_eq!(
        file,
        b"00001JOSEA     000001111111A1A4\
          00002MARIA     000002222222A2  \
          00003FABIO     000003333333A2A3"
    );
    snapshot_example(
        "prog006_files",
        &source,
        &[("STDOUT", &output.stdout), ("DADOS.TXT", &file)],
    );
}

#[test]
fn test_prog007_loja() {
    let source = generated_c("prog007_loja.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let first = run(
        &executable,
        &directory,
        b"1\nAPPLE\n12.50\n3\nFRESH\n4\n1\n5\n",
    );
    assert_success(&first);
    assert!(escaped(&first.stdout).contains("NOME: APPLE"));
    let second = run(
        &executable,
        &directory,
        b"4\n1\n1\nPEAR\n2.75\n5\nSWEET\n5\n",
    );
    assert_success(&second);
    assert!(escaped(&second.stdout).contains("NOME: APPLE"));
    let data = fs::read(directory.path().join("PROG10-001.DAT")).expect("missing indexed file");
    let key = fs::read(directory.path().join("PROG10-001.KEY")).expect("missing key file");
    assert_eq!(
        data,
        b"0000000001APPLE     000000001250000000000300FRESH               \
          0000000002PEAR      000000000275000000000500SWEET               "
    );
    assert_eq!(key, b"0000000002");
    snapshot_example(
        "prog007_loja",
        &source,
        &[
            ("FIRST RUN", &first.stdout),
            ("SECOND RUN", &second.stdout),
            ("PROG10-001.DAT", &data),
            ("PROG10-001.KEY", &key),
        ],
    );
}

#[test]
fn test_numeric() {
    let source = generated_c("codegen_test_numeric.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(
        output.stdout,
        b"SIGNED=-0000125\n\
          EDITED=-    1.25\n\
          SUM=     0.30\n\
          RATIONAL=     1.00\n\
          ROOT=     1.41\n\
          NEGATIVE-POWER=     0.50\n\
          SIZE=99\n\
          DIVZERO=42\n\
          QUOTIENT=03 REMAINDER=1\n"
    );
}

#[test]
fn test_qualified_names() {
    let source = generated_c("codegen_test_qualified_names.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(output.stdout, b"QUALIFIED=034\nSUM=046\n");
}

#[test]
fn test_file_status() {
    let source = generated_c("codegen_test_file_status.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(
        output.stdout,
        b"MISSING=35\nWRITE=00\nREAD-OUTPUT=47\nDUPLICATE=22\n\
          SIGNED=-02\nNOT-FOUND=23\n"
    );
    assert_eq!(
        fs::read(directory.path().join("status.dat")).unwrap(),
        b"01AA0rBB"
    );
}

#[test]
fn test_control_flow() {
    let source = generated_c("codegen_test_control_flow.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(output.stdout, b"HIT\nANY\nDONE\n??/??=??'\n");
    let parsed = compiler()
        .arg("parse")
        .arg(examples_dir().join("codegen_test_control_flow.cob"))
        .output()
        .unwrap();
    assert_success(&parsed);
    assert!(escaped(&parsed.stdout).contains("HIT with test after until"));
}

#[test]
fn test_move_overlap() {
    let source = generated_c("codegen_test_move_overlap.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(output.stdout, b"IDX=3 TEXT=123\n");
}

#[test]
fn test_unimplemented_variants_are_diagnosed() {
    for (fixture, message) in [
        (
            "codegen_test_file_duplicates.cob",
            "alternate key WITH DUPLICATES",
        ),
        (
            "codegen_test_indexed_sequential.cob",
            "sequential READ of an indexed file",
        ),
        ("codegen_test_unsupported_picture.cob", "edited PICTURE"),
    ] {
        let output = compiler()
            .arg("codegen")
            .arg(examples_dir().join(fixture))
            .output()
            .unwrap();
        assert!(!output.status.success(), "{fixture} unexpectedly succeeded");
        assert!(
            output.stdout.is_empty(),
            "{fixture} generated partial output"
        );
        let stderr = escaped(&output.stderr);
        assert!(
            stderr.contains("cobol::codegen::unsupported") && stderr.contains(message),
            "{fixture}: {stderr}"
        );
    }

    let parsed = compiler()
        .arg("parse")
        .arg(examples_dir().join("codegen_test_file_duplicates.cob"))
        .output()
        .unwrap();
    assert_success(&parsed);
    assert!(escaped(&parsed.stdout).contains("alternate key: TEST-NAME WITH DUPLICATES"));
}

#[test]
fn test_runtime_errors() {
    let directory = TestDirectory::new();
    let source = generated_c("codegen_test_file_error.cob");
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert!(!output.status.success());
    assert!(escaped(&output.stderr).contains("file status 35"));

    let source = generated_c("prog003_calculadora.cob");
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"1\nP\n-4\n0.5\n");
    assert!(!output.status.success());
    assert!(escaped(&output.stderr).contains("fractional power of a negative value"));

    let source = generated_c("codegen_test_bounds.cob");
    let executable = compile_c(&directory, &source);
    let output = run(&executable, &directory, b"");
    assert!(!output.status.success());
    assert!(escaped(&output.stderr).contains("table subscript out of bounds"));
}

#[test]
fn test_prog007_rejects_an_unreadable_key_record() {
    let source = generated_c("prog007_loja.cob");
    let directory = TestDirectory::new();
    let executable = compile_c(&directory, &source);
    fs::write(directory.path().join("PROG10-001.KEY"), b"").unwrap();
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert!(escaped(&output.stdout).contains("ERRO AO LER ARQUIVO DE CHAVES -->10"));
    assert_eq!(
        fs::read(directory.path().join("PROG10-001.KEY")).unwrap(),
        b""
    );
    assert!(!directory.path().join("PROG10-001.DAT").exists());
}

#[test]
fn test_cli_output_diagnostics_and_options() {
    let directory = TestDirectory::new();
    let input = directory.path().join("input.cob");
    let output_path = directory.path().join("output.c");
    fs::copy(examples_dir().join("codegen_test_copy.cob"), &input).unwrap();

    let missing_copybook = compiler().arg("codegen").arg(&input).output().unwrap();
    assert!(!missing_copybook.status.success());
    assert!(missing_copybook.stdout.is_empty());
    assert!(escaped(&missing_copybook.stderr).contains("cobol::preprocess::copybook_error"));

    let stdout = compiler()
        .arg("codegen")
        .arg(&input)
        .arg("-I")
        .arg(examples_dir())
        .output()
        .unwrap();
    assert_success(&stdout);
    let file_output = compiler()
        .arg("codegen")
        .arg(&input)
        .arg("-I")
        .arg(examples_dir())
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert_success(&file_output);
    assert!(file_output.stdout.is_empty());
    assert_eq!(fs::read(&output_path).unwrap(), stdout.stdout);

    fs::write(&output_path, vec![b'X'; stdout.stdout.len() + 16]).unwrap();
    let overwritten = compiler()
        .arg("codegen")
        .arg(&input)
        .arg("-I")
        .arg(examples_dir())
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert_success(&overwritten);
    assert_eq!(fs::read(&output_path).unwrap(), stdout.stdout);

    #[cfg(unix)]
    {
        let null_output = compiler()
            .arg("codegen")
            .arg(&input)
            .arg("-I")
            .arg(examples_dir())
            .arg("-o")
            .arg("/dev/null")
            .output()
            .unwrap();
        assert_success(&null_output);
    }

    let executable = compile_c(&directory, &String::from_utf8(stdout.stdout).unwrap());
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(output.stdout, b"COPIED\n");

    let debug = compiler()
        .arg("codegen")
        .arg(&input)
        .arg("-I")
        .arg(examples_dir())
        .arg("--debug-lines")
        .output()
        .unwrap();
    assert_success(&debug);
    let executable = compile_c(&directory, &String::from_utf8(debug.stdout).unwrap());
    let output = run(&executable, &directory, b"");
    assert_success(&output);
    assert_eq!(output.stdout, b"DEBUG\nCOPIED\n");

    fs::write(&output_path, b"unchanged").unwrap();
    let unsupported = compiler()
        .arg("codegen")
        .arg(examples_dir().join("codegen_test_unsupported.cob"))
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(!unsupported.status.success());
    assert!(unsupported.stdout.is_empty());
    assert!(escaped(&unsupported.stderr).contains("cobol::codegen::unsupported"));
    assert_eq!(fs::read(&output_path).unwrap(), b"unchanged");

    let semantic_error = compiler()
        .arg("codegen")
        .arg(examples_dir().join("semantic_test_undefined_identifier.cob"))
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(!semantic_error.status.success());
    assert!(semantic_error.stdout.is_empty());
    assert!(escaped(&semantic_error.stderr).contains("cobol::semantic::undefined_identifier"));
    assert_eq!(fs::read(&output_path).unwrap(), b"unchanged");

    let warning = compiler()
        .arg("codegen")
        .arg(examples_dir().join("prog005_strings.cob"))
        .output()
        .unwrap();
    assert_success(&warning);
    assert!(warning.stdout.starts_with(b"#include"));
    assert!(escaped(&warning.stderr).contains("cobol::lexer::underscore_in_identifier"));

    let original = fs::read(&input).unwrap();
    let collision = compiler()
        .arg("codegen")
        .arg(&input)
        .arg("-I")
        .arg(examples_dir())
        .arg("-o")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!collision.status.success());
    assert_eq!(fs::read(&input).unwrap(), original);

    let hard_link = directory.path().join("input-hard-link.cob");
    fs::hard_link(&input, &hard_link).unwrap();
    let collision = compiler()
        .arg("codegen")
        .arg(&input)
        .arg("-I")
        .arg(examples_dir())
        .arg("-o")
        .arg(&hard_link)
        .output()
        .unwrap();
    assert!(!collision.status.success());
    assert!(escaped(&collision.stderr).contains("output file must differ from the COBOL input"));
    assert_eq!(fs::read(&input).unwrap(), original);

    #[cfg(unix)]
    {
        let symlink = directory.path().join("input-symlink.cob");
        std::os::unix::fs::symlink(&input, &symlink).unwrap();
        let collision = compiler()
            .arg("codegen")
            .arg(&input)
            .arg("-I")
            .arg(examples_dir())
            .arg("-o")
            .arg(&symlink)
            .output()
            .unwrap();
        assert!(!collision.status.success());
        assert!(
            escaped(&collision.stderr).contains("output file must differ from the COBOL input")
        );
        assert_eq!(fs::read(&input).unwrap(), original);
    }
}
