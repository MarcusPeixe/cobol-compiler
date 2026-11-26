//! Statements.

use super::data::figurative_byte;
use super::expr::{Class, Kind, Place, class_of, name_and_indices};
use super::{Generator, expression_span, index_label, para_label};
use crate::parser::{
    AcceptStatement, AddStatement, BinaryOperator, Condition, DisplayItem, DisplayStatement,
    DivideStatement, EvaluateChoice, EvaluateStatement, Expression, IfStatement, MoveStatement,
    MultiplyStatement, PerformStatement, PerformVarying, SetKind, SetStatement, Statement,
    SubtractStatement,
};
use crate::source::Span;

/// Statement lists directly nested inside a statement.
pub(super) fn nested_statements(statement: &Statement) -> Vec<&[Statement]> {
    match statement {
        Statement::If(node) => {
            let mut lists = vec![node.then_statements.as_slice()];
            lists.extend(node.else_statements.as_deref());
            lists
        }
        Statement::Perform(PerformStatement::Inline(inline)) => vec![inline.body.as_slice()],
        Statement::Perform(PerformStatement::Varying(varying)) => vec![varying.body.as_slice()],
        Statement::Evaluate(node) => node
            .whens
            .iter()
            .map(|when| when.statements.as_slice())
            .collect(),
        Statement::Add(node) => vec![node.on_size_error.as_slice()],
        Statement::Subtract(node) => vec![node.on_size_error.as_slice()],
        Statement::Multiply(node) => vec![node.on_size_error.as_slice()],
        Statement::Divide(node) => vec![node.on_size_error.as_slice()],
        Statement::Search(node) => {
            let mut lists: Vec<&[Statement]> = node
                .whens
                .iter()
                .map(|when| when.statements.as_slice())
                .collect();
            lists.extend(node.at_end.as_deref());
            lists
        }
        _ => Vec::new(),
    }
}

fn binary(left: Expression, op: BinaryOperator, right: Expression) -> Expression {
    let span = expression_span(&left);
    Expression::BinaryOp {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span,
    }
}

/// `a + b + c` for a non-empty list.
fn sum(operands: &[Expression]) -> Expression {
    let mut total = operands[0].clone();
    for operand in &operands[1..] {
        total = binary(total, BinaryOperator::Add, operand.clone());
    }
    total
}

fn relation(left: &Expression, op: BinaryOperator, right: &Expression) -> Condition {
    Condition::Relation {
        left: left.clone(),
        op,
        right: right.clone(),
        span: expression_span(left),
    }
}

