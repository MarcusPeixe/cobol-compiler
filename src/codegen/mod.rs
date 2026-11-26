//! Translation of analyzed COBOL programs into standalone C11.

use std::fmt::Write as _;

use crate::diagnostics::Diagnostic;
use crate::parser::{
    AddStatement, BinaryOperator, Condition, DataItem, DisplayItem, DivideStatement,
    EvaluateChoice, EvaluateStatement, Expression, FileControl, InspectMode, InspectRegion,
    InspectStatement, IoHandlers, Literal, MultiplyStatement, OpenStatement, PerformStatement,
    ReadStatement, SearchStatement, SetKind, SetStatement, StartStatement, Statement,
    SubtractStatement, UnaryOperator, WriteStatement,
};
use crate::semantic::{AnalyzedProgram, DataType, ItemLayout, PictureInfo, Scope, SymbolKind};
use crate::source::Span;

const RUNTIME: &str = include_str!("runtime.c");

/// Generates C only when all encountered constructs can be translated.
pub fn generate(program: &AnalyzedProgram) -> Result<String, Vec<Diagnostic>> {
    let mut generator = Generator {
        program,
        output: String::new(),
        diagnostics: Vec::new(),
        items: Vec::new(),
        indexes: Vec::new(),
        files: Vec::new(),
        next_temporary: 0,
    };
    generator.collect_storage();
    generator.collect_files();
    generator.emit_program();
    if generator.diagnostics.is_empty() {
        Ok(generator.output)
    } else {
        Err(generator.diagnostics)
    }
}

struct Generator<'a> {
    program: &'a AnalyzedProgram,
    output: String,
    diagnostics: Vec<Diagnostic>,
    items: Vec<Item>,
    indexes: Vec<Index>,
    files: Vec<File>,
    next_temporary: usize,
}

#[derive(Clone)]
struct Dimension {
    count: usize,
    stride: usize,
}

#[derive(Clone)]
struct Item {
    declaration: DataItem,
    offset: usize,
    element_size: usize,
    picture: Option<PictureInfo>,
    dimensions: Vec<Dimension>,
    file: Option<usize>,
}

#[derive(Clone)]
struct Index {
    name: String,
    table: Span,
    count: usize,
}

#[derive(Clone)]
struct File {
    index: usize,
    control: FileControl,
    record: Item,
    keys: Vec<Item>,
    indexed: bool,
}

enum Binding {
    Item(usize),
    Index(usize),
}

