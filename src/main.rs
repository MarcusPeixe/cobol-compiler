use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};

use cobol_compiler::codegen;
use cobol_compiler::diagnostics::Diagnostic;
use cobol_compiler::lexer::lex;
use cobol_compiler::parser::parse;
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::semantic::{analyze, format_symbol_table};
use cobol_compiler::source::{ReaderOptions, SourceMap};

#[derive(Parser)]
#[command(version, about = "A COBOL compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Preprocess and tokenize a source file, printing one token per line.
    Lex {
        file: PathBuf,
        /// Directory to search for COPY books (repeatable). The source file's own directory is always searched first.
        #[arg(short = 'I', long = "include", value_name = "DIR")]
        include: Vec<PathBuf>,
        /// Compile lines with 'D' in column 7 instead of skipping them.
        #[arg(long)]
        debug_lines: bool,
    },
    /// Parse a source file and print the abstract syntax tree.
    Parse {
        file: PathBuf,
        /// Directory to search for COPY books (repeatable). The source file's own directory is always searched first.
        #[arg(short = 'I', long = "include", value_name = "DIR")]
        include: Vec<PathBuf>,
        /// Compile lines with 'D' in column 7 instead of skipping them.
        #[arg(long)]
        debug_lines: bool,
    },
    /// Perform semantic analysis on a source file.
    Analyze {
        file: PathBuf,
        /// Directory to search for COPY books (repeatable). The source file's own directory is always searched first.
        #[arg(short = 'I', long = "include", value_name = "DIR")]
        include: Vec<PathBuf>,
        /// Compile lines with 'D' in column 7 instead of skipping them.
        #[arg(long)]
        debug_lines: bool,
    },
    /// Generate a standalone C11 translation unit.
    Codegen {
        file: PathBuf,
        /// Write C to this file instead of stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        /// Directory to search for COPY books (repeatable). The source file's own directory is always searched first.
        #[arg(short = 'I', long = "include", value_name = "DIR")]
        include: Vec<PathBuf>,
        /// Compile lines with 'D' in column 7 instead of skipping them.
        #[arg(long)]
        debug_lines: bool,
    },
}

fn main() -> anyhow::Result<ExitCode> {
    match Cli::parse().command {
        Command::Lex {
            file,
            include,
            debug_lines,
        } => run_lex(&file, include, debug_lines),
        Command::Parse {
            file,
            include,
            debug_lines,
        } => run_parse(&file, include, debug_lines),
        Command::Analyze {
            file,
            include,
            debug_lines,
        } => run_analyze(&file, include, debug_lines),
        Command::Codegen {
            file,
            output,
            include,
            debug_lines,
        } => run_codegen(&file, output.as_deref(), include, debug_lines),
    }
}

