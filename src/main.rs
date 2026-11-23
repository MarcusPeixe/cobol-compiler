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

    print_ast(&program, 0);

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

fn print_ast(program: &cobol_compiler::parser::Program, indent: usize) {
    let ind = " ".repeat(indent);
    println!("{}Program", ind);
    println!("{}  IdentificationDivision", ind);
    println!(
        "{}    program_id: {}",
        ind, program.identification_division.program_id
    );
    if let Some(author) = &program.identification_division.author {
        println!("{}    author: {}", ind, author);
    }
    if let Some(installation) = &program.identification_division.installation {
        println!("{}    installation: {}", ind, installation);
    }
    if let Some(date_written) = &program.identification_division.date_written {
        println!("{}    date_written: {}", ind, date_written);
    }
    if let Some(date_compiled) = &program.identification_division.date_compiled {
        println!("{}    date_compiled: {}", ind, date_compiled);
    }
    if let Some(security) = &program.identification_division.security {
        println!("{}    security: {}", ind, security);
    }
    if let Some(remarks) = &program.identification_division.remarks {
        println!("{}    remarks: {}", ind, remarks);
    }

    if let Some(env_div) = &program.environment_division {
        println!("{}  EnvironmentDivision", ind);
        if env_div.configuration_section.is_some() {
            println!("{}    ConfigurationSection", ind);
        }
        if env_div.input_output_section.is_some() {
            println!("{}    InputOutputSection", ind);
        }
    }

    if let Some(data_div) = &program.data_division {
        println!("{}  DataDivision", ind);
        if let Some(file_sec) = &data_div.file_section {
            println!("{}    FileSection", ind);
            for file_desc in &file_sec.file_descriptions {
                println!("{}      FileDescription: {}", ind, file_desc.file_name);
            }
        }
        if let Some(work_sec) = &data_div.working_storage_section {
            println!("{}    WorkingStorageSection", ind);
            print_data_items(&work_sec.data_items, indent + 6);
        }
    }

    if let Some(proc_div) = &program.procedure_division {
        println!("{}  ProcedureDivision", ind);
        for para in &proc_div.paragraphs {
            println!("{}    Paragraph: {}", ind, para.name);
            if para.statements.is_empty() {
                println!("{}      (no statements)", ind);
            } else {
                print_statements(&para.statements, indent + 6);
            }
        }
    }
}

fn print_statements(statements: &[cobol_compiler::parser::Statement], indent: usize) {
    for (i, stmt) in statements.iter().enumerate() {
        print_statement(stmt, indent, i);
    }
}

fn print_statement(stmt: &cobol_compiler::parser::Statement, indent: usize, index: usize) {
    use cobol_compiler::parser::Statement;
    let ind = " ".repeat(indent);

    match stmt {
        Statement::Display(display_stmt) => {
            println!("{}[{}] DISPLAY", ind, index);
            for item in &display_stmt.items {
                match item {
                    cobol_compiler::parser::DisplayItem::String(s, _) => {
                        println!("{}      \"{}\"", ind, s);
                    }
                    cobol_compiler::parser::DisplayItem::Expression(expr) => {
                        println!("{}      {}", ind, format_expression(expr));
                    }
                }
            }
        }
        Statement::Move(move_stmt) => {
            println!(
                "{}[{}] MOVE {} TO",
                ind,
                index,
                format_expression(&move_stmt.source)
            );
            for target in &move_stmt.targets {
                println!("{}      {}", ind, format_expression(target));
            }
        }
        Statement::Accept(accept_stmt) => {
            println!(
                "{}[{}] ACCEPT {}",
                ind,
                index,
                format_expression(&accept_stmt.target)
            );
        }
        Statement::Perform(perf_stmt) => match perf_stmt {
            cobol_compiler::parser::PerformStatement::ProcedureName(name, _) => {
                println!("{}[{}] PERFORM {}", ind, index, name);
            }
            cobol_compiler::parser::PerformStatement::Times { .. } => {
                println!("{}[{}] PERFORM ... TIMES", ind, index);
            }
            cobol_compiler::parser::PerformStatement::Until { .. } => {
                println!("{}[{}] PERFORM ... UNTIL", ind, index);
            }
        },
        Statement::If(if_stmt) => {
            println!("{}[{}] IF", ind, index);
            println!("{}      then-statements:", ind);
            if if_stmt.then_statements.is_empty() {
                println!("{}        (none)", ind);
            } else {
                print_statements(&if_stmt.then_statements, indent + 8);
            }
            if let Some(else_stmts) = &if_stmt.else_statements
                && !else_stmts.is_empty()
            {
                println!("{}      else-statements:", ind);
                print_statements(else_stmts, indent + 8);
            }
        }
        Statement::StopRun(_) => {
            println!("{}[{}] STOP RUN", ind, index);
        }
        Statement::GoTo(goto_stmt) => {
            println!("{}[{}] GO TO {}", ind, index, goto_stmt.target);
        }
        Statement::Paragraph(name, _) => {
            println!("{}[{}] PARAGRAPH {}", ind, index, name);
        }
    }
}

fn format_expression(expr: &cobol_compiler::parser::Expression) -> String {
    use cobol_compiler::parser::Expression;

    match expr {
        Expression::Identifier(name, _) => name.clone(),
        Expression::NumericLiteral(n, _) => n.clone(),
        Expression::StringLiteral(s, _) => format!("\"{}\"", s),
        Expression::BinaryOp {
            left, op, right, ..
        } => {
            format!(
                "{} {} {}",
                format_expression(left),
                format_binary_op(op),
                format_expression(right)
            )
        }
        Expression::UnaryOp { op, operand, .. } => {
            format!("{}{}", format_unary_op(op), format_expression(operand))
        }
    }
}

fn format_binary_op(op: &cobol_compiler::parser::BinaryOperator) -> &'static str {
    use cobol_compiler::parser::BinaryOperator;

    match op {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Power => "**",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "<>",
        BinaryOperator::LessThan => "<",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::GreaterThanOrEqual => ">=",
    }
}

fn format_unary_op(op: &cobol_compiler::parser::UnaryOperator) -> &'static str {
    use cobol_compiler::parser::UnaryOperator;

    match op {
        UnaryOperator::Plus => "+",
        UnaryOperator::Minus => "-",
    }
}

fn print_data_items(items: &[cobol_compiler::parser::DataItem], indent: usize) {
    let ind = " ".repeat(indent);
    for item in items {
        println!("{}DataItem (level {}): {}", ind, item.level, item.name);
        if let Some(picture) = &item.picture {
            println!("{}  PIC: {}", ind, picture);
        }
        if !item.children.is_empty() {
            print_data_items(&item.children, indent + 2);
        }
    }
}