impl Generator<'_> {
    fn collect_storage(&mut self) {
        let Some(data) = self.program.program.data_division.clone() else {
            return;
        };
        let layout = self.program.layout.clone();
        if let Some(working_storage) = data.working_storage_section {
            for (item, item_layout) in working_storage
                .data_items
                .iter()
                .zip(&layout.working_storage)
            {
                self.collect_item(item, item_layout, &[], None);
            }
        }
        if let Some(file_section) = data.file_section {
            for (file_index, file_layout) in layout.files.iter().enumerate() {
                if let Some(file) = file_section
                    .file_descriptions
                    .iter()
                    .find(|file| file.file_name.eq_ignore_ascii_case(&file_layout.name))
                {
                    for (record, record_layout) in file.records.iter().zip(&file_layout.records) {
                        self.collect_item(record, record_layout, &[], Some(file_index));
                    }
                }
            }
        }
    }

    fn collect_item(
        &mut self,
        declaration: &DataItem,
        layout: &ItemLayout,
        parents: &[Dimension],
        file: Option<usize>,
    ) {
        let mut dimensions = parents.to_vec();
        if let Some(count) = layout.occurs {
            dimensions.push(Dimension {
                count,
                stride: layout.element_size,
            });
            for name in &declaration.indexed_by {
                self.indexes.push(Index {
                    name: name.clone(),
                    table: declaration.span,
                    count,
                });
            }
        }
        if declaration
            .picture
            .as_ref()
            .is_some_and(|picture| picture.contains('P'))
        {
            self.unsupported(declaration.span, "PICTURE scaling position");
        }
        if layout
            .picture
            .as_ref()
            .is_some_and(|picture| picture.digits > 2048)
        {
            self.unsupported(declaration.span, "PICTURE exceeding runtime precision");
        }
        if layout
            .picture
            .as_ref()
            .is_some_and(|picture| picture.edited)
        {
            let supported = declaration
                .picture
                .as_deref()
                .and_then(expand_picture)
                .is_some_and(|picture| {
                    layout.picture.as_ref().is_some_and(|info| {
                        info.data_type == DataType::Numeric && supported_edited_picture(&picture)
                    })
                });
            if !supported {
                self.unsupported(declaration.span, "edited PICTURE");
            }
        }
        self.items.push(Item {
            declaration: declaration.clone(),
            offset: layout.offset,
            element_size: layout.element_size,
            picture: layout.picture.clone(),
            dimensions: dimensions.clone(),
            file,
        });
        for (child, child_layout) in declaration.children.iter().zip(&layout.children) {
            self.collect_item(child, child_layout, &dimensions, file);
        }
    }

    fn collect_files(&mut self) {
        let controls = self
            .program
            .program
            .environment_division
            .as_ref()
            .and_then(|division| division.input_output_section.as_ref())
            .map(|section| section.file_controls.clone())
            .unwrap_or_default();
        let layouts = self.program.layout.files.clone();
        for (index, layout) in layouts.iter().enumerate() {
            let Some(control) = controls
                .iter()
                .find(|control| control.file_name.eq_ignore_ascii_case(&layout.name))
                .cloned()
            else {
                let span = self.program.program.identification_division.span;
                self.unsupported(span, "file without SELECT");
                continue;
            };
            if layout.records.len() != 1 || layout.record_size == 0 {
                self.unsupported(control.span, "file with multiple or empty records");
                continue;
            }
            let Some(record) = self
                .items
                .iter()
                .find(|item| item.file == Some(index) && item.declaration.level == 1)
                .cloned()
            else {
                self.unsupported(control.span, "file without a record layout");
                continue;
            };
            let indexed = match control.organization.as_deref() {
                None | Some("SEQUENTIAL") => false,
                Some("INDEXED") => true,
                _ => {
                    self.unsupported(control.span, "file organization");
                    continue;
                }
            };
            if indexed {
                if !matches!(control.access_mode.as_deref(), Some("DYNAMIC" | "RANDOM")) {
                    self.unsupported(
                        control.span,
                        "indexed access mode other than DYNAMIC or RANDOM",
                    );
                }
            } else if !matches!(control.access_mode.as_deref(), None | Some("SEQUENTIAL")) {
                self.unsupported(control.span, "sequential access mode");
            }
            if !indexed && (control.record_key.is_some() || !control.alternate_keys.is_empty()) {
                self.unsupported(control.span, "keys on a sequential file");
            }
            if control.assign_to.is_none() {
                self.unsupported(control.span, "file without ASSIGN");
            }
            for key in &control.alternate_keys {
                if key.duplicates {
                    self.unsupported(
                        expression_span(&key.expression),
                        "alternate key WITH DUPLICATES",
                    );
                }
            }
            let mut keys = Vec::new();
            for key in control
                .record_key
                .iter()
                .chain(control.alternate_keys.iter().map(|key| &key.expression))
            {
                let Expression::Identifier(name, span) = key else {
                    self.unsupported(expression_span(key), "file key expression");
                    continue;
                };
                match self.resolve(name, *span) {
                    Some(Binding::Item(item_index)) => {
                        let item = &self.items[item_index];
                        if item.file == Some(index)
                            && item.dimensions.is_empty()
                            && !item.picture.as_ref().is_some_and(|picture| picture.edited)
                            && item.offset + item.element_size <= layout.record_size
                        {
                            keys.push(item.clone());
                        } else {
                            self.unsupported(*span, "file key layout");
                        }
                    }
                    Some(_) => self.unsupported(*span, "file key layout"),
                    None => {}
                }
            }
            if indexed && control.record_key.is_none() {
                self.unsupported(control.span, "indexed file without a record key");
            }
            self.files.push(File {
                index,
                control,
                record,
                keys,
                indexed,
            });
        }
    }

    fn emit_program(&mut self) {
        self.output.push_str(RUNTIME);
        self.output.push('\n');

        writeln!(
            self.output,
            "static unsigned char cob_ws[{}];",
            self.program.layout.working_storage_size.max(1)
        )
        .unwrap();
        for (index, file) in self.program.layout.files.iter().enumerate() {
            writeln!(
                self.output,
                "static unsigned char cob_file_buffer_{index}[{}];",
                file.record_size.max(1)
            )
            .unwrap();
        }
        for index in 0..self.indexes.len() {
            writeln!(self.output, "static long long cob_index_{index};").unwrap();
        }
        for file in &self.files {
            if !file.keys.is_empty() {
                writeln!(
                    self.output,
                    "static const CobKey cob_keys_{}[] = {{",
                    file.index
                )
                .unwrap();
                for key in &file.keys {
                    let (digits, scale, kind, signed) = match &key.picture {
                        Some(picture) if picture.data_type == DataType::Numeric => (
                            picture.digits,
                            picture.scale,
                            1,
                            usize::from(picture.signed),
                        ),
                        _ => (0, 0, 0, 0),
                    };
                    writeln!(
                        self.output,
                        "    {{{}, {}, {digits}, {scale}, {kind}, {signed}}},",
                        key.offset, key.element_size
                    )
                    .unwrap();
                }
                self.output.push_str("};\n");
            }
            writeln!(self.output, "static CobFile cob_file_{};", file.index).unwrap();
        }
        self.output.push('\n');
        self.emit_initialization();

        let paragraphs = self
            .program
            .program
            .procedure_division
            .as_ref()
            .map(|division| division.paragraphs.clone())
            .unwrap_or_default();

        for index in 0..paragraphs.len() {
            writeln!(self.output, "static void cob_paragraph_{index}(void);").unwrap();
        }
        self.output.push('\n');
        for (index, paragraph) in paragraphs.iter().enumerate() {
            writeln!(self.output, "static void cob_paragraph_{index}(void) {{").unwrap();
            for statement in &paragraph.statements {
                self.emit_statement(statement, 1);
            }
            self.output.push_str("}\n\n");
        }
        self.output
            .push_str("int main(void) {\n    cob_initialize();\n");
        for index in 0..paragraphs.len() {
            writeln!(self.output, "    cob_paragraph_{index}();").unwrap();
        }
        self.output.push_str("    return EXIT_SUCCESS;\n}\n");
    }

    fn emit_initialization(&mut self) {
        self.output.push_str("static void cob_initialize(void) {\n");
        self.line(1, "memset(cob_ws, ' ', sizeof(cob_ws));");
        for index in 0..self.program.layout.files.len() {
            self.line(
                1,
                &format!("memset(cob_file_buffer_{index}, ' ', sizeof(cob_file_buffer_{index}));"),
            );
        }
        for index in 0..self.indexes.len() {
            self.line(1, &format!("cob_index_{index} = 0;"));
        }
        for (item_index, item) in self.items.clone().iter().enumerate() {
            if !item.declaration.children.is_empty() {
                if item.declaration.value.is_some() {
                    self.unsupported(item.declaration.span, "VALUE on a group item");
                }
                continue;
            }
            if !item_is_numeric(item) && item.declaration.value.is_none() {
                continue;
            }
            let mut offset = item.offset.to_string();
            for (dimension_index, dimension) in item.dimensions.iter().enumerate() {
                let variable = format!("cob_init_{item_index}_{dimension_index}");
                self.line(
                    dimension_index + 1,
                    &format!(
                        "for (size_t {variable} = 0; {variable} < {}; ++{variable}) {{",
                        dimension.count
                    ),
                );
                write!(offset, " + {variable} * {}", dimension.stride).unwrap();
            }
            let indent = item.dimensions.len() + 1;
            let field = self.field_for_item(item, &offset);
            if item
                .picture
                .as_ref()
                .is_some_and(|picture| picture.data_type == DataType::Numeric)
            {
                self.line(
                    indent,
                    &format!("(void)cob_store({field}, cob_num_integer(0), 0);"),
                );
            }
            if let Some(value) = &item.declaration.value {
                self.emit_initial_value(item, &field, value, indent);
            }
            for dimension_index in (0..item.dimensions.len()).rev() {
                self.line(dimension_index + 1, "}");
            }
        }
        for file in self.files.clone() {
            self.line(
                1,
                &format!("cob_file_{0}.record = cob_file_buffer_{0};", file.index),
            );
            self.line(
                1,
                &format!(
                    "cob_file_{}.record_size = {};",
                    file.index, file.record.element_size
                ),
            );
            self.line(
                1,
                &format!(
                    "cob_file_{}.indexed = {};",
                    file.index,
                    usize::from(file.indexed)
                ),
            );
            if !file.keys.is_empty() {
                self.line(1, &format!("cob_file_{0}.keys = cob_keys_{0};", file.index));
                self.line(
                    1,
                    &format!("cob_file_{}.key_count = {};", file.index, file.keys.len()),
                );
            }
            if let Some(status) = &file.control.file_status
                && let Some((field, item)) = self.field_expression(status)
            {
                if item.element_size < 2 {
                    self.unsupported(expression_span(status), "short FILE STATUS item");
                } else {
                    self.line(1, &format!("cob_file_{}.status = {field};", file.index));
                    self.line(1, &format!("cob_file_{}.has_status = 1;", file.index));
                }
            }
        }
        self.output.push_str("}\n\n");
    }

    fn emit_initial_value(&mut self, item: &Item, field: &str, value: &Literal, indent: usize) {
        let numeric = item
            .picture
            .as_ref()
            .is_some_and(|picture| picture.data_type == DataType::Numeric);
        match value {
            Literal::Numeric(value, _) if numeric => self.line(
                indent,
                &format!(
                    "(void)cob_store({field}, cob_num_parse({}, {}), 0);",
                    c_string(value),
                    value.len()
                ),
            ),
            Literal::String(value, _) if !numeric => self.line(
                indent,
                &format!(
                    "cob_move_text({field}, cob_text({}, {}));",
                    c_string(value),
                    value.len()
                ),
            ),
            Literal::FigurativeConstant(value, span) => {
                self.emit_fill(field, numeric, value, *span, indent);
            }
            Literal::Numeric(_, span) | Literal::String(_, span) => {
                self.unsupported(*span, "VALUE conversion");
            }
        }
    }

    fn field_for_item(&mut self, item: &Item, offset: &str) -> String {
        let storage = item.file.map_or_else(
            || "cob_ws".to_string(),
            |index| format!("cob_file_buffer_{index}"),
        );
        let (digits, scale, kind, signed, picture) = match &item.picture {
            Some(picture) if picture.data_type == DataType::Numeric => {
                let expanded = if picture.edited {
                    match expand_picture(item.declaration.picture.as_deref().unwrap_or_default()) {
                        Some(value) => value,
                        None => {
                            self.unsupported(item.declaration.span, "edited PICTURE");
                            String::new()
                        }
                    }
                } else {
                    String::new()
                };
                (
                    picture.digits,
                    picture.scale,
                    if picture.edited { 2 } else { 1 },
                    usize::from(picture.signed),
                    expanded,
                )
            }
            _ => (0, 0, 0, 0, String::new()),
        };
        format!(
            "cob_field({storage} + ({offset}), {}, {digits}, {scale}, {kind}, {signed}, {})",
            item.element_size,
            c_string(&picture)
        )
    }

    fn resolve(&mut self, name: &str, span: Span) -> Option<Binding> {
        let symbols = self
            .program
            .symbol_table
            .lookup_data_name(name, Scope::Global);
        let [symbol] = symbols.as_slice() else {
            self.unsupported(span, "unresolved or ambiguous data name");
            return None;
        };
        match symbol.kind {
            SymbolKind::DataItem => {
                if let Some(item) = self
                    .items
                    .iter()
                    .position(|item| item.declaration.span == symbol.span)
                {
                    Some(Binding::Item(item))
                } else {
                    self.unsupported(span, "data item without storage");
                    None
                }
            }
            SymbolKind::IndexName => {
                if let Some(index) = self
                    .indexes
                    .iter()
                    .position(|index| index.name.eq_ignore_ascii_case(&symbol.name))
                {
                    Some(Binding::Index(index))
                } else {
                    self.unsupported(span, "index without a table");
                    None
                }
            }
            _ => {
                self.unsupported(span, "data reference");
                None
            }
        }
    }

    fn field_expression(&mut self, expression: &Expression) -> Option<(String, Item)> {
        let (name, indices, span) = match expression {
            Expression::Identifier(name, span) => (name, &[][..], *span),
            Expression::Subscript {
                name,
                indices,
                span,
            } => (name, indices.as_slice(), *span),
            _ => {
                self.unsupported(expression_span(expression), "storage reference");
                return None;
            }
        };
        let Binding::Item(item_index) = self.resolve(name, span)? else {
            self.unsupported(span, "index as a storage item");
            return None;
        };
        let item = self.items[item_index].clone();
        if item.dimensions.len() != indices.len() {
            self.unsupported(span, "table reference without matching subscripts");
            return None;
        }
        let mut offset = item.offset.to_string();
        for (dimension, index) in item.dimensions.iter().zip(indices) {
            let value = self.numeric_expression(index, None)?;
            write!(
                offset,
                " + cob_index({value}, {}) * {}",
                dimension.count, dimension.stride
            )
            .unwrap();
        }
        let field = self.field_for_item(&item, &offset);
        Some((field, item))
    }

    fn numeric_expression(
        &mut self,
        expression: &Expression,
        power_scale: Option<usize>,
    ) -> Option<String> {
        match expression {
            Expression::NumericLiteral(value, _) => Some(format!(
                "cob_num_parse({}, {})",
                c_string(value),
                value.len()
            )),
            Expression::FigurativeConstant(value, _) if is_zero(value) => {
                Some("cob_num_integer(0)".to_string())
            }
            Expression::Identifier(name, span) => match self.resolve(name, *span)? {
                Binding::Index(index) => Some(format!("cob_num_integer(cob_index_{index})")),
                Binding::Item(_) => {
                    let (field, item) = self.field_expression(expression)?;
                    if !item_is_numeric(&item) {
                        self.unsupported(*span, "non-numeric arithmetic operand");
                        None
                    } else {
                        Some(format!("cob_load({field})"))
                    }
                }
            },
            Expression::Subscript { span, .. } => {
                let (field, item) = self.field_expression(expression)?;
                if !item_is_numeric(&item) {
                    self.unsupported(*span, "non-numeric arithmetic operand");
                    None
                } else {
                    Some(format!("cob_load({field})"))
                }
            }
            Expression::UnaryOp { op, operand, .. } => {
                let operand = self.numeric_expression(operand, None)?;
                Some(match op {
                    UnaryOperator::Plus => operand,
                    UnaryOperator::Minus => format!("cob_num_negate({operand})"),
                })
            }
            Expression::BinaryOp {
                left,
                op,
                right,
                span,
            } => {
                let left = self.numeric_expression(left, None)?;
                let right = self.numeric_expression(right, None)?;
                let function = match op {
                    BinaryOperator::Add => "cob_num_add",
                    BinaryOperator::Subtract => "cob_num_subtract",
                    BinaryOperator::Multiply => "cob_num_multiply",
                    BinaryOperator::Divide => "cob_num_divide",
                    BinaryOperator::Power => {
                        let Some(scale) = power_scale else {
                            self.unsupported(*span, "nested exponentiation");
                            return None;
                        };
                        return Some(format!("cob_num_power({left}, {right}, {scale})"));
                    }
                    _ => {
                        self.unsupported(*span, "relational arithmetic expression");
                        return None;
                    }
                };
                Some(format!("{function}({left}, {right})"))
            }
            _ => {
                self.unsupported(expression_span(expression), "numeric expression");
                None
            }
        }
    }

    fn text_expression(&mut self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::StringLiteral(value, _) => {
                Some(format!("cob_text({}, {})", c_string(value), value.len()))
            }
            Expression::Identifier(_, _) | Expression::Subscript { .. } => {
                let (field, _) = self.field_expression(expression)?;
                Some(format!("cob_field_text({field})"))
            }
            _ => {
                self.unsupported(expression_span(expression), "text expression");
                None
            }
        }
    }

    fn emit_fill(&mut self, field: &str, numeric: bool, value: &str, span: Span, indent: usize) {
        if numeric && is_zero(value) {
            self.line(
                indent,
                &format!("(void)cob_store({field}, cob_num_integer(0), 0);"),
            );
            return;
        }
        let byte = match value {
            "ZERO" | "ZEROS" | "ZEROES" => "'0'",
            "SPACE" | "SPACES" => "' '",
            "LOW-VALUE" | "LOW-VALUES" | "NULL" | "NULLS" => "0",
            "HIGH-VALUE" | "HIGH-VALUES" => "UCHAR_MAX",
            "QUOTE" | "QUOTES" => "'\"'",
            _ => {
                self.unsupported(span, "figurative constant");
                return;
            }
        };
        if numeric {
            self.unsupported(span, "figurative constant for numeric item");
        } else {
            self.line(indent, &format!("cob_fill({field}, {byte});"));
        }
    }

    fn emit_statement(&mut self, statement: &Statement, indent: usize) {
        match statement {
            Statement::Display(display) => {
                for item in &display.items {
                    match item {
                        DisplayItem::String(value, _)
                        | DisplayItem::Expression(Expression::StringLiteral(value, _)) => {
                            self.line(
                                indent,
                                &format!("cob_display({}, {});", c_string(value), value.len()),
                            );
                        }
                        DisplayItem::Expression(expression) => {
                            self.emit_display(expression, indent)
                        }
                    }
                }
                if !display.no_advancing {
                    self.line(indent, "cob_display(\"\\n\", 1);");
                }
            }
            Statement::Move(value) => self.emit_move(&value.source, &value.targets, indent),
            Statement::Accept(value) => {
                if let Some((field, _)) = self.field_expression(&value.target) {
                    self.line(indent, &format!("cob_accept({field});"));
                }
            }
            Statement::Compute(value) => {
                if let Some((field, item)) = self.field_expression(&value.target) {
                    if !item_is_numeric(&item) {
                        self.unsupported(value.span, "non-numeric COMPUTE destination");
                    } else if let Some(expression) = self.numeric_expression(
                        &value.expression,
                        item.picture.as_ref().map(|picture| picture.scale),
                    ) {
                        self.line(
                            indent,
                            &format!("(void)cob_store({field}, {expression}, 0);"),
                        );
                    }
                }
            }
            Statement::Add(value) => self.emit_add(value, indent),
            Statement::Subtract(value) => self.emit_subtract(value, indent),
            Statement::Multiply(value) => self.emit_multiply(value, indent),
            Statement::Divide(value) => self.emit_divide(value, indent),
            Statement::If(value) => {
                if let Some(condition) = self.condition(&value.condition) {
                    self.line(indent, &format!("if ({condition}) {{"));
                    self.emit_block(&value.then_statements, indent + 1);
                    if let Some(otherwise) = &value.else_statements {
                        self.line(indent, "} else {");
                        self.emit_block(otherwise, indent + 1);
                    }
                    self.line(indent, "}");
                }
            }
            Statement::Perform(value) => self.emit_perform(value, indent),
            Statement::Set(value) => self.emit_set(value, indent),
            Statement::Search(value) => self.emit_search(value, indent),
            Statement::Inspect(value) => self.emit_inspect(value, indent),
            Statement::Evaluate(value) => self.emit_evaluate(value, indent),
            Statement::Open(value) => self.emit_open(value, indent),
            Statement::Close(value) => {
                for (name, span) in &value.files {
                    if let Some(file) = self.file(name, *span) {
                        let temporary = self.temporary();
                        self.line(indent, "{");
                        self.line(
                            indent + 1,
                            &format!(
                                "int cob_status_{temporary} = cob_file_close(&cob_file_{});",
                                file.index
                            ),
                        );
                        self.line(
                            indent + 1,
                            &format!(
                                "cob_file_check(&cob_file_{}, cob_status_{temporary}, 0);",
                                file.index
                            ),
                        );
                        self.line(indent, "}");
                    }
                }
            }
            Statement::Read(value) => self.emit_read(value, indent),
            Statement::Write(value) => self.emit_write(value, indent),
            Statement::Start(value) => self.emit_start(value, indent),
            Statement::StopRun(_) => self.line(indent, "exit(EXIT_SUCCESS);"),
            _ => self.unsupported(statement_span(statement), "statement"),
        }
    }

    fn emit_block(&mut self, statements: &[Statement], indent: usize) {
        for statement in statements {
            self.emit_statement(statement, indent);
        }
    }

    fn paragraph_index(&mut self, name: &str, span: Span) -> Option<usize> {
        let index = self
            .program
            .program
            .procedure_division
            .as_ref()
            .and_then(|division| {
                division
                    .paragraphs
                    .iter()
                    .position(|paragraph| paragraph.name.eq_ignore_ascii_case(name))
            });
        if index.is_none() {
            self.unsupported(span, "unknown paragraph");
        }
        index
    }

    fn emit_perform(&mut self, perform: &PerformStatement, indent: usize) {
        match perform {
            PerformStatement::ProcedureName(name, span) => {
                if let Some(index) = self.paragraph_index(name, *span) {
                    self.line(indent, &format!("cob_paragraph_{index}();"));
                }
            }
            PerformStatement::Until {
                condition,
                paragraph,
                test_after,
                span,
            } => {
                if let (Some(index), Some(condition)) = (
                    self.paragraph_index(paragraph, *span),
                    self.condition(condition),
                ) {
                    if *test_after {
                        self.line(indent, "do {");
                        self.line(indent + 1, &format!("cob_paragraph_{index}();"));
                        self.line(indent, &format!("}} while (!({condition}));"));
                    } else {
                        self.line(indent, &format!("while (!({condition})) {{"));
                        self.line(indent + 1, &format!("cob_paragraph_{index}();"));
                        self.line(indent, "}");
                    }
                }
            }
            _ => self.unsupported(perform_span(perform), "PERFORM form"),
        }
    }

    fn condition(&mut self, condition: &Condition) -> Option<String> {
        match condition {
            Condition::Relation {
                left,
                op,
                right,
                span,
            } => {
                let left_numeric = self.expression_is_numeric(left)?;
                let right_numeric = self.expression_is_numeric(right)?;
                let comparison = if left_numeric && right_numeric {
                    format!(
                        "cob_num_compare({}, {})",
                        self.numeric_expression(left, None)?,
                        self.numeric_expression(right, None)?
                    )
                } else if !left_numeric && !right_numeric {
                    format!(
                        "cob_text_compare({}, {})",
                        self.text_expression(left)?,
                        self.text_expression(right)?
                    )
                } else {
                    self.unsupported(*span, "comparison between numeric and text values");
                    return None;
                };
                let operator = match op {
                    BinaryOperator::Equal => "==",
                    BinaryOperator::NotEqual => "!=",
                    BinaryOperator::LessThan => "<",
                    BinaryOperator::LessThanOrEqual => "<=",
                    BinaryOperator::GreaterThan => ">",
                    BinaryOperator::GreaterThanOrEqual => ">=",
                    _ => {
                        self.unsupported(*span, "condition operator");
                        return None;
                    }
                };
                Some(format!("({comparison} {operator} 0)"))
            }
            Condition::And { left, right, .. } => Some(format!(
                "({} && {})",
                self.condition(left)?,
                self.condition(right)?
            )),
            Condition::Or { left, right, .. } => Some(format!(
                "({} || {})",
                self.condition(left)?,
                self.condition(right)?
            )),
            Condition::Not { condition, .. } => Some(format!("!({})", self.condition(condition)?)),
        }
    }

    fn sum_expressions(&mut self, operands: &[Expression]) -> Option<String> {
        let mut sum = "cob_num_integer(0)".to_string();
        for operand in operands {
            sum = format!(
                "cob_num_add({sum}, {})",
                self.numeric_expression(operand, None)?
            );
        }
        Some(sum)
    }

    fn arithmetic_store(
        &mut self,
        target: &Expression,
        value: &str,
        size_error: Option<usize>,
        indent: usize,
    ) {
        let Some((field, item)) = self.field_expression(target) else {
            return;
        };
        if !item_is_numeric(&item) {
            self.unsupported(
                expression_span(target),
                "non-numeric arithmetic destination",
            );
            return;
        }
        if let Some(id) = size_error {
            self.line(
                indent,
                &format!("cob_size_{id} |= cob_store({field}, {value}, 1);"),
            );
        } else {
            self.line(indent, &format!("(void)cob_store({field}, {value}, 0);"));
        }
    }

    fn begin_arithmetic(&mut self, handler: &[Statement], indent: usize) -> Option<usize> {
        self.line(indent, "{");
        if handler.is_empty() {
            None
        } else {
            let id = self.temporary();
            self.line(indent + 1, &format!("int cob_size_{id} = 0;"));
            Some(id)
        }
    }

    fn end_arithmetic(&mut self, handler: &[Statement], size_error: Option<usize>, indent: usize) {
        if let Some(id) = size_error {
            self.line(indent + 1, &format!("if (cob_size_{id}) {{"));
            self.emit_block(handler, indent + 2);
            self.line(indent + 1, "}");
        }
        self.line(indent, "}");
    }

    fn emit_add(&mut self, add: &AddStatement, indent: usize) {
        let Some(sum) = self.sum_expressions(&add.operands) else {
            return;
        };
        let value = self.temporary();
        let size_error = self.begin_arithmetic(&add.on_size_error, indent);
        self.line(indent + 1, &format!("CobNum cob_value_{value} = {sum};"));
        if add.giving {
            for target in &add.targets {
                self.arithmetic_store(
                    target,
                    &format!("cob_value_{value}"),
                    size_error,
                    indent + 1,
                );
            }
        } else if !add.giving_targets.is_empty() {
            if add.targets.len() != 1 {
                self.unsupported(add.span, "ADD TO GIVING with multiple source targets");
            } else if let Some(base) = self.numeric_expression(&add.targets[0], None) {
                self.line(
                    indent + 1,
                    &format!("cob_value_{value} = cob_num_add(cob_value_{value}, {base});"),
                );
                for target in &add.giving_targets {
                    self.arithmetic_store(
                        target,
                        &format!("cob_value_{value}"),
                        size_error,
                        indent + 1,
                    );
                }
            }
        } else {
            for target in &add.targets {
                if let Some(base) = self.numeric_expression(target, None) {
                    self.arithmetic_store(
                        target,
                        &format!("cob_num_add({base}, cob_value_{value})"),
                        size_error,
                        indent + 1,
                    );
                }
            }
        }
        self.end_arithmetic(&add.on_size_error, size_error, indent);
    }

    fn emit_subtract(&mut self, subtract: &SubtractStatement, indent: usize) {
        let Some(sum) = self.sum_expressions(&subtract.operands) else {
            return;
        };
        let value = self.temporary();
        let size_error = self.begin_arithmetic(&subtract.on_size_error, indent);
        self.line(indent + 1, &format!("CobNum cob_value_{value} = {sum};"));
        if subtract.giving.is_empty() {
            for target in &subtract.from {
                if let Some(base) = self.numeric_expression(target, None) {
                    self.arithmetic_store(
                        target,
                        &format!("cob_num_subtract({base}, cob_value_{value})"),
                        size_error,
                        indent + 1,
                    );
                }
            }
        } else if subtract.from.len() != 1 {
            self.unsupported(
                subtract.span,
                "SUBTRACT GIVING with multiple source targets",
            );
        } else if let Some(base) = self.numeric_expression(&subtract.from[0], None) {
            self.line(
                indent + 1,
                &format!("cob_value_{value} = cob_num_subtract({base}, cob_value_{value});"),
            );
            for target in &subtract.giving {
                self.arithmetic_store(
                    target,
                    &format!("cob_value_{value}"),
                    size_error,
                    indent + 1,
                );
            }
        }
        self.end_arithmetic(&subtract.on_size_error, size_error, indent);
    }

    fn emit_multiply(&mut self, multiply: &MultiplyStatement, indent: usize) {
        let Some(factor) = self.numeric_expression(&multiply.factor, None) else {
            return;
        };
        let value = self.temporary();
        let size_error = self.begin_arithmetic(&multiply.on_size_error, indent);
        self.line(indent + 1, &format!("CobNum cob_value_{value} = {factor};"));
        if multiply.giving.is_empty() {
            for target in &multiply.by {
                if let Some(base) = self.numeric_expression(target, None) {
                    self.arithmetic_store(
                        target,
                        &format!("cob_num_multiply({base}, cob_value_{value})"),
                        size_error,
                        indent + 1,
                    );
                }
            }
        } else if multiply.by.len() != 1 {
            self.unsupported(
                multiply.span,
                "MULTIPLY GIVING with multiple source targets",
            );
        } else if let Some(base) = self.numeric_expression(&multiply.by[0], None) {
            self.line(
                indent + 1,
                &format!("cob_value_{value} = cob_num_multiply({base}, cob_value_{value});"),
            );
            for target in &multiply.giving {
                self.arithmetic_store(
                    target,
                    &format!("cob_value_{value}"),
                    size_error,
                    indent + 1,
                );
            }
        }
        self.end_arithmetic(&multiply.on_size_error, size_error, indent);
    }

    fn emit_divide(&mut self, divide: &DivideStatement, indent: usize) {
        if divide.giving.is_empty() {
            self.unsupported(divide.span, "DIVIDE without GIVING");
            return;
        }
        let (Some(dividend), Some(divisor)) = (
            self.numeric_expression(&divide.dividend, None),
            self.numeric_expression(&divide.divisor, None),
        ) else {
            return;
        };
        let value = self.temporary();
        let size_error = self.begin_arithmetic(&divide.on_size_error, indent);
        self.line(
            indent + 1,
            &format!("CobNum cob_dividend_{value} = {dividend};"),
        );
        self.line(
            indent + 1,
            &format!("CobNum cob_divisor_{value} = {divisor};"),
        );
        self.line(
            indent + 1,
            &format!("if (cob_divisor_{value}.sign == 0) {{"),
        );
        if let Some(id) = size_error {
            self.line(indent + 2, &format!("cob_size_{id} = 1;"));
        } else {
            self.line(indent + 2, "cob_fail(\"division by zero\");");
        }
        self.line(indent + 1, "} else {");
        self.line(
            indent + 2,
            &format!("CobNum cob_quotient_{value} = cob_num_divide(cob_dividend_{value}, cob_divisor_{value});"),
        );
        for target in &divide.giving {
            self.arithmetic_store(
                target,
                &format!("cob_quotient_{value}"),
                size_error,
                indent + 2,
            );
        }
        if let Some(remainder) = &divide.remainder {
            if divide.giving.len() != 1 {
                self.unsupported(divide.span, "DIVIDE REMAINDER with multiple quotients");
            } else if let Some(quotient) = self.numeric_expression(&divide.giving[0], None) {
                if let Some(id) = size_error {
                    self.line(indent + 2, &format!("if (!cob_size_{id}) {{"));
                }
                let remainder_indent = indent + 2 + usize::from(size_error.is_some());
                self.arithmetic_store(
                    remainder,
                    &format!("cob_num_subtract(cob_dividend_{value}, cob_num_multiply(cob_divisor_{value}, {quotient}))"),
                    size_error,
                    remainder_indent,
                );
                if size_error.is_some() {
                    self.line(indent + 2, "}");
                }
            }
        }
        self.line(indent + 1, "}");
        self.end_arithmetic(&divide.on_size_error, size_error, indent);
    }

    fn emit_set(&mut self, set: &SetStatement, indent: usize) {
        let Some(value) = self.numeric_expression(&set.value, None) else {
            return;
        };
        let temporary = self.temporary();
        self.line(indent, "{");
        self.line(
            indent + 1,
            &format!("long long cob_set_{temporary} = cob_num_to_integer({value});"),
        );
        for target in &set.targets {
            let Expression::Identifier(name, span) = target else {
                self.unsupported(expression_span(target), "SET target");
                continue;
            };
            match self.resolve(name, *span) {
                Some(Binding::Index(index)) => {
                    let value = match set.kind {
                        SetKind::To => format!("cob_set_{temporary}"),
                        SetKind::UpBy => {
                            format!("cob_index_add(cob_index_{index}, cob_set_{temporary})")
                        }
                        SetKind::DownBy => {
                            format!("cob_index_subtract(cob_index_{index}, cob_set_{temporary})")
                        }
                    };
                    self.line(indent + 1, &format!("cob_index_{index} = {value};"));
                }
                Some(Binding::Item(_)) => self.unsupported(*span, "SET data item"),
                None => {}
            }
        }
        self.line(indent, "}");
    }

    fn emit_search(&mut self, search: &SearchStatement, indent: usize) {
        if search.all || search.varying.is_some() {
            self.unsupported(search.span, "SEARCH ALL or SEARCH VARYING");
            return;
        }
        let Expression::Identifier(name, span) = &search.table else {
            self.unsupported(expression_span(&search.table), "SEARCH table");
            return;
        };
        let Some(Binding::Item(table_index)) = self.resolve(name, *span) else {
            return;
        };
        let table = self.items[table_index].clone();
        let Some((index, index_info)) = self
            .indexes
            .iter()
            .enumerate()
            .find(|(_, index)| index.table == table.declaration.span)
            .map(|(index, info)| (index, info.clone()))
        else {
            self.unsupported(search.span, "SEARCH table without an index");
            return;
        };
        if table.dimensions.len() != 1 {
            self.unsupported(search.span, "nested SEARCH table");
            return;
        }
        let temporary = self.temporary();
        self.line(indent, "{");
        self.line(indent + 1, &format!("int cob_found_{temporary} = 0;"));
        self.line(
            indent + 1,
            &format!("if (cob_index_{index} < 1) cob_fail(\"invalid SEARCH index\");"),
        );
        self.line(
            indent + 1,
            &format!(
                "for (; (unsigned long long)cob_index_{index} <= {}ULL; ++cob_index_{index}) {{",
                index_info.count
            ),
        );
        for when in &search.whens {
            if let Some(condition) = self.condition(&when.condition) {
                self.line(indent + 2, &format!("if ({condition}) {{"));
                self.line(indent + 3, &format!("cob_found_{temporary} = 1;"));
                self.emit_block(&when.statements, indent + 3);
                self.line(indent + 3, "break;");
                self.line(indent + 2, "}");
            }
        }
        self.line(indent + 1, "}");
        if let Some(at_end) = &search.at_end {
            self.line(indent + 1, &format!("if (!cob_found_{temporary}) {{"));
            self.emit_block(at_end, indent + 2);
            self.line(indent + 1, "}");
        } else {
            self.line(indent + 1, &format!("(void)cob_found_{temporary};"));
        }
        self.line(indent, "}");
    }

    fn emit_evaluate(&mut self, evaluate: &EvaluateStatement, indent: usize) {
        let Some(numeric) = self.expression_is_numeric(&evaluate.subject) else {
            return;
        };
        let subject = if numeric {
            self.numeric_expression(&evaluate.subject, None)
        } else {
            self.text_expression(&evaluate.subject)
        };
        let Some(subject) = subject else {
            return;
        };
        let temporary = self.temporary();
        self.line(indent, "{");
        self.line(
            indent + 1,
            &format!(
                "{} cob_subject_{temporary} = {subject};",
                if numeric { "CobNum" } else { "CobText" }
            ),
        );
        self.line(indent + 1, &format!("(void)cob_subject_{temporary};"));
        let mut first = true;
        for when in &evaluate.whens {
            if when.other {
                if first {
                    self.line(indent + 1, "if (1) {");
                } else {
                    self.line(indent + 1, "else {");
                }
            } else {
                let mut choices = Vec::new();
                for choice in &when.choices {
                    if let Some(choice) = self.evaluate_choice(choice, numeric, temporary) {
                        choices.push(choice);
                    }
                }
                if choices.is_empty() {
                    self.unsupported(when.span, "empty EVALUATE WHEN");
                    continue;
                }
                self.line(
                    indent + 1,
                    &format!(
                        "{}if ({}) {{",
                        if first { "" } else { "else " },
                        choices.join(" || ")
                    ),
                );
            }
            self.emit_block(&when.statements, indent + 2);
            self.line(indent + 1, "}");
            first = false;
        }
        self.line(indent, "}");
    }

    fn evaluate_choice(
        &mut self,
        choice: &EvaluateChoice,
        numeric: bool,
        subject: usize,
    ) -> Option<String> {
        let compare = |value: &str| {
            format!(
                "{}(cob_subject_{subject}, {value})",
                if numeric {
                    "cob_num_compare"
                } else {
                    "cob_text_compare"
                }
            )
        };
        match choice {
            EvaluateChoice::Value(value) => {
                let actual = self.expression_is_numeric(value)?;
                if actual != numeric {
                    self.unsupported(expression_span(value), "EVALUATE choice category");
                    return None;
                }
                let value = if numeric {
                    self.numeric_expression(value, None)?
                } else {
                    self.text_expression(value)?
                };
                Some(format!("({} == 0)", compare(&value)))
            }
            EvaluateChoice::Range(first, last) => {
                let first = if numeric {
                    self.numeric_expression(first, None)?
                } else {
                    self.text_expression(first)?
                };
                let last = if numeric {
                    self.numeric_expression(last, None)?
                } else {
                    self.text_expression(last)?
                };
                Some(format!(
                    "({} >= 0 && {} <= 0)",
                    compare(&first),
                    compare(&last)
                ))
            }
            EvaluateChoice::Any => Some("1".to_string()),
            EvaluateChoice::Condition(condition) => self.condition(condition),
        }
    }

    fn inspect_region(&mut self, region: &InspectRegion) -> Option<String> {
        let after = match &region.after {
            Some(value) => format!("1, {}", self.text_expression(value)?),
            None => "0, cob_text(\"\", 0)".to_string(),
        };
        let before = match &region.before {
            Some(value) => format!("1, {}", self.text_expression(value)?),
            None => "0, cob_text(\"\", 0)".to_string(),
        };
        Some(format!("{after}, {before}"))
    }

    fn emit_inspect(&mut self, inspect: &InspectStatement, indent: usize) {
        let Some((target, item)) = self.field_expression(&inspect.target) else {
            return;
        };
        if item_is_numeric(&item) {
            self.unsupported(inspect.span, "INSPECT on a numeric item");
            return;
        }
        for tally in &inspect.tallying {
            let Some((counter, counter_item)) = self.field_expression(&tally.counter) else {
                continue;
            };
            if !item_is_numeric(&counter_item) {
                self.unsupported(expression_span(&tally.counter), "INSPECT counter");
                continue;
            }
            let pattern = match &tally.pattern {
                Some(value) => self.text_expression(value),
                None => Some("cob_text(\"\", 0)".to_string()),
            };
            if let (Some(pattern), Some(region)) = (pattern, self.inspect_region(&tally.region)) {
                self.line(
                    indent,
                    &format!(
                        "cob_inspect_tally({target}, {counter}, {}, {pattern}, {region});",
                        inspect_mode(tally.mode)
                    ),
                );
            }
        }
        for replacement in &inspect.replacing {
            let pattern = match &replacement.pattern {
                Some(value) => self.text_expression(value),
                None => Some("cob_text(\"\", 0)".to_string()),
            };
            let by = self.text_expression(&replacement.by);
            if let (Some(pattern), Some(by), Some(region)) =
                (pattern, by, self.inspect_region(&replacement.region))
            {
                self.line(
                    indent,
                    &format!(
                        "cob_inspect_replace({target}, {}, {pattern}, {by}, {region});",
                        inspect_mode(replacement.mode)
                    ),
                );
            }
        }
        if let Some(convert) = &inspect.converting {
            let from = self.text_expression(&convert.from);
            let to = self.text_expression(&convert.to);
            if let (Some(from), Some(to), Some(region)) =
                (from, to, self.inspect_region(&convert.region))
            {
                self.line(
                    indent,
                    &format!("cob_inspect_convert({target}, {from}, {to}, {region});"),
                );
            }
        }
    }

    fn file(&mut self, name: &str, span: Span) -> Option<File> {
        let file = self
            .files
            .iter()
            .find(|file| file.control.file_name.eq_ignore_ascii_case(name))
            .cloned();
        if file.is_none() {
            self.unsupported(span, "unknown file");
        }
        file
    }

    fn file_key(&mut self, file: &File, expression: &Expression) -> Option<usize> {
        let Expression::Identifier(name, span) = expression else {
            self.unsupported(expression_span(expression), "file key expression");
            return None;
        };
        let Some(Binding::Item(item_index)) = self.resolve(name, *span) else {
            return None;
        };
        let key = file
            .keys
            .iter()
            .position(|key| key.declaration.span == self.items[item_index].declaration.span);
        if key.is_none() {
            self.unsupported(*span, "undeclared file key");
        }
        key
    }

    fn emit_open(&mut self, open: &OpenStatement, indent: usize) {
        for group in &open.groups {
            let mode = match group.mode.as_str() {
                "INPUT" => "COB_FILE_INPUT",
                "OUTPUT" => "COB_FILE_OUTPUT",
                "I-O" => "COB_FILE_IO",
                "EXTEND" => "COB_FILE_EXTEND",
                _ => {
                    self.unsupported(open.span, "OPEN mode");
                    continue;
                }
            };
            for (name, span) in &group.files {
                let Some(file) = self.file(name, *span) else {
                    continue;
                };
                if file.indexed && mode == "COB_FILE_EXTEND" {
                    self.unsupported(*span, "OPEN EXTEND for an indexed file");
                    continue;
                }
                let Some(assignment) = &file.control.assign_to else {
                    self.unsupported(*span, "file without ASSIGN");
                    continue;
                };
                let Some(assignment) = self.text_expression(assignment) else {
                    continue;
                };
                let temporary = self.temporary();
                self.line(indent, "{");
                self.line(
                    indent + 1,
                    &format!(
                        "int cob_status_{temporary} = cob_file_open(&cob_file_{}, {assignment}, {mode});",
                        file.index
                    ),
                );
                self.line(
                    indent + 1,
                    &format!(
                        "cob_file_check(&cob_file_{}, cob_status_{temporary}, 0);",
                        file.index
                    ),
                );
                self.line(indent, "}");
            }
        }
    }

    fn emit_io_handlers(
        &mut self,
        file: &File,
        handlers: &IoHandlers,
        status: usize,
        indent: usize,
    ) {
        let at_end = handlers.at_end.is_some();
        let invalid = handlers.invalid_key.is_some();
        if let Some(statements) = &handlers.at_end {
            self.line(indent, &format!("if (cob_status_{status} == 10) {{"));
            self.emit_block(statements, indent + 1);
            self.line(indent, "}");
        }
        if let Some(statements) = &handlers.invalid_key {
            self.line(
                indent,
                &format!("if (cob_status_{status} == 21 || cob_status_{status} == 22 || cob_status_{status} == 23) {{"),
            );
            self.emit_block(statements, indent + 1);
            self.line(indent, "}");
        }
        self.line(
            indent,
            &format!(
                "cob_file_check(&cob_file_{}, cob_status_{status}, ({} && cob_status_{status} == 10) || ({} && (cob_status_{status} == 21 || cob_status_{status} == 22 || cob_status_{status} == 23)));",
                file.index,
                usize::from(at_end),
                usize::from(invalid)
            ),
        );
    }

    fn emit_read(&mut self, read: &ReadStatement, indent: usize) {
        let Some(file) = self.file(&read.file.0, read.file.1) else {
            return;
        };
        if file.indexed && read.key.is_none() {
            self.unsupported(read.span, "sequential READ of an indexed file");
            return;
        }
        if !file.indexed && read.key.is_some() {
            self.unsupported(read.span, "keyed READ of a sequential file");
            return;
        }
        if (read.key.is_some()
            && (read.handlers.at_end.is_some() || read.handlers.not_at_end.is_some()))
            || (read.key.is_none()
                && (read.handlers.invalid_key.is_some() || read.handlers.not_invalid_key.is_some()))
        {
            self.unsupported(read.span, "READ handler for the wrong access form");
            return;
        }
        let key = match &read.key {
            Some(expression) => match self.file_key(&file, expression) {
                Some(key) => key.to_string(),
                None => return,
            },
            None => "-1".to_string(),
        };
        let status = self.temporary();
        self.line(indent, "{");
        self.line(
            indent + 1,
            &format!(
                "int cob_status_{status} = cob_file_read(&cob_file_{}, {key});",
                file.index
            ),
        );
        self.line(indent + 1, &format!("if (cob_status_{status} == 0) {{"));
        if let Some(into) = &read.into {
            let source = Expression::Identifier(
                file.record.declaration.name.clone(),
                file.record.declaration.span,
            );
            self.emit_move(&source, std::slice::from_ref(into), indent + 2);
        }
        if let Some(statements) = &read.handlers.not_at_end {
            self.emit_block(statements, indent + 2);
        }
        if let Some(statements) = &read.handlers.not_invalid_key {
            self.emit_block(statements, indent + 2);
        }
        self.line(indent + 1, "}");
        self.emit_io_handlers(&file, &read.handlers, status, indent + 1);
        self.line(indent, "}");
    }

    fn emit_write(&mut self, write: &WriteStatement, indent: usize) {
        if write.advancing.is_some() {
            self.unsupported(write.span, "WRITE ADVANCING");
            return;
        }
        if write.handlers.at_end.is_some() || write.handlers.not_at_end.is_some() {
            self.unsupported(write.span, "WRITE AT END handler");
            return;
        }
        let Some((_, record)) = self.field_expression(&write.record) else {
            return;
        };
        let Some(file_index) = record.file else {
            self.unsupported(write.span, "WRITE of working-storage item");
            return;
        };
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.index == file_index)
            .cloned()
        else {
            self.unsupported(write.span, "WRITE without file layout");
            return;
        };
        if record.declaration.span != file.record.declaration.span {
            self.unsupported(write.span, "WRITE of a non-record item");
            return;
        }
        if let Some(source) = &write.from {
            self.emit_move(source, std::slice::from_ref(&write.record), indent);
        }
        let status = self.temporary();
        self.line(indent, "{");
        self.line(
            indent + 1,
            &format!(
                "int cob_status_{status} = cob_file_write(&cob_file_{});",
                file.index
            ),
        );
        if let Some(statements) = &write.handlers.not_invalid_key {
            self.line(indent + 1, &format!("if (cob_status_{status} == 0) {{"));
            self.emit_block(statements, indent + 2);
            self.line(indent + 1, "}");
        }
        self.emit_io_handlers(&file, &write.handlers, status, indent + 1);
        self.line(indent, "}");
    }

    fn emit_start(&mut self, start: &StartStatement, indent: usize) {
        if start.handlers.at_end.is_some() || start.handlers.not_at_end.is_some() {
            self.unsupported(start.span, "START AT END handler");
            return;
        }
        let Some(file) = self.file(&start.file.0, start.file.1) else {
            return;
        };
        if !file.indexed || file.control.access_mode.as_deref() != Some("DYNAMIC") {
            self.unsupported(start.span, "START outside dynamic indexed access");
            return;
        }
        let Some((operator, key)) = &start.key else {
            self.unsupported(start.span, "START without a key");
            return;
        };
        if *operator != BinaryOperator::Equal {
            self.unsupported(start.span, "START relation other than EQUAL");
            return;
        }
        let Some(key) = self.file_key(&file, key) else {
            return;
        };
        let status = self.temporary();
        self.line(indent, "{");
        self.line(
            indent + 1,
            &format!(
                "int cob_status_{status} = cob_file_start(&cob_file_{}, {key});",
                file.index
            ),
        );
        if let Some(statements) = &start.handlers.not_invalid_key {
            self.line(indent + 1, &format!("if (cob_status_{status} == 0) {{"));
            self.emit_block(statements, indent + 2);
            self.line(indent + 1, "}");
        }
        self.emit_io_handlers(&file, &start.handlers, status, indent + 1);
        self.line(indent, "}");
    }

    fn emit_display(&mut self, expression: &Expression, indent: usize) {
        match expression {
            Expression::NumericLiteral(value, _) => self.line(
                indent,
                &format!("cob_display({}, {});", c_string(value), value.len()),
            ),
            Expression::Identifier(name, span) => match self.resolve(name, *span) {
                Some(Binding::Index(index)) => {
                    self.line(indent, &format!("cob_display_index(cob_index_{index});"));
                }
                Some(Binding::Item(_)) => {
                    if let Some((field, _)) = self.field_expression(expression) {
                        self.line(indent, &format!("cob_display_field({field});"));
                    }
                }
                None => {}
            },
            Expression::Subscript { .. } => {
                if let Some((field, _)) = self.field_expression(expression) {
                    self.line(indent, &format!("cob_display_field({field});"));
                }
            }
            Expression::FigurativeConstant(value, span) => {
                let text = match value.as_str() {
                    "SPACE" | "SPACES" => " ",
                    "ZERO" | "ZEROS" | "ZEROES" => "0",
                    "LOW-VALUE" | "LOW-VALUES" | "NULL" | "NULLS" => "\0",
                    "QUOTE" | "QUOTES" => "\"",
                    _ => {
                        self.unsupported(*span, "DISPLAY figurative constant");
                        return;
                    }
                };
                self.line(indent, &format!("cob_display({}, 1);", c_string(text)));
            }
            _ => self.unsupported(expression_span(expression), "DISPLAY operand"),
        }
    }

    fn emit_move(&mut self, source: &Expression, targets: &[Expression], indent: usize) {
        if let Expression::FigurativeConstant(value, span) = source {
            for target in targets {
                if let Some((field, item)) = self.field_expression(target) {
                    self.emit_fill(&field, item_is_numeric(&item), value, *span, indent);
                }
            }
            return;
        }
        let upper = match source {
            Expression::FunctionCall {
                name,
                arguments,
                span,
            } if name == "UPPER-CASE" && arguments.len() == 1 => Some(&arguments[0]),
            Expression::FunctionCall { span, .. } => {
                self.unsupported(*span, "intrinsic function");
                return;
            }
            _ => None,
        };
        let numeric = if upper.is_some() {
            false
        } else {
            match self.expression_is_numeric(source) {
                Some(value) => value,
                None => return,
            }
        };
        let temporary = self.temporary();
        self.line(indent, "{");
        if numeric {
            let Some(value) = self.numeric_expression(source, None) else {
                self.line(indent, "}");
                return;
            };
            self.line(
                indent + 1,
                &format!("CobNum cob_move_{temporary} = {value};"),
            );
            let copy_text = matches!(
                source,
                Expression::Identifier(..) | Expression::Subscript { .. }
            ) && targets
                .iter()
                .any(|target| self.expression_is_numeric(target) == Some(false));
            if copy_text && let Some(text) = self.text_expression(source) {
                self.line(
                    indent + 1,
                    &format!("CobText cob_text_{temporary} = {text};"),
                );
                self.line(
                    indent + 1,
                    &format!(
                        "unsigned char *cob_copy_{temporary} = cob_copy_text(cob_text_{temporary});"
                    ),
                );
                self.line(
                    indent + 1,
                    &format!("cob_text_{temporary} = cob_text(cob_copy_{temporary}, cob_text_{temporary}.length);"),
                );
            }
            for target in targets {
                if let Some((field, item)) = self.field_expression(target) {
                    if item_is_numeric(&item) {
                        self.line(
                            indent + 1,
                            &format!("(void)cob_store({field}, cob_move_{temporary}, 0);"),
                        );
                    } else if matches!(
                        source,
                        Expression::Identifier(..) | Expression::Subscript { .. }
                    ) {
                        self.line(
                            indent + 1,
                            &format!("cob_move_text({field}, cob_text_{temporary});"),
                        );
                    } else {
                        self.unsupported(expression_span(source), "numeric MOVE to text");
                    }
                }
            }
            if copy_text {
                self.line(indent + 1, &format!("free(cob_copy_{temporary});"));
            }
        } else {
            let Some(text) = self.text_expression(upper.unwrap_or(source)) else {
                self.line(indent, "}");
                return;
            };
            self.line(
                indent + 1,
                &format!("CobText cob_text_{temporary} = {text};"),
            );
            self.line(
                indent + 1,
                &format!(
                    "unsigned char *cob_copy_{temporary} = cob_copy_text(cob_text_{temporary});"
                ),
            );
            self.line(
                indent + 1,
                &format!("cob_text_{temporary} = cob_text(cob_copy_{temporary}, cob_text_{temporary}.length);"),
            );
            for target in targets {
                if let Some((field, item)) = self.field_expression(target) {
                    if item_is_numeric(&item) {
                        self.unsupported(expression_span(target), "text MOVE to numeric item");
                    } else {
                        let function = if upper.is_some() {
                            "cob_move_upper"
                        } else {
                            "cob_move_text"
                        };
                        self.line(
                            indent + 1,
                            &format!("{function}({field}, cob_text_{temporary});"),
                        );
                    }
                }
            }
            self.line(indent + 1, &format!("free(cob_copy_{temporary});"));
        }
        self.line(indent, "}");
    }

    fn expression_is_numeric(&mut self, expression: &Expression) -> Option<bool> {
        match expression {
            Expression::NumericLiteral(..)
            | Expression::BinaryOp { .. }
            | Expression::UnaryOp { .. } => Some(true),
            Expression::StringLiteral(..) | Expression::FunctionCall { .. } => Some(false),
            Expression::FigurativeConstant(value, _) if is_zero(value) => Some(true),
            Expression::Identifier(name, span) => match self.resolve(name, *span)? {
                Binding::Item(item) => Some(item_is_numeric(&self.items[item])),
                Binding::Index(_) => Some(true),
            },
            Expression::Subscript { .. } => {
                let (_, item) = self.field_expression(expression)?;
                Some(item_is_numeric(&item))
            }
            _ => {
                self.unsupported(expression_span(expression), "expression category");
                None
            }
        }
    }

    fn temporary(&mut self) -> usize {
        let result = self.next_temporary;
        self.next_temporary += 1;
        result
    }

    fn line(&mut self, indent: usize, text: &str) {
        writeln!(self.output, "{}{text}", "    ".repeat(indent)).unwrap();
    }

    fn unsupported(&mut self, span: Span, feature: &str) {
        self.diagnostics.push(
            Diagnostic::error(
                "cobol::codegen::unsupported",
                format!("C code generation does not support this {feature}"),
            )
            .with_span(span),
        );
    }
}