impl Generator<'_> {
    pub(super) fn statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.statement(statement);
        }
    }

    fn statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Display(node) => self.display(node),
            Statement::Move(node) => self.gen_move(node),
            Statement::StopRun(_) => {
                self.emit("li $v0, 10");
                self.emit("syscall");
            }
            Statement::Accept(node) => self.accept(node),
            Statement::Compute(node) => {
                self.arithmetic(&[], |this| this.assign(&node.target, &node.expression));
            }
            Statement::Add(node) => self.add(node),
            Statement::Subtract(node) => self.subtract(node),
            Statement::Multiply(node) => self.multiply(node),
            Statement::Divide(node) => self.divide(node),
            Statement::If(node) => self.if_statement(node),
            Statement::Perform(node) => self.perform_statement(node),
            Statement::GoTo(node) => {
                let name = node.target.to_ascii_uppercase();
                match self.paragraphs.get(&name) {
                    Some(&number) => self.emit(format!("j {}", para_label(number, &name))),
                    None => self.unsupported(node.span, &format!("GO TO unknown paragraph {name}")),
                }
            }
            Statement::Evaluate(node) => self.evaluate(node),
            Statement::Set(node) => self.set(node),
            Statement::Paragraph(..) => {}
            Statement::Search(node) => self.unsupported(node.span, "SEARCH"),
            Statement::Inspect(node) => self.unsupported(node.span, "INSPECT"),
            Statement::Open(node) => self.unsupported(node.span, "OPEN"),
            Statement::Close(node) => self.unsupported(node.span, "CLOSE"),
            Statement::Read(node) => self.unsupported(node.span, "READ"),
            Statement::Write(node) => self.unsupported(node.span, "WRITE"),
            Statement::Start(node) => self.unsupported(node.span, "START"),
            Statement::Initialize(node) => self.unsupported(node.span, "INITIALIZE"),
        }
    }

    fn print_literal(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let label = self.literal(bytes);
        self.emit(format!("la $a0, {label}"));
        self.emit("li $v0, 4");
        self.emit("syscall");
    }

    fn display(&mut self, node: &DisplayStatement) {
        for item in &node.items {
            match item {
                DisplayItem::String(text, _) => self.print_literal(text.as_bytes()),
                DisplayItem::Expression(expression) => self.display_expression(expression),
            }
        }
        if !node.no_advancing {
            self.emit("li $a0, 10");
            self.emit("li $v0, 11");
            self.emit("syscall");
        }
    }

    fn display_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::StringLiteral(text, _) | Expression::NumericLiteral(text, _) => {
                self.print_literal(text.as_bytes());
            }
            Expression::FigurativeConstant(word, _) => {
                self.print_literal(&[figurative_byte(word)]);
            }
            _ if self.kind(expression) == Kind::Text => {
                if self.gen_text(expression) {
                    self.emit("jal rt_print");
                }
            }
            Expression::Identifier(name, span) | Expression::Subscript { name, span, .. } => {
                if let Some(Place::Index(index)) = self.place(name) {
                    self.emit(format!("lw $a0, {}", index_label(&index)));
                    self.emit("li $v0, 1");
                    self.emit("syscall");
                    return;
                }
                let indices = name_and_indices(expression).map_or(&[][..], |(_, i)| i);
                if let Some(item) = self.gen_addr(name, indices, *span) {
                    let info = item.picture.as_ref();
                    self.emit(format!("li $a1, {}", item.element_size));
                    self.emit(format!("li $a2, {}", info.map_or(0, |info| info.scale)));
                    self.emit(format!(
                        "li $a3, {}",
                        u8::from(info.is_some_and(|info| info.signed))
                    ));
                    self.emit("jal rt_print_num");
                }
            }
            other => {
                let scale = self.gen_num(other);
                self.emit("move $a0, $v0");
                self.emit(format!("li $a1, {scale}"));
                self.emit("jal rt_print_val");
            }
        }
    }

    /// The storage class of an assignment target: `Some(None)` for an index
    /// name, `None` when the target cannot be resolved.
    fn target_class(&self, target: &Expression) -> Option<Option<Class>> {
        if matches!(target, Expression::RefMod { .. }) {
            return Some(Some(Class::Text));
        }
        let (name, _) = name_and_indices(target)?;
        match self.place(name)? {
            Place::Index(_) => Some(None),
            Place::Item(resolved) => Some(Some(class_of(resolved.item()))),
        }
    }

    fn gen_move(&mut self, node: &MoveStatement) {
        for target in &node.targets {
            self.move_one(&node.source, target, node.span);
        }
    }

    fn move_one(&mut self, source: &Expression, target: &Expression, span: Span) {
        let Some(class) = self.target_class(target) else {
            self.unsupported(span, "this MOVE target");
            return;
        };
        if class != Some(Class::Text) {
            if self.kind(source) == Kind::Text {
                self.unsupported(span, "MOVE of text to a numeric item");
            } else {
                self.assign(target, source);
            }
            return;
        }
        if let Expression::FigurativeConstant(word, _) = source {
            if self.gen_text(target) {
                self.emit(format!("li $a2, {}", figurative_byte(word)));
                self.emit("jal rt_fill");
            }
            return;
        }
        let computed = matches!(
            source,
            Expression::BinaryOp { .. } | Expression::UnaryOp { .. }
        ) || matches!(source, Expression::FunctionCall { .. })
            && self.kind(source) == Kind::Num;
        if computed {
            self.unsupported(span, "MOVE of a computed value to a text item");
            return;
        }
        if !self.gen_text(source) {
            return;
        }
        self.emit("addiu $sp, $sp, -8");
        self.emit("sw $a0, 0($sp)");
        self.emit("sw $a1, 4($sp)");
        if !self.gen_text(target) {
            return;
        }
        self.emit("move $a2, $a0");
        self.emit("move $a3, $a1");
        self.emit("lw $a0, 0($sp)");
        self.emit("lw $a1, 4($sp)");
        self.emit("addiu $sp, $sp, 8");
        self.emit("jal rt_copy_pad");
    }

    fn target_scale(&self, target: &Expression) -> u32 {
        name_and_indices(target)
            .and_then(|(name, _)| match self.place(name) {
                Some(Place::Item(resolved)) => resolved
                    .item()
                    .picture
                    .as_ref()
                    .map(|info| info.scale as u32),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// `target = expression`.
    fn assign(&mut self, target: &Expression, expression: &Expression) {
        self.div_scale = self.target_scale(target).max(4);
        let scale = self.gen_num(expression);
        self.store_num(target, scale);
    }

    fn accept(&mut self, node: &AcceptStatement) {
        match self.target_class(&node.target) {
            Some(Some(Class::Text)) => {
                if self.gen_text(&node.target) {
                    self.emit("jal rt_accept_str");
                }
            }
            Some(_) => {
                let scale = self.target_scale(&node.target);
                self.emit(format!("li $a0, {scale}"));
                self.emit("jal rt_accept_num");
                self.store_num(&node.target, scale);
            }
            None => self.unsupported(node.span, "this ACCEPT target"),
        }
    }

    /// Runs `body` as an arithmetic statement: stores are skipped after a
    /// division by zero, and `handler` runs when a size error occurred.
    fn arithmetic(&mut self, handler: &[Statement], body: impl FnOnce(&mut Self)) {
        self.guard = true;
        self.size_check = !handler.is_empty();
        self.emit("sw $zero, size_err");
        body(self);
        self.guard = false;
        self.size_check = false;
        if !handler.is_empty() {
            let done = self.new_label("size_ok");
            self.emit("lw $t0, size_err");
            self.emit(format!("beqz $t0, {done}"));
            self.statements(handler);
            self.label(&done);
        }
    }

    fn add(&mut self, node: &AddStatement) {
        self.arithmetic(&node.on_size_error, |this| {
            let total = sum(&node.operands);
            if node.giving {
                for target in &node.targets {
                    this.assign(target, &total);
                }
            } else if !node.giving_targets.is_empty() {
                let with_first = binary(total, BinaryOperator::Add, node.targets[0].clone());
                for target in &node.giving_targets {
                    this.assign(target, &with_first);
                }
            } else {
                for target in &node.targets {
                    let value = binary(target.clone(), BinaryOperator::Add, total.clone());
                    this.assign(target, &value);
                }
            }
        });
    }

    fn subtract(&mut self, node: &SubtractStatement) {
        self.arithmetic(&node.on_size_error, |this| {
            let total = sum(&node.operands);
            if node.giving.is_empty() {
                for target in &node.from {
                    let value = binary(target.clone(), BinaryOperator::Subtract, total.clone());
                    this.assign(target, &value);
                }
            } else {
                let value = binary(node.from[0].clone(), BinaryOperator::Subtract, total);
                for target in &node.giving {
                    this.assign(target, &value);
                }
            }
        });
    }

    fn multiply(&mut self, node: &MultiplyStatement) {
        self.arithmetic(&node.on_size_error, |this| {
            if node.giving.is_empty() {
                for target in &node.by {
                    let value = binary(
                        target.clone(),
                        BinaryOperator::Multiply,
                        node.factor.clone(),
                    );
                    this.assign(target, &value);
                }
            } else {
                let value = binary(
                    node.factor.clone(),
                    BinaryOperator::Multiply,
                    node.by[0].clone(),
                );
                for target in &node.giving {
                    this.assign(target, &value);
                }
            }
        });
    }

    fn divide(&mut self, node: &DivideStatement) {
        if node.giving.is_empty() {
            self.unsupported(node.span, "DIVIDE without GIVING");
            return;
        }
        self.arithmetic(&node.on_size_error, |this| {
            let quotient = binary(
                node.dividend.clone(),
                BinaryOperator::Divide,
                node.divisor.clone(),
            );
            for target in &node.giving {
                this.assign(target, &quotient);
            }
            if let Some(remainder) = &node.remainder {
                let product = binary(
                    node.giving[0].clone(),
                    BinaryOperator::Multiply,
                    node.divisor.clone(),
                );
                let value = binary(node.dividend.clone(), BinaryOperator::Subtract, product);
                this.assign(remainder, &value);
            }
        });
    }

    fn set(&mut self, node: &SetStatement) {
        for target in &node.targets {
            let value = match node.kind {
                SetKind::To => node.value.clone(),
                SetKind::UpBy => binary(target.clone(), BinaryOperator::Add, node.value.clone()),
                SetKind::DownBy => {
                    binary(target.clone(), BinaryOperator::Subtract, node.value.clone())
                }
            };
            self.assign(target, &value);
        }
    }

    fn if_statement(&mut self, node: &IfStatement) {
        let otherwise = self.new_label("else");
        self.gen_cond(&node.condition, &otherwise, false);
        self.statements(&node.then_statements);
        match &node.else_statements {
            Some(else_statements) => {
                let end = self.new_label("endif");
                self.emit(format!("j {end}"));
                self.label(&otherwise);
                self.statements(else_statements);
                self.label(&end);
            }
            None => self.label(&otherwise),
        }
    }

    fn perform_statement(&mut self, node: &PerformStatement) {
        match node {
            PerformStatement::ProcedureName(name, span) => self.perform(name, name, *span),
            PerformStatement::Thru { first, last, span } => self.perform(first, last, *span),
            PerformStatement::Times {
                times,
                paragraph,
                span,
            } => self.perform_times(times, |this| this.perform(paragraph, paragraph, *span)),
            PerformStatement::Until {
                condition,
                paragraph,
                span,
            } => self.perform_until(condition, |this| this.perform(paragraph, paragraph, *span)),
            PerformStatement::Varying(varying) => self.perform_varying(varying),
            PerformStatement::Inline(inline) => {
                if let Some(times) = &inline.times {
                    self.perform_times(times, |this| this.statements(&inline.body));
                } else if let Some(condition) = &inline.condition {
                    self.perform_until(condition, |this| this.statements(&inline.body));
                } else {
                    self.statements(&inline.body);
                }
            }
        }
    }

    fn perform_times(&mut self, times: &Expression, body: impl FnOnce(&mut Self)) {
        let counter = self.new_word("times");
        let top = self.new_label("times_top");
        let end = self.new_label("times_end");
        let scale = self.gen_num(times);
        self.rescale(scale, 0);
        self.emit(format!("sw $v0, {counter}"));
        self.label(&top);
        self.emit(format!("lw $t0, {counter}"));
        self.emit(format!("blez $t0, {end}"));
        self.emit("addiu $t0, $t0, -1");
        self.emit(format!("sw $t0, {counter}"));
        body(self);
        self.emit(format!("j {top}"));
        self.label(&end);
    }

    fn perform_until(&mut self, condition: &Condition, body: impl FnOnce(&mut Self)) {
        let top = self.new_label("until_top");
        let end = self.new_label("until_end");
        self.label(&top);
        self.gen_cond(condition, &end, true);
        body(self);
        self.emit(format!("j {top}"));
        self.label(&end);
    }

    fn perform_varying(&mut self, node: &PerformVarying) {
        self.assign(&node.variable, &node.from);
        self.perform_until(&node.condition, |this| {
            match &node.paragraph {
                Some(paragraph) => this.perform(paragraph, paragraph, node.span),
                None => this.statements(&node.body),
            }
            let next = binary(node.variable.clone(), BinaryOperator::Add, node.by.clone());
            this.assign(&node.variable, &next);
        });
    }

    fn evaluate(&mut self, node: &EvaluateStatement) {
        let end = self.new_label("eval_end");
        let subject_false = matches!(
            &node.subject,
            Expression::FigurativeConstant(word, _) if word == "FALSE"
        );
        for when in &node.whens {
            let body = self.new_label("when");
            let next = self.new_label("when_next");
            for choice in &when.choices {
                match choice {
                    EvaluateChoice::Any => self.emit(format!("j {body}")),
                    EvaluateChoice::Condition(condition) => {
                        self.gen_cond(condition, &body, !subject_false);
                    }
                    EvaluateChoice::Value(value) => {
                        let condition = relation(&node.subject, BinaryOperator::Equal, value);
                        self.gen_cond(&condition, &body, true);
                    }
                    EvaluateChoice::Range(low, high) => {
                        let condition = Condition::And {
                            left: Box::new(relation(
                                &node.subject,
                                BinaryOperator::GreaterThanOrEqual,
                                low,
                            )),
                            right: Box::new(relation(
                                &node.subject,
                                BinaryOperator::LessThanOrEqual,
                                high,
                            )),
                            span: node.span,
                        };
                        self.gen_cond(&condition, &body, true);
                    }
                }
            }
            if !when.other {
                self.emit(format!("j {next}"));
            }
            self.label(&body);
            self.statements(&when.statements);
            self.emit(format!("j {end}"));
            self.label(&next);
        }
        self.label(&end);
    }
}
