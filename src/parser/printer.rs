//! Plain-text AST dump used by the CLI and the golden tests.
//! Every node is printed, including empty divisions and sections.

use super::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut output = String::new();
    output.push_str("Program\n");
    output.push_str("  IdentificationDivision\n");
    output.push_str(&format!(
        "    program_id: {}\n",
        program.identification_division.program_id
    ));

    if let Some(author) = &program.identification_division.author {
        output.push_str(&format!("    author: {}\n", author));
    }
    if let Some(installation) = &program.identification_division.installation {
        output.push_str(&format!("    installation: {}\n", installation));
    }
    if let Some(date_written) = &program.identification_division.date_written {
        output.push_str(&format!("    date_written: {}\n", date_written));
    }
    if let Some(date_compiled) = &program.identification_division.date_compiled {
        output.push_str(&format!("    date_compiled: {}\n", date_compiled));
    }
    if let Some(security) = &program.identification_division.security {
        output.push_str(&format!("    security: {}\n", security));
    }
    if let Some(remarks) = &program.identification_division.remarks {
        output.push_str(&format!("    remarks: {}\n", remarks));
    }

    if let Some(env_div) = &program.environment_division {
        output.push_str("  EnvironmentDivision\n");
        if env_div.configuration_section.is_some() {
            output.push_str("    ConfigurationSection\n");
        }
        if let Some(section) = &env_div.input_output_section {
            output.push_str("    InputOutputSection\n");
            for control in &section.file_controls {
                format_file_control(&mut output, control);
            }
        }
    }

    if let Some(data_div) = &program.data_division {
        output.push_str("  DataDivision\n");
        if let Some(file_sec) = &data_div.file_section {
            output.push_str("    FileSection\n");
            for file_desc in &file_sec.file_descriptions {
                output.push_str(&format!("      FileDescription: {}\n", file_desc.file_name));
                for clause in &file_desc.clauses {
                    match clause {
                        FileClause::BlockContains(size, _) => {
                            output.push_str(&format!("        BLOCK CONTAINS: {size}\n"));
                        }
                        FileClause::RecordContains(size, _) => {
                            output.push_str(&format!("        RECORD CONTAINS: {size}\n"));
                        }
                    }
                }
                format_data_items(&mut output, &file_desc.records, 8);
            }
        }
        if let Some(work_sec) = &data_div.working_storage_section {
            output.push_str("    WorkingStorageSection\n");
            format_data_items(&mut output, &work_sec.data_items, 6);
        }
    }

    if let Some(proc_div) = &program.procedure_division {
        output.push_str("  ProcedureDivision\n");
        for para in &proc_div.paragraphs {
            output.push_str(&format!("    Paragraph: {}\n", para.name));
            for (i, stmt) in para.statements.iter().enumerate() {
                format_statement(&mut output, stmt, 6, i);
            }
        }
    }

    output
}

fn format_file_control(output: &mut String, control: &FileControl) {
    output.push_str(&format!("      FileControl: {}\n", control.file_name));
    if let Some(assign_to) = &control.assign_to {
        output.push_str(&format!(
            "        assign: {}\n",
            format_expression(assign_to)
        ));
    }
    if let Some(organization) = &control.organization {
        output.push_str(&format!("        organization: {organization}\n"));
    }
    if let Some(access_mode) = &control.access_mode {
        output.push_str(&format!("        access mode: {access_mode}\n"));
    }
    if let Some(key) = &control.record_key {
        output.push_str(&format!("        record key: {}\n", format_expression(key)));
    }
    for key in &control.alternate_keys {
        output.push_str(&format!(
            "        alternate key: {}{}\n",
            format_expression(&key.expression),
            if key.duplicates {
                " WITH DUPLICATES"
            } else {
                ""
            }
        ));
    }
    if let Some(status) = &control.file_status {
        output.push_str(&format!(
            "        file status: {}\n",
            format_expression(status)
        ));
    }
}