fn c_string(value: &str) -> String {
    let mut result = String::from("\"");
    for byte in value.bytes() {
        match byte {
            b'"' => result.push_str("\\\""),
            b'\\' => result.push_str("\\\\"),
            b'?' => result.push_str("\\?"),
            0x20..=0x7e => result.push(char::from(byte)),
            _ => write!(result, "\\{byte:03o}").unwrap(),
        }
    }
    result.push('"');
    result
}

fn is_zero(value: &str) -> bool {
    matches!(value, "ZERO" | "ZEROS" | "ZEROES")
}

fn inspect_mode(mode: InspectMode) -> &'static str {
    match mode {
        InspectMode::Characters => "COB_INSPECT_CHARACTERS",
        InspectMode::All => "COB_INSPECT_ALL",
        InspectMode::Leading => "COB_INSPECT_LEADING",
        InspectMode::First => "COB_INSPECT_FIRST",
    }
}

fn item_is_numeric(item: &Item) -> bool {
    item.picture
        .as_ref()
        .is_some_and(|picture| picture.data_type == DataType::Numeric)
}

fn expand_picture(picture: &str) -> Option<String> {
    let mut result = String::new();
    let mut chars = picture.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '(' {
            result.push(ch.to_ascii_uppercase());
            continue;
        }
        let previous = result.pop()?;
        let mut count = String::new();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            count.push(chars.next()?);
        }
        if chars.next()? != ')' {
            return None;
        }
        let count = count.parse::<usize>().ok()?;
        if count == 0 || count > 4096 || result.len().checked_add(count)? > 4096 {
            return None;
        }
        result.extend(std::iter::repeat_n(previous, count));
    }
    Some(result)
}

