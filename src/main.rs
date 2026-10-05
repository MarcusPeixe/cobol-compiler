use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};

use cobol_compiler::diagnostics::Diagnostic;
use cobol_compiler::lexer::lex;
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
}

fn main() -> anyhow::Result<ExitCode> {
    match Cli::parse().command {
        Command::Lex {
            file,
            include,
            debug_lines,
        } => run_lex(&file, include, debug_lines),
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