pub(crate) fn format_data_items(output: &mut String, items: &[DataItem], indent: usize) {
    for item in items {
        let ind = " ".repeat(indent);
        output.push_str(&format!(
            "{}DataItem (level {}): {}\n",
            ind, item.level, item.name
        ));
        if let Some(pic) = &item.picture {
            output.push_str(&format!("{}  PIC: {}\n", ind, pic));
        }
        if let Some(value) = &item.value {
            let value = match value {
                Literal::Numeric(value, _) => value.clone(),
                Literal::String(value, _) => format!("\"{value}\""),
                Literal::FigurativeConstant(value, _) => value.clone(),
            };
            output.push_str(&format!("{}  VALUE: {}\n", ind, value));
        }
        if let Some(occurs) = &item.occurs {
            output.push_str(&format!("{}  OCCURS: {}\n", ind, occurs));
        }
        if !item.indexed_by.is_empty() {
            output.push_str(&format!(
                "{}  INDEXED BY: {}\n",
                ind,
                item.indexed_by.join(", ")
            ));
        }
        if !item.children.is_empty() {
            format_data_items(output, &item.children, indent + 2);
        }
    }
}

pub(crate) fn format_statement(output: &mut String, stmt: &Statement, indent: usize, index: usize) {
    let ind = " ".repeat(indent);

    match stmt {
        Statement::Display(display_stmt) => {
            output.push_str(&format!("{}[{}] DISPLAY\n", ind, index));
            for item in &display_stmt.items {
                match item {
                    DisplayItem::String(s, _) => {
                        output.push_str(&format!("{}      \"{}\"\n", ind, s));
                    }
                    DisplayItem::Expression(expr) => {
                        output.push_str(&format!("{}      {}\n", ind, format_expression(expr)));
                    }
                }
            }
            if display_stmt.no_advancing {
                output.push_str(&format!("{}      WITH NO ADVANCING\n", ind));
            }
        }
        Statement::Move(move_stmt) => {
            output.push_str(&format!("{}[{}] MOVE\n", ind, index));
            output.push_str(&format!(
                "{}      from: {}\n",
                ind,
                format_expression(&move_stmt.source)
            ));
            output.push_str(&format!("{}      to: ", ind));
            for (i, target) in move_stmt.targets.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format_expression(target));
            }
            output.push('\n');
        }
        Statement::Compute(compute_stmt) => {
            output.push_str(&format!("{}[{}] COMPUTE\n", ind, index));
            output.push_str(&format!(
                "{}      target: {}\n",
                ind,
                format_expression(&compute_stmt.target)
            ));
            output.push_str(&format!(
                "{}      expression: {}\n",
                ind,
                format_expression(&compute_stmt.expression)
            ));
        }
        Statement::Accept(accept_stmt) => {
            output.push_str(&format!("{}[{}] ACCEPT\n", ind, index));
            output.push_str(&format!(
                "{}      {}\n",
                ind,
                format_expression(&accept_stmt.target)
            ));
        }
        Statement::Perform(perform_stmt) => {
            output.push_str(&format!("{}[{}] PERFORM\n", ind, index));
            match perform_stmt {
                PerformStatement::ProcedureName(name, _) => {
                    output.push_str(&format!("{}      {}\n", ind, name));
                }
                PerformStatement::Times {
                    times, paragraph, ..
                } => {
                    output.push_str(&format!(
                        "{}      {} times {}\n",
                        ind,
                        format_expression(times),
                        paragraph
                    ));
                }
                PerformStatement::Until {
                    paragraph,
                    condition,
                    test_after,
                    ..
                } => {
                    output.push_str(&format!(
                        "{}      {}{} until {}\n",
                        ind,
                        paragraph,
                        if *test_after { " with test after" } else { "" },
                        format_condition(condition)
                    ));
                }
                PerformStatement::Thru { first, last, .. } => {
                    output.push_str(&format!("{}      {} thru {}\n", ind, first, last));
                }
                PerformStatement::Varying(varying) => {
                    match &varying.paragraph {
                        Some(paragraph) => {
                            output.push_str(&format!("{}      paragraph: {}\n", ind, paragraph))
                        }
                        None => output.push_str(&format!("{}      inline\n", ind)),
                    }
                    if varying.test_after {
                        output.push_str(&format!("{}      test: AFTER\n", ind));
                    }
                    output.push_str(&format!(
                        "{}      varying: {} from {} by {}\n",
                        ind,
                        format_expression(&varying.variable),
                        format_expression(&varying.from),
                        format_expression(&varying.by)
                    ));
                    output.push_str(&format!(
                        "{}      until: {}\n",
                        ind,
                        format_condition(&varying.condition)
                    ));
                    if varying.paragraph.is_none() {
                        format_block(output, "body", &varying.body, indent);
                    }
                }
                PerformStatement::Inline(inline) => {
                    output.push_str(&format!("{}      inline\n", ind));
                    if inline.test_after {
                        output.push_str(&format!("{}      test: AFTER\n", ind));
                    }
                    if let Some(times) = &inline.times {
                        output.push_str(&format!(
                            "{}      times: {}\n",
                            ind,
                            format_expression(times)
                        ));
                    }
                    if let Some(condition) = &inline.condition {
                        output.push_str(&format!(
                            "{}      until: {}\n",
                            ind,
                            format_condition(condition)
                        ));
                    }
                    format_block(output, "body", &inline.body, indent);
                }
            }
        }
        Statement::If(if_stmt) => {
            output.push_str(&format!("{}[{}] IF\n", ind, index));
            output.push_str(&format!(
                "{}      condition: {}\n",
                ind,
                format_condition(&if_stmt.condition)
            ));
            output.push_str(&format!("{}      then:\n", ind));
            for (nested_index, nested) in if_stmt.then_statements.iter().enumerate() {
                format_statement(output, nested, indent + 8, nested_index);
            }
            if let Some(else_stmts) = &if_stmt.else_statements {
                output.push_str(&format!("{}      else:\n", ind));
                for (nested_index, nested) in else_stmts.iter().enumerate() {
                    format_statement(output, nested, indent + 8, nested_index);
                }
            }
        }
        Statement::Add(add) => {
            output.push_str(&format!("{}[{}] ADD\n", ind, index));
            output.push_str(&format!(
                "{}      operands: {}\n",
                ind,
                format_expression_list(&add.operands)
            ));
            output.push_str(&format!(
                "{}      {}: {}\n",
                ind,
                if add.giving { "giving" } else { "to" },
                format_expression_list(&add.targets)
            ));
            if !add.giving_targets.is_empty() {
                output.push_str(&format!(
                    "{}      giving: {}\n",
                    ind,
                    format_expression_list(&add.giving_targets)
                ));
            }
            format_size_error_statements(output, &add.on_size_error, indent);
        }
        Statement::Subtract(subtract) => {
            output.push_str(&format!("{}[{}] SUBTRACT\n", ind, index));
            output.push_str(&format!(
                "{}      operands: {}\n",
                ind,
                format_expression_list(&subtract.operands)
            ));
            output.push_str(&format!(
                "{}      from: {}\n",
                ind,
                format_expression_list(&subtract.from)
            ));
            if !subtract.giving.is_empty() {
                output.push_str(&format!(
                    "{}      giving: {}\n",
                    ind,
                    format_expression_list(&subtract.giving)
                ));
            }
            format_size_error_statements(output, &subtract.on_size_error, indent);
        }
        Statement::Multiply(multiply) => {
            output.push_str(&format!("{}[{}] MULTIPLY\n", ind, index));
            output.push_str(&format!(
                "{}      factor: {}\n",
                ind,
                format_expression(&multiply.factor)
            ));
            output.push_str(&format!(
                "{}      by: {}\n",
                ind,
                format_expression_list(&multiply.by)
            ));
            if !multiply.giving.is_empty() {
                output.push_str(&format!(
                    "{}      giving: {}\n",
                    ind,
                    format_expression_list(&multiply.giving)
                ));
            }
            format_size_error_statements(output, &multiply.on_size_error, indent);
        }
        Statement::Divide(divide) => {
            output.push_str(&format!("{}[{}] DIVIDE\n", ind, index));
            output.push_str(&format!(
                "{}      dividend: {}\n",
                ind,
                format_expression(&divide.dividend)
            ));
            output.push_str(&format!(
                "{}      divisor: {}\n",
                ind,
                format_expression(&divide.divisor)
            ));
            if !divide.giving.is_empty() {
                output.push_str(&format!(
                    "{}      giving: {}\n",
                    ind,
                    format_expression_list(&divide.giving)
                ));
            }
            if let Some(remainder) = &divide.remainder {
                output.push_str(&format!(
                    "{}      remainder: {}\n",
                    ind,
                    format_expression(remainder)
                ));
            }
            format_size_error_statements(output, &divide.on_size_error, indent);
        }
        Statement::StopRun(_) => {
            output.push_str(&format!("{}[{}] STOP RUN\n", ind, index));
        }
        Statement::GoTo(goto_stmt) => {
            output.push_str(&format!("{}[{}] GO TO\n", ind, index));
            output.push_str(&format!("{}      {}\n", ind, goto_stmt.target));
        }
        Statement::Paragraph(_, _) => {
            // Paragraphs are handled separately in the procedure division loop
        }
        Statement::Set(set) => {
            output.push_str(&format!("{}[{}] SET\n", ind, index));
            output.push_str(&format!(
                "{}      targets: {}\n",
                ind,
                format_expression_list(&set.targets)
            ));
            let kind = match set.kind {
                SetKind::To => "to",
                SetKind::UpBy => "up by",
                SetKind::DownBy => "down by",
            };
            output.push_str(&format!(
                "{}      {}: {}\n",
                ind,
                kind,
                format_expression(&set.value)
            ));
        }
        Statement::Search(search) => {
            output.push_str(&format!("{}[{}] SEARCH\n", ind, index));
            if search.all {
                output.push_str(&format!("{}      all: true\n", ind));
            }
            output.push_str(&format!(
                "{}      table: {}\n",
                ind,
                format_expression(&search.table)
            ));
            if let Some(varying) = &search.varying {
                output.push_str(&format!(
                    "{}      varying: {}\n",
                    ind,
                    format_expression(varying)
                ));
            }
            if let Some(at_end) = &search.at_end {
                format_block(output, "at end", at_end, indent);
            }
            for when in &search.whens {
                output.push_str(&format!(
                    "{}      when: {}\n",
                    ind,
                    format_condition(&when.condition)
                ));
                for (nested_index, nested) in when.statements.iter().enumerate() {
                    format_statement(output, nested, indent + 8, nested_index);
                }
            }
        }
        Statement::Inspect(inspect) => {
            output.push_str(&format!("{}[{}] INSPECT\n", ind, index));
            output.push_str(&format!(
                "{}      target: {}\n",
                ind,
                format_expression(&inspect.target)
            ));
            for tally in &inspect.tallying {
                output.push_str(&format!(
                    "{}      tallying: {} for {}{}{}\n",
                    ind,
                    format_expression(&tally.counter),
                    format_inspect_mode(tally.mode),
                    format_optional_pattern(&tally.pattern),
                    format_region(&tally.region)
                ));
            }
            for replace in &inspect.replacing {
                output.push_str(&format!(
                    "{}      replacing: {}{} by {}{}\n",
                    ind,
                    format_inspect_mode(replace.mode),
                    format_optional_pattern(&replace.pattern),
                    format_expression(&replace.by),
                    format_region(&replace.region)
                ));
            }
            if let Some(convert) = &inspect.converting {
                output.push_str(&format!(
                    "{}      converting: {} to {}{}\n",
                    ind,
                    format_expression(&convert.from),
                    format_expression(&convert.to),
                    format_region(&convert.region)
                ));
            }
        }
        Statement::Evaluate(evaluate) => {
            output.push_str(&format!("{}[{}] EVALUATE\n", ind, index));
            output.push_str(&format!(
                "{}      subject: {}\n",
                ind,
                format_expression(&evaluate.subject)
            ));
            for when in &evaluate.whens {
                let mut labels: Vec<String> = when.choices.iter().map(format_choice).collect();
                if when.other {
                    labels.push("OTHER".to_string());
                }
                output.push_str(&format!("{}      when: {}\n", ind, labels.join(", ")));
                for (nested_index, nested) in when.statements.iter().enumerate() {
                    format_statement(output, nested, indent + 8, nested_index);
                }
            }
        }
        Statement::Open(open) => {
            output.push_str(&format!("{}[{}] OPEN\n", ind, index));
            for group in &open.groups {
                let files: Vec<&str> = group.files.iter().map(|(name, _)| name.as_str()).collect();
                output.push_str(&format!(
                    "{}      {}: {}\n",
                    ind,
                    group.mode,
                    files.join(", ")
                ));
            }
        }
        Statement::Close(close) => {
            output.push_str(&format!("{}[{}] CLOSE\n", ind, index));
            let files: Vec<&str> = close.files.iter().map(|(name, _)| name.as_str()).collect();
            output.push_str(&format!("{}      files: {}\n", ind, files.join(", ")));
        }
        Statement::Read(read) => {
            output.push_str(&format!("{}[{}] READ\n", ind, index));
            output.push_str(&format!("{}      file: {}\n", ind, read.file.0));
            if let Some(into) = &read.into {
                output.push_str(&format!("{}      into: {}\n", ind, format_expression(into)));
            }
            if let Some(key) = &read.key {
                output.push_str(&format!("{}      key: {}\n", ind, format_expression(key)));
            }
            format_handlers(output, &read.handlers, indent);
        }
        Statement::Write(write) => {
            output.push_str(&format!("{}[{}] WRITE\n", ind, index));
            output.push_str(&format!(
                "{}      record: {}\n",
                ind,
                format_expression(&write.record)
            ));
            if let Some(from) = &write.from {
                output.push_str(&format!("{}      from: {}\n", ind, format_expression(from)));
            }
            if let Some(advancing) = &write.advancing {
                let when = if advancing.before { "before" } else { "after" };
                let lines = advancing
                    .lines
                    .as_ref()
                    .map_or_else(|| "PAGE".to_string(), format_expression);
                output.push_str(&format!("{}      {} advancing: {}\n", ind, when, lines));
            }
            format_handlers(output, &write.handlers, indent);
        }
        Statement::Initialize(initialize) => {
            output.push_str(&format!("{}[{}] INITIALIZE\n", ind, index));
            output.push_str(&format!(
                "{}      targets: {}\n",
                ind,
                format_expression_list(&initialize.targets)
            ));
        }
        Statement::Start(start) => {
            output.push_str(&format!("{}[{}] START\n", ind, index));
            output.push_str(&format!("{}      file: {}\n", ind, start.file.0));
            if let Some((op, key)) = &start.key {
                output.push_str(&format!(
                    "{}      key: {} {}\n",
                    ind,
                    format_binary_operator(op),
                    format_expression(key)
                ));
            }
            format_handlers(output, &start.handlers, indent);
        }
    }
}