fn run_lex(path: &Path, mut include: Vec<PathBuf>, debug_lines: bool) -> anyhow::Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut map = SourceMap::default();
    let root = map.add_file(path.display().to_string(), text);

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    include.insert(0, dir.to_path_buf());
    let loader = FsLoader {
        search_dirs: include,
    };

    let (logical, mut diagnostics) =
        preprocess(&mut map, root, ReaderOptions { debug_lines }, &loader);
    let (tokens, lex_diagnostics) = lex(&logical);
    diagnostics.extend(lex_diagnostics);

    for token in &tokens {
        let (line, col) = map.line_col(token.span.file, token.span.start);
        println!(
            "{}:{line:03}:{col:03}  {}",
            map.file(token.span.file).name,
            token.kind
        );
    }

    let failed = diagnostics.iter().any(Diagnostic::is_error);
    for diagnostic in diagnostics {
        eprintln!("{:?}", diagnostic.into_report(&map));
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn run_parse(
    path: &Path,
    mut include: Vec<PathBuf>,
    debug_lines: bool,
) -> anyhow::Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut map = SourceMap::default();
    let root = map.add_file(path.display().to_string(), text);

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    include.insert(0, dir.to_path_buf());
    let loader = FsLoader {
        search_dirs: include,
    };

    let (logical, mut diagnostics) =
        preprocess(&mut map, root, ReaderOptions { debug_lines }, &loader);
    let (tokens, lex_diagnostics) = lex(&logical);
    diagnostics.extend(lex_diagnostics);

    let (program, parse_diagnostics) = parse(tokens);
    diagnostics.extend(parse_diagnostics);

    print_ast(&program);

    let failed = diagnostics.iter().any(Diagnostic::is_error);
    for diagnostic in diagnostics {
        eprintln!("{:?}", diagnostic.into_report(&map));
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn print_ast(program: &cobol_compiler::parser::Program) {
    print!("{}", cobol_compiler::parser::format_program(program));
}

fn run_analyze(
    path: &Path,
    mut include: Vec<PathBuf>,
    debug_lines: bool,
) -> anyhow::Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut map = SourceMap::default();
    let root = map.add_file(path.display().to_string(), text);

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    include.insert(0, dir.to_path_buf());
    let loader = FsLoader {
        search_dirs: include,
    };

    let (logical, mut diagnostics) =
        preprocess(&mut map, root, ReaderOptions { debug_lines }, &loader);
    let (tokens, lex_diagnostics) = lex(&logical);
    diagnostics.extend(lex_diagnostics);

    let (program, parse_diagnostics) = parse(tokens);
    diagnostics.extend(parse_diagnostics);

    let (annotated, semantic_diagnostics) = analyze(&program);
    diagnostics.extend(semantic_diagnostics);

    // Print annotated AST
    print_ast(&annotated.program);

    // Print symbol table summary
    println!("\nSYMBOL TABLE:");
    print!("{}", format_symbol_table(&annotated.symbol_table));

    let failed = diagnostics.iter().any(Diagnostic::is_error);
    println!("\nSTORAGE LAYOUT:");
    print!(
        "{}",
        cobol_compiler::semantic::format_layout(&annotated.layout)
    );
    println!("\nDIAGNOSTICS:");
    if diagnostics.is_empty() {
        println!("  (none)");
    } else {
        for diagnostic in &diagnostics {
            eprintln!("{:?}", diagnostic.clone().into_report(&map));
        }
    }

    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn run_codegen(
    path: &Path,
    output: Option<&Path>,
    mut include: Vec<PathBuf>,
    debug_lines: bool,
) -> anyhow::Result<ExitCode> {
    let input =
        std::fs::File::open(path).with_context(|| format!("cannot read {}", path.display()))?;
    let mut input = same_file::Handle::from_file(input)
        .with_context(|| format!("cannot inspect {}", path.display()))?;
    let mut text = String::new();
    input
        .as_file_mut()
        .read_to_string(&mut text)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let mut map = SourceMap::default();
    let root = map.add_file(path.display().to_string(), text);

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    include.insert(0, dir.to_path_buf());
    let loader = FsLoader {
        search_dirs: include,
    };

    let (logical, mut diagnostics) =
        preprocess(&mut map, root, ReaderOptions { debug_lines }, &loader);
    let (tokens, lex_diagnostics) = lex(&logical);
    diagnostics.extend(lex_diagnostics);
    let (program, parse_diagnostics) = parse(tokens);
    diagnostics.extend(parse_diagnostics);
    let (analyzed, semantic_diagnostics) = analyze(&program);
    diagnostics.extend(semantic_diagnostics);

    let generated = if diagnostics.iter().any(Diagnostic::is_error) {
        None
    } else {
        match codegen::generate(&analyzed) {
            Ok(c) => Some(c),
            Err(codegen_diagnostics) => {
                diagnostics.extend(codegen_diagnostics);
                None
            }
        }
    };
    for diagnostic in diagnostics {
        eprintln!("{:?}", diagnostic.into_report(&map));
    }

    let Some(c) = generated else {
        return Ok(ExitCode::FAILURE);
    };
    if let Some(output) = output {
        let destination = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(output)
            .with_context(|| format!("cannot write {}", output.display()))?;
        let mut destination = same_file::Handle::from_file(destination)
            .with_context(|| format!("cannot inspect {}", output.display()))?;
        anyhow::ensure!(
            input != destination,
            "output file must differ from the COBOL input"
        );
        let destination = destination.as_file_mut();
        if destination
            .metadata()
            .with_context(|| format!("cannot inspect {}", output.display()))?
            .is_file()
        {
            destination
                .set_len(0)
                .with_context(|| format!("cannot write {}", output.display()))?;
        }
        destination
            .write_all(c.as_bytes())
            .with_context(|| format!("cannot write {}", output.display()))?;
    } else {
        std::io::stdout()
            .lock()
            .write_all(c.as_bytes())
            .context("cannot write generated C to stdout")?;
    }
    Ok(ExitCode::SUCCESS)
}
