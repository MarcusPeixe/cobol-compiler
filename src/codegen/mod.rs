//! Code generation: analyzed COBOL program to MIPS assembly for MARS.
//!
//! The generator walks the AST directly (no intermediate representation).
//! Working storage is one static buffer `ws` laid out by
//! [`crate::semantic::compute_layout`] and kept in DISPLAY format; arithmetic
//! converts fields to 32-bit scaled integers (so values are limited to about
//! nine digits) and converts back when storing. See `AGENTS.md` for the
//! conventions shared with the runtime in [`runtime`].

mod data;
mod expr;
mod indexed;
mod initialize;
mod inspect;
mod io;
mod runtime;
mod stmt;
mod wide;

use std::collections::{HashMap, HashSet};

use crate::diagnostics::Diagnostic;
use crate::parser::{Expression, Paragraph, PerformStatement, Statement};
use crate::semantic::{AnalyzedProgram, StorageLayout};
use crate::source::Span;

/// Generates MIPS assembly. Unsupported constructs produce
/// `cobol::codegen::unsupported` errors; the assembly is then incomplete and
/// must not be used.
pub fn generate(analyzed: &AnalyzedProgram) -> (String, Vec<Diagnostic>) {
    let mut generator = Generator::new(analyzed);
    generator.program(analyzed);
    generator.finish(analyzed)
}

pub(crate) struct Generator<'a> {
    layout: &'a StorageLayout,
    text: String,
    pool: Vec<Vec<u8>>,
    words: Vec<String>,
    indexes: Vec<String>,
    table_indexes: HashMap<Vec<String>, Vec<String>>,
    file_controls: HashMap<String, &'a crate::parser::FileControl>,
    needs_inspect: bool,
    needs_indexed: bool,
    needs_wide: bool,
    needs_wide_arithmetic: bool,
    edit_digits_size: usize,
    paragraphs: HashMap<String, usize>,
    perform_ends: HashSet<String>,
    labels: usize,
    diagnostics: Vec<Diagnostic>,
    /// Statement-level flags for arithmetic stores.
    guard: bool,
    size_check: bool,
    /// Minimum scale of a division result.
    div_scale: u32,
}