fn format_choice(choice: &EvaluateChoice) -> String {
    match choice {
        EvaluateChoice::Value(expression) => format_expression(expression),
        EvaluateChoice::Range(first, last) => {
            format!(
                "{} thru {}",
                format_expression(first),
                format_expression(last)
            )
        }
        EvaluateChoice::Condition(condition) => format_condition(condition),
        EvaluateChoice::Any => "ANY".to_string(),
    }
}

fn format_block(output: &mut String, label: &str, statements: &[Statement], indent: usize) {
    output.push_str(&format!("{}      {}:\n", " ".repeat(indent), label));
    for (index, statement) in statements.iter().enumerate() {
        format_statement(output, statement, indent + 8, index);
    }
}

fn format_handlers(output: &mut String, handlers: &IoHandlers, indent: usize) {
    for (label, block) in [
        ("at end", &handlers.at_end),
        ("not at end", &handlers.not_at_end),
        ("invalid key", &handlers.invalid_key),
        ("not invalid key", &handlers.not_invalid_key),
    ] {
        if let Some(statements) = block {
            format_block(output, label, statements, indent);
        }
    }
}

fn format_inspect_mode(mode: InspectMode) -> &'static str {
    match mode {
        InspectMode::Characters => "CHARACTERS",
        InspectMode::All => "ALL",
        InspectMode::Leading => "LEADING",
        InspectMode::First => "FIRST",
    }
}

