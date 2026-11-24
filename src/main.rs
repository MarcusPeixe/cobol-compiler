use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};

use cobol_compiler::diagnostics::Diagnostic;
use cobol_compiler::lexer::lex;
use cobol_compiler::parser::parse;
use cobol_compiler::preprocess::{FsLoader, preprocess};
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
    /// Parse the IDENTIFICATION DIVISION of a source file.
    Parse {
        file: PathBuf,
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

fn run_parse(path: &Path, mut include: Vec<PathBuf>, debug_lines: bool) -> anyhow::Result<ExitCode> {
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

    let (program, parse_diagnostics) = parse(&tokens);

    if !parse_diagnostics.iter().any(Diagnostic::is_error) {
        println!("IDENTIFICATION DIVISION");
        if let Some(name) = program.identification_division.program_name() {
            println!("PROGRAM-ID: {name}");
        } else {
            println!("PROGRAM-ID: (unnamed)");
        }

        if let Some(environment) = program.environment_division {
            println!("ENVIRONMENT DIVISION");
            if let Some(configuration) = environment.configuration_section {
                println!("CONFIGURATION SECTION");
                if let Some(source) = configuration.source_computer {
                    if let Some(name) = source.name {
                        println!("SOURCE-COMPUTER: {name}");
                    }
                }
                if let Some(object) = configuration.object_computer {
                    if let Some(name) = object.name {
                        println!("OBJECT-COMPUTER: {name}");
                    }
                }
            }
            if let Some(input_output) = environment.input_output_section {
                println!("INPUT-OUTPUT SECTION");
                for entry in input_output.file_control {
                    if let Some(destination) = entry.destination {
                        println!("SELECT {} ASSIGN TO {}", entry.file_name, destination);
                    } else {
                        println!("SELECT {}", entry.file_name);
                    }
                }
            }
        }
    }

    let failed = diagnostics.iter().any(Diagnostic::is_error)
        || parse_diagnostics.iter().any(Diagnostic::is_error);
    for diagnostic in diagnostics {
        eprintln!("{:?}", diagnostic.into_report(&map));
    }
    for diagnostic in parse_diagnostics {
        eprintln!("{:?}", diagnostic.into_report(&map));
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