impl<'a> Generator<'a> {
    fn new(analyzed: &'a AnalyzedProgram) -> Self {
        let mut indexes = Vec::new();
        let mut table_indexes = HashMap::new();
        if let Some(data) = &analyzed.program.data_division
            && let Some(section) = &data.working_storage_section
        {
            for item in &section.data_items {
                collect_indexes(item, &mut Vec::new(), &mut indexes, &mut table_indexes);
            }
        }
        let mut paragraphs = HashMap::new();
        let file_controls = analyzed
            .program
            .environment_division
            .as_ref()
            .and_then(|division| division.input_output_section.as_ref())
            .map(|section| {
                section
                    .file_controls
                    .iter()
                    .map(|control| (control.file_name.to_ascii_uppercase(), control))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(procedure) = &analyzed.program.procedure_division {
            for (number, paragraph) in procedure.paragraphs.iter().enumerate() {
                paragraphs
                    .entry(paragraph.name.to_ascii_uppercase())
                    .or_insert(number);
            }
        }
        Generator {
            layout: &analyzed.layout,
            text: String::new(),
            pool: Vec::new(),
            words: Vec::new(),
            indexes,
            table_indexes,
            file_controls,
            needs_inspect: false,
            needs_indexed: false,
            needs_wide: false,
            needs_wide_arithmetic: false,
            edit_digits_size: 32,
            paragraphs,
            perform_ends: HashSet::new(),
            labels: 0,
            diagnostics: Vec::new(),
            guard: false,
            size_check: false,
            div_scale: 4,
        }
    }

    fn program(&mut self, analyzed: &AnalyzedProgram) {
        self.emit_raw("    .text\n    .globl main\nmain:\n");
        if let Some(procedure) = &analyzed.program.procedure_division {
            for paragraph in &procedure.paragraphs {
                collect_perform_ends(&paragraph.statements, &mut self.perform_ends);
            }
            for paragraph in &procedure.paragraphs {
                self.paragraph(paragraph);
            }
        }
        self.emit("li $v0, 10");
        self.emit("syscall");
    }

    fn paragraph(&mut self, paragraph: &Paragraph) {
        let name = paragraph.name.to_ascii_uppercase();
        let number = self.paragraphs[&name];
        self.label(&para_label(number, &name));
        self.statements(&paragraph.statements);
        if self.perform_ends.contains(&name) {
            let skip = self.new_label("pend");
            self.emit("lw $t0, perf_sp");
            self.emit(format!("beqz $t0, {skip}"));
            self.emit("la $t1, perf_stack");
            self.emit("addu $t1, $t1, $t0");
            self.emit("lw $t2, -8($t1)");
            self.emit(format!("li $t3, {}", number + 1));
            self.emit(format!("bne $t2, $t3, {skip}"));
            self.emit("lw $t4, -4($t1)");
            self.emit("addiu $t0, $t0, -8");
            self.emit("sw $t0, perf_sp");
            self.emit("jr $t4");
            self.label(&skip);
        }
    }

    fn finish(mut self, analyzed: &AnalyzedProgram) -> (String, Vec<Diagnostic>) {
        let mut out = format!(
            "# Generated by cobol-compiler from program {}\n    .data\nws:\n",
            analyzed.program.identification_division.program_id
        );
        out.push_str(&data::data_directives(&data::initial_image(self.layout)));
        out.push_str("    .align 2\n");
        out.push_str("size_err: .word 0\nperf_sp: .word 0\n");
        for (number, file) in self.layout.files.iter().enumerate() {
            out.push_str(&format!(
                "file_fd_{number}: .word -1\nfile_mode_{number}: .word 0\nfile_buf_{number}: .space {}\n    .align 2\n",
                file.record_size.max(1)
            ));
            if self
                .file_controls
                .get(&file.name)
                .is_some_and(|control| control.organization.as_deref() == Some("INDEXED"))
            {
                out.push_str(&format!(
                    "file_index_{number}: .word 0\nfile_cursor_{number}: .word 0\nfile_cursor_pending_{number}: .word 0\nfile_cursor_offset_{number}: .word 0\nfile_cursor_length_{number}: .word 0\nfile_cursor_signed_{number}: .word 0\nfile_path_{number}: .space 256\n"
                ));
            }
        }
        for index in &self.indexes {
            out.push_str(&format!("{}: .word 1\n", index_label(index)));
        }
        for word in &self.words {
            out.push_str(&format!("{word}: .word 0\n"));
        }
        out.push_str("perf_stack: .space 1024\n");
        out.push_str(&format!(
            "tmp_buf: .space 256\nfn_buf: .space 256\ned_digits: .space {}\n",
            self.edit_digits_size
        ));
        for (number, bytes) in self.pool.iter().enumerate() {
            out.push_str(&format!("lit_{number}:\n"));
            out.push_str(&data::literal_directive(bytes, true));
        }
        out.push_str(&self.text);
        if self.needs_indexed {
            out.push_str(indexed::RUNTIME);
        }
        if self.needs_inspect {
            out.push_str(inspect::RUNTIME);
        }
        if self.needs_wide {
            out.push_str(wide::RUNTIME);
        }
        if self.needs_wide_arithmetic {
            out.push_str(wide::ARITHMETIC_RUNTIME);
        }
        out.push_str(runtime::RUNTIME);
        self.text = out;
        (self.text, self.diagnostics)
    }

    fn emit(&mut self, line: impl AsRef<str>) {
        self.text.push_str("    ");
        self.text.push_str(line.as_ref());
        self.text.push('\n');
    }

    fn emit_raw(&mut self, text: &str) {
        self.text.push_str(text);
    }

    fn label(&mut self, name: &str) {
        self.text.push_str(name);
        self.text.push_str(":\n");
    }

    fn new_label(&mut self, prefix: &str) -> String {
        self.labels += 1;
        format!("{prefix}_{}", self.labels)
    }

    /// Allocates a zero-initialized data word and returns its label.
    fn new_word(&mut self, prefix: &str) -> String {
        let label = self.new_label(prefix);
        self.words.push(label.clone());
        label
    }

    /// Interns a literal in the string pool and returns its label.
    fn literal(&mut self, bytes: &[u8]) -> String {
        let position = match self.pool.iter().position(|known| known == bytes) {
            Some(position) => position,
            None => {
                self.pool.push(bytes.to_vec());
                self.pool.len() - 1
            }
        };
        format!("lit_{position}")
    }

    fn unsupported(&mut self, span: Span, what: &str) {
        self.diagnostics.push(
            Diagnostic::error(
                "cobol::codegen::unsupported",
                format!("{what} is not supported by the code generator"),
            )
            .with_span(span),
        );
    }

    fn push_v0(&mut self) {
        self.emit("addiu $sp, $sp, -4");
        self.emit("sw $v0, 0($sp)");
    }

    fn pop(&mut self, register: &str) {
        self.emit(format!("lw {register}, 0($sp)"));
        self.emit("addiu $sp, $sp, 4");
    }

    /// Pushes the `PERFORM` frame and jumps to `first`; execution resumes
    /// after the call once the paragraph `last` finishes.
    fn perform(&mut self, first: &str, last: &str, span: Span) {
        let (Some(&first_number), Some(&last_number)) = (
            self.paragraphs.get(&first.to_ascii_uppercase()),
            self.paragraphs.get(&last.to_ascii_uppercase()),
        ) else {
            self.unsupported(span, &format!("PERFORM of unknown paragraph {first}"));
            return;
        };
        let resume = self.new_label("ret");
        self.emit("lw $t0, perf_sp");
        self.emit("la $t1, perf_stack");
        self.emit("addu $t1, $t1, $t0");
        self.emit(format!("li $t2, {}", last_number + 1));
        self.emit("sw $t2, 0($t1)");
        self.emit(format!("la $t2, {resume}"));
        self.emit("sw $t2, 4($t1)");
        self.emit("addiu $t0, $t0, 8");
        self.emit("sw $t0, perf_sp");
        self.emit(format!(
            "j {}",
            para_label(first_number, &first.to_ascii_uppercase())
        ));
        self.label(&resume);
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn para_label(number: usize, name: &str) -> String {
    format!("para_{number}_{}", sanitize(name))
}

fn index_label(name: &str) -> String {
    format!("idx_{}", sanitize(name))
}

fn collect_indexes(
    item: &crate::parser::DataItem,
    path: &mut Vec<String>,
    out: &mut Vec<String>,
    table_indexes: &mut HashMap<Vec<String>, Vec<String>>,
) {
    path.push(item.name.to_ascii_uppercase());
    if !item.indexed_by.is_empty() {
        table_indexes.insert(
            path.clone(),
            item.indexed_by
                .iter()
                .map(|name| name.to_ascii_uppercase())
                .collect(),
        );
    }
    for name in &item.indexed_by {
        out.push(name.to_ascii_uppercase());
    }
    for child in &item.children {
        collect_indexes(child, path, out, table_indexes);
    }
    path.pop();
}

/// Records every paragraph that some `PERFORM` can finish at.
fn collect_perform_ends(statements: &[Statement], ends: &mut HashSet<String>) {
    for statement in statements {
        if let Statement::Perform(perform) = statement {
            match perform {
                PerformStatement::ProcedureName(name, _) => {
                    ends.insert(name.to_ascii_uppercase());
                }
                PerformStatement::Thru { last, .. } => {
                    ends.insert(last.to_ascii_uppercase());
                }
                PerformStatement::Times { paragraph, .. }
                | PerformStatement::Until { paragraph, .. } => {
                    ends.insert(paragraph.to_ascii_uppercase());
                }
                PerformStatement::Varying(varying) => {
                    if let Some(paragraph) = &varying.paragraph {
                        ends.insert(paragraph.to_ascii_uppercase());
                    }
                }
                PerformStatement::Inline(_) => {}
            }
        }
        for nested in stmt::nested_statements(statement) {
            collect_perform_ends(nested, ends);
        }
    }
}

pub(crate) fn expression_span(expression: &Expression) -> Span {
    match expression {
        Expression::Identifier(_, span)
        | Expression::NumericLiteral(_, span)
        | Expression::StringLiteral(_, span)
        | Expression::FigurativeConstant(_, span)
        | Expression::Subscript { span, .. }
        | Expression::RefMod { span, .. }
        | Expression::FunctionCall { span, .. }
        | Expression::BinaryOp { span, .. }
        | Expression::UnaryOp { span, .. } => *span,
    }
}

#[cfg(test)]
mod tests;