fn supported_edited_picture(picture: &str) -> bool {
    let mut sign = false;
    let mut decimal = false;
    let mut required_digit = false;
    let mut suppression = None;
    for (index, ch) in picture.chars().enumerate() {
        match ch {
            '9' => required_digit = true,
            'Z' | '*' if !decimal && !required_digit => {
                if suppression.is_some_and(|previous| previous != ch) {
                    return false;
                }
                suppression = Some(ch);
            }
            '.' | 'V' if !decimal => decimal = true,
            '+' | '-' if !sign && (index == 0 || index + 1 == picture.len()) => sign = true,
            _ => return false,
        }
    }
    true
}

fn expression_span(expression: &Expression) -> Span {
    match expression {
        Expression::Identifier(_, span)
        | Expression::NumericLiteral(_, span)
        | Expression::StringLiteral(_, span)
        | Expression::FigurativeConstant(_, span) => *span,
        Expression::Subscript { span, .. }
        | Expression::RefMod { span, .. }
        | Expression::FunctionCall { span, .. }
        | Expression::BinaryOp { span, .. }
        | Expression::UnaryOp { span, .. } => *span,
    }
}

fn statement_span(statement: &Statement) -> Span {
    match statement {
        Statement::Move(value) => value.span,
        Statement::Compute(value) => value.span,
        Statement::Display(value) => value.span,
        Statement::Accept(value) => value.span,
        Statement::Perform(value) => perform_span(value),
        Statement::If(value) => value.span,
        Statement::Add(value) => value.span,
        Statement::Subtract(value) => value.span,
        Statement::Multiply(value) => value.span,
        Statement::Divide(value) => value.span,
        Statement::StopRun(span) | Statement::Paragraph(_, span) => *span,
        Statement::GoTo(value) => value.span,
        Statement::Set(value) => value.span,
        Statement::Search(value) => value.span,
        Statement::Inspect(value) => value.span,
        Statement::Evaluate(value) => value.span,
        Statement::Open(value) => value.span,
        Statement::Close(value) => value.span,
        Statement::Read(value) => value.span,
        Statement::Write(value) => value.span,
        Statement::Start(value) => value.span,
        Statement::Initialize(value) => value.span,
    }
}

fn perform_span(perform: &PerformStatement) -> Span {
    match perform {
        PerformStatement::ProcedureName(_, span)
        | PerformStatement::Thru { span, .. }
        | PerformStatement::Times { span, .. }
        | PerformStatement::Until { span, .. } => *span,
        PerformStatement::Varying(value) => value.span,
        PerformStatement::Inline(value) => value.span,
    }
}

#[cfg(test)]
mod tests;