fn format_optional_pattern(pattern: &Option<Expression>) -> String {
    pattern.as_ref().map_or_else(String::new, |pattern| {
        format!(" {}", format_expression(pattern))
    })
}

fn format_region(region: &InspectRegion) -> String {
    let mut text = String::new();
    if let Some(before) = &region.before {
        text.push_str(&format!(" before {}", format_expression(before)));
    }
    if let Some(after) = &region.after {
        text.push_str(&format!(" after {}", format_expression(after)));
    }
    text
}

pub(crate) fn format_size_error_statements(
    output: &mut String,
    statements: &[Statement],
    indent: usize,
) {
    if !statements.is_empty() {
        let ind = " ".repeat(indent);
        output.push_str(&format!("{}      ON SIZE ERROR\n", ind));
        for (index, statement) in statements.iter().enumerate() {
            format_statement(output, statement, indent + 6, index);
        }
    }
}

pub(crate) fn format_expression_list(expressions: &[Expression]) -> String {
    expressions
        .iter()
        .map(format_expression)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_expression(expr: &Expression) -> String {
    match expr {
        Expression::StringLiteral(s, _) => format!("\"{}\"", s),
        Expression::FunctionCall {
            name, arguments, ..
        } => format!("FUNCTION {}({})", name, format_expression_list(arguments)),
        Expression::NumericLiteral(n, _) => n.clone(),
        Expression::Identifier(id, _) => id.clone(),
        Expression::FigurativeConstant(word, _) => word.clone(),
        Expression::Subscript { name, indices, .. } => {
            format!("{}({})", name, format_expression_list(indices))
        }
        Expression::RefMod {
            name,
            start,
            length,
            ..
        } => match length {
            Some(length) => format!(
                "{}({}:{})",
                name,
                format_expression(start),
                format_expression(length)
            ),
            None => format!("{}({}:)", name, format_expression(start)),
        },
        Expression::BinaryOp {
            left, op, right, ..
        } => {
            format!(
                "{} {} {}",
                format_expression(left),
                format_binary_operator(op),
                format_expression(right)
            )
        }
        Expression::UnaryOp { op, operand, .. } => {
            format!(
                "{}{}",
                format_unary_operator(op),
                format_expression(operand)
            )
        }
    }
}

pub(crate) fn format_binary_operator(op: &BinaryOperator) -> &'static str {
    match op {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Power => "**",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::LessThan => "<",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThanOrEqual => ">=",
    }
}

pub(crate) fn format_unary_operator(op: &UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Plus => "+",
        UnaryOperator::Minus => "-",
    }
}

pub(crate) fn format_condition(cond: &Condition) -> String {
    match cond {
        Condition::Relation {
            left, op, right, ..
        } => {
            format!(
                "{} {} {}",
                format_expression(left),
                format_binary_operator(op),
                format_expression(right)
            )
        }
        Condition::And { left, right, .. } => {
            format!("{} AND {}", format_condition(left), format_condition(right))
        }
        Condition::Or { left, right, .. } => {
            format!("{} OR {}", format_condition(left), format_condition(right))
        }
        Condition::Not { condition, .. } => {
            format!("NOT {}", format_condition(condition))
        }
    }
}
