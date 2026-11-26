//! Expressions, addresses and conditions.

use super::data::figurative_byte;
use super::{Generator, expression_span, index_label};
use crate::parser::{BinaryOperator, Condition, Expression, UnaryOperator};
use crate::semantic::{DataType, ItemLayout, Resolved};
use crate::source::Span;

/// What an operand is, for choosing numeric or text semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Num,
    Text,
    /// Figurative constants take the type of the other operand.
    Neutral,
}

pub(super) enum Place<'a> {
    Item(Resolved<'a>),
    Index(String),
}

/// How an elementary item is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    Plain,
    Edited,
    Text,
}

pub(super) fn class_of(item: &ItemLayout) -> Class {
    match &item.picture {
        Some(info) if info.data_type == DataType::Numeric && item.children.is_empty() => {
            if info.edited {
                Class::Edited
            } else {
                Class::Plain
            }
        }
        _ => Class::Text,
    }
}

pub(super) fn pow10(exponent: u32) -> i64 {
    10_i64.pow(exponent.min(18))
}

pub(super) fn name_and_indices(expression: &Expression) -> Option<(&str, &[Expression])> {
    match expression {
        Expression::Identifier(name, _) => Some((name, &[])),
        Expression::Subscript { name, indices, .. } => Some((name, indices)),
        _ => None,
    }
}

fn is_function(name: &str, wanted: &str) -> bool {
    name.eq_ignore_ascii_case(wanted)
}

impl<'a> Generator<'a> {
    pub(super) fn place(&self, name: &str) -> Option<Place<'a>> {
        if let Some(resolved) = self.layout.resolve(name) {
            return Some(Place::Item(resolved));
        }
        let upper = name.to_ascii_uppercase();
        self.indexes.contains(&upper).then_some(Place::Index(upper))
    }

    pub(super) fn kind(&self, expression: &Expression) -> Kind {
        match expression {
            Expression::NumericLiteral(..)
            | Expression::BinaryOp { .. }
            | Expression::UnaryOp { .. } => Kind::Num,
            Expression::StringLiteral(..) | Expression::RefMod { .. } => Kind::Text,
            Expression::FigurativeConstant(..) => Kind::Neutral,
            Expression::FunctionCall { name, .. } => {
                if is_function(name, "LENGTH") {
                    Kind::Num
                } else {
                    Kind::Text
                }
            }
            Expression::Identifier(name, _) | Expression::Subscript { name, .. } => {
                match self.place(name) {
                    Some(Place::Item(resolved)) if class_of(resolved.item()) == Class::Plain => {
                        Kind::Num
                    }
                    Some(Place::Index(_)) => Kind::Num,
                    _ => Kind::Text,
                }
            }
        }
    }

    /// Emits code leaving the address of a data item in `$a0`.
    pub(super) fn gen_addr(
        &mut self,
        name: &str,
        indices: &[Expression],
        span: Span,
    ) -> Option<&'a ItemLayout> {
        let Some(Place::Item(resolved)) = self.place(name) else {
            self.unsupported(span, &format!("data reference {name}"));
            return None;
        };
        if resolved.file.is_some() {
            self.unsupported(span, "file record access");
            return None;
        }
        let item = resolved.item();
        let mut offset = item.offset as i64;
        let mut dynamic = 0;
        for (position, table) in resolved.tables().into_iter().enumerate() {
            let stride = table.element_size as i64;
            match indices.get(position) {
                None => {}
                Some(Expression::NumericLiteral(text, _)) if text.parse::<i64>().is_ok() => {
                    offset += (text.parse::<i64>().unwrap_or(1) - 1) * stride;
                }
                Some(index) => {
                    self.gen_num(index);
                    self.emit("addiu $v0, $v0, -1");
                    self.emit(format!("li $t0, {stride}"));
                    self.emit("mul $v0, $v0, $t0");
                    self.push_v0();
                    dynamic += 1;
                }
            }
        }
        if offset == 0 {
            self.emit("la $a0, ws");
        } else {
            self.emit(format!("la $a0, ws+{offset}"));
        }
        for _ in 0..dynamic {
            self.pop("$t0");
            self.emit("addu $a0, $a0, $t0");
        }
        Some(item)
    }

    pub(super) fn rescale(&mut self, from: u32, to: u32) {
        if to > from {
            self.emit(format!("li $t0, {}", pow10(to - from)));
            self.emit("mul $v0, $v0, $t0");
        } else if to < from {
            self.emit(format!("li $t0, {}", pow10(from - to)));
            self.emit("div $v0, $t0");
            self.emit("mflo $v0");
        }
    }

    /// Emits code leaving a scaled integer in `$v0`; returns its scale.
    pub(super) fn gen_num(&mut self, expression: &Expression) -> u32 {
        match expression {
            Expression::NumericLiteral(text, span) => {
                let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
                let digits = format!("{int_part}{frac_part}");
                match digits.parse::<i32>() {
                    Ok(value) => self.emit(format!("li $v0, {value}")),
                    Err(_) => {
                        self.unsupported(*span, &format!("numeric literal {text} (over 9 digits)"));
                    }
                }
                frac_part.len() as u32
            }
            Expression::FigurativeConstant(word, span) => {
                if matches!(word.as_str(), "ZERO" | "ZEROS" | "ZEROES") {
                    self.emit("li $v0, 0");
                } else {
                    self.unsupported(*span, &format!("{word} in a numeric context"));
                }
                0
            }
            Expression::Identifier(name, span) | Expression::Subscript { name, span, .. } => {
                let indices = name_and_indices(expression).map_or(&[][..], |(_, i)| i);
                if let Some(Place::Index(index)) = self.place(name) {
                    self.emit(format!("lw $v0, {}", index_label(&index)));
                    return 0;
                }
                let Some(item) = self.gen_addr(name, indices, *span) else {
                    return 0;
                };
                if class_of(item) != Class::Plain {
                    self.unsupported(*span, &format!("non-numeric item {name} in arithmetic"));
                    return 0;
                }
                let info = item.picture.as_ref().map_or(0, |info| info.scale);
                self.emit(format!("li $a1, {}", item.element_size));
                self.emit("jal rt_num_load");
                info as u32
            }
            Expression::UnaryOp { op, operand, .. } => {
                let scale = self.gen_num(operand);
                if *op == UnaryOperator::Minus {
                    self.emit("negu $v0, $v0");
                }
                scale
            }
            Expression::BinaryOp {
                left,
                op,
                right,
                span,
            } => self.gen_binary(left, *op, right, *span),
            Expression::FunctionCall {
                name,
                arguments,
                span,
            } if is_function(name, "LENGTH") && arguments.len() == 1 => {
                if let Some((target, _)) = name_and_indices(&arguments[0])
                    && let Some(Place::Item(resolved)) = self.place(target)
                {
                    self.emit(format!("li $v0, {}", resolved.item().element_size));
                    return 0;
                }
                self.unsupported(*span, "FUNCTION LENGTH of this argument");
                0
            }
            other => {
                self.unsupported(expression_span(other), "this expression in arithmetic");
                0
            }
        }
    }

    fn gen_binary(
        &mut self,
        left: &Expression,
        op: BinaryOperator,
        right: &Expression,
        span: Span,
    ) -> u32 {
        if !matches!(
            op,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Power
        ) {
            self.unsupported(span, "a comparison used as a number");
            return 0;
        }
        let left_scale = self.gen_num(left);
        self.push_v0();
        let right_scale = self.gen_num(right);
        match op {
            BinaryOperator::Add | BinaryOperator::Subtract => {
                self.pop("$t0");
                let scale = left_scale.max(right_scale);
                if left_scale < scale {
                    self.emit(format!("li $t1, {}", pow10(scale - left_scale)));
                    self.emit("mul $t0, $t0, $t1");
                }
                if right_scale < scale {
                    self.emit(format!("li $t1, {}", pow10(scale - right_scale)));
                    self.emit("mul $v0, $v0, $t1");
                }
                let mnemonic = if op == BinaryOperator::Add {
                    "addu"
                } else {
                    "subu"
                };
                self.emit(format!("{mnemonic} $v0, $t0, $v0"));
                scale
            }
            BinaryOperator::Multiply => {
                self.pop("$t0");
                self.emit("mul $v0, $t0, $v0");
                left_scale + right_scale
            }
            BinaryOperator::Divide => {
                let scale = left_scale.max(self.div_scale);
                let shift = scale - left_scale + right_scale;
                let done = self.new_label("div_done");
                let ok = self.new_label("div_ok");
                self.emit("move $t1, $v0");
                self.pop("$t0");
                self.emit(format!("bnez $t1, {ok}"));
                self.emit("li $t2, 1");
                self.emit("sw $t2, size_err");
                self.emit("li $v0, 0");
                self.emit(format!("j {done}"));
                self.label(&ok);
                if shift > 0 {
                    self.emit(format!("li $t2, {}", pow10(shift)));
                    self.emit("mul $t0, $t0, $t2");
                }
                self.emit("div $t0, $t1");
                self.emit("mflo $v0");
                self.label(&done);
                scale
            }
            _ => {
                // The exponent is truncated to an integer.
                self.rescale(right_scale, 0);
                self.emit("move $a1, $v0");
                self.pop("$a0");
                self.emit(format!("li $a2, {}", pow10(left_scale)));
                self.emit("jal rt_pow");
                left_scale
            }
        }
    }

    /// Emits code leaving the address in `$a0` and the length in `$a1`.
    /// Returns `false` after reporting an unsupported operand.
    pub(super) fn gen_text(&mut self, expression: &Expression) -> bool {
        match expression {
            Expression::StringLiteral(text, _) | Expression::NumericLiteral(text, _) => {
                let label = self.literal(text.as_bytes());
                self.emit(format!("la $a0, {label}"));
                self.emit(format!("li $a1, {}", text.len()));
                true
            }
            Expression::FigurativeConstant(word, _) => {
                let label = self.literal(&[figurative_byte(word)]);
                self.emit(format!("la $a0, {label}"));
                self.emit("li $a1, 1");
                true
            }
            Expression::Identifier(name, span) | Expression::Subscript { name, span, .. } => {
                let indices = name_and_indices(expression).map_or(&[][..], |(_, i)| i);
                let Some(item) = self.gen_addr(name, indices, *span) else {
                    return false;
                };
                self.emit(format!("li $a1, {}", item.element_size));
                true
            }
            Expression::RefMod {
                name,
                start,
                length,
                span,
            } => self.gen_refmod(name, start, length.as_deref(), *span),
            Expression::FunctionCall {
                name,
                arguments,
                span,
            } => self.gen_text_function(name, arguments, *span),
            other => {
                self.unsupported(expression_span(other), "this expression as text");
                false
            }
        }
    }

    fn gen_refmod(
        &mut self,
        name: &str,
        start: &Expression,
        length: Option<&Expression>,
        span: Span,
    ) -> bool {
        let Some(item) = self.gen_addr(name, &[], span) else {
            return false;
        };
        let size = item.element_size;
        self.emit("addiu $sp, $sp, -4");
        self.emit("sw $a0, 0($sp)");
        self.gen_num(start);
        self.emit("addiu $v0, $v0, -1");
        self.push_v0();
        match length {
            Some(length) => {
                self.gen_num(length);
                self.emit("move $a1, $v0");
                self.pop("$t1");
            }
            None => {
                self.pop("$t1");
                self.emit(format!("li $a1, {size}"));
                self.emit("subu $a1, $a1, $t1");
            }
        }
        self.pop("$t0");
        self.emit("addu $a0, $t0, $t1");
        true
    }

    fn gen_text_function(&mut self, name: &str, arguments: &[Expression], span: Span) -> bool {
        let upper = is_function(name, "UPPER-CASE");
        if !(upper || is_function(name, "LOWER-CASE")) || arguments.len() != 1 {
            self.unsupported(span, &format!("FUNCTION {name}"));
            return false;
        }
        if !self.gen_text(&arguments[0]) {
            return false;
        }
        self.emit("addiu $sp, $sp, -4");
        self.emit("sw $a1, 0($sp)");
        self.emit("move $a3, $a1");
        self.emit("la $a2, fn_buf");
        self.emit("jal rt_copy_pad");
        self.pop("$a1");
        self.emit("la $a0, fn_buf");
        self.emit(format!("li $a2, {}", u8::from(upper)));
        self.emit("jal rt_case");
        self.emit("la $a0, fn_buf");
        true
    }

    /// Stores the scaled integer in `$v0` (of scale `scale`) into `target`.
    pub(super) fn store_num(&mut self, target: &Expression, scale: u32) {
        let Some((name, indices)) = name_and_indices(target) else {
            self.unsupported(expression_span(target), "this assignment target");
            return;
        };
        let span = expression_span(target);
        if let Some(Place::Index(index)) = self.place(name) {
            self.rescale(scale, 0);
            self.emit(format!("sw $v0, {}", index_label(&index)));
            return;
        }
        let Some(Place::Item(resolved)) = self.place(name) else {
            self.unsupported(span, &format!("data reference {name}"));
            return;
        };
        let item = resolved.item();
        let class = class_of(item);
        let Some(info) = item.picture.clone().filter(|_| class != Class::Text) else {
            self.unsupported(span, &format!("numeric store into non-numeric item {name}"));
            return;
        };
        self.rescale(scale, info.scale as u32);
        self.push_v0();
        if self.gen_addr(name, indices, span).is_none() {
            self.pop("$t0");
            return;
        }
        let skip = self.new_label("skip");
        if class == Class::Edited {
            self.pop("$a2");
            let pattern = self.literal(info.pattern.as_bytes());
            self.emit(format!("la $a1, {pattern}"));
            self.emit("jal rt_edit");
            return;
        }
        self.pop("$a3");
        self.emit(format!("li $a1, {}", item.element_size));
        self.emit(format!("li $a2, {}", u8::from(info.signed)));
        if self.guard {
            self.emit("lw $t0, size_err");
            self.emit(format!("bnez $t0, {skip}"));
        }
        if self.size_check {
            let fits = self.new_label("fits");
            self.emit("jal rt_fits");
            self.emit(format!("bnez $v0, {fits}"));
            self.emit("li $t0, 1");
            self.emit("sw $t0, size_err");
            self.emit(format!("j {skip}"));
            self.label(&fits);
        }
        self.emit("jal rt_num_store");
        self.label(&skip);
    }

    /// Jumps to `label` when `condition` evaluates to `jump_if`; falls through otherwise.
    pub(super) fn gen_cond(&mut self, condition: &Condition, label: &str, jump_if: bool) {
        match condition {
            Condition::Not { condition, .. } => self.gen_cond(condition, label, !jump_if),
            Condition::And { left, right, .. } | Condition::Or { left, right, .. } => {
                let is_and = matches!(condition, Condition::And { .. });
                if is_and != jump_if {
                    // AND jumping on false, OR jumping on true: either side decides.
                    self.gen_cond(left, label, jump_if);
                    self.gen_cond(right, label, jump_if);
                } else {
                    let skip = self.new_label("cond");
                    self.gen_cond(left, &skip, !jump_if);
                    self.gen_cond(right, label, jump_if);
                    self.label(&skip);
                }
            }
            Condition::Relation {
                left,
                op,
                right,
                span,
            } => self.gen_relation(left, *op, right, label, jump_if, *span),
        }
    }

    fn gen_relation(
        &mut self,
        left: &Expression,
        op: BinaryOperator,
        right: &Expression,
        label: &str,
        jump_if: bool,
        span: Span,
    ) {
        let op = if jump_if { op } else { negate(op) };
        let text = self.kind(left) == Kind::Text || self.kind(right) == Kind::Text;
        if text {
            if !self.gen_text(left) {
                return;
            }
            self.emit("addiu $sp, $sp, -8");
            self.emit("sw $a0, 0($sp)");
            self.emit("sw $a1, 4($sp)");
            if !self.gen_text(right) {
                return;
            }
            self.emit("move $a2, $a0");
            self.emit("move $a3, $a1");
            self.emit("lw $a0, 0($sp)");
            self.emit("lw $a1, 4($sp)");
            self.emit("addiu $sp, $sp, 8");
            self.emit("jal rt_cmp");
            let branch = match op {
                BinaryOperator::Equal => "beqz",
                BinaryOperator::NotEqual => "bnez",
                BinaryOperator::LessThan => "bltz",
                BinaryOperator::LessThanOrEqual => "blez",
                BinaryOperator::GreaterThan => "bgtz",
                BinaryOperator::GreaterThanOrEqual => "bgez",
                _ => {
                    self.unsupported(span, "this operator in a condition");
                    return;
                }
            };
            self.emit(format!("{branch} $v0, {label}"));
            return;
        }
        let left_scale = self.gen_num(left);
        self.push_v0();
        let right_scale = self.gen_num(right);
        self.pop("$t0");
        let scale = left_scale.max(right_scale);
        if left_scale < scale {
            self.emit(format!("li $t1, {}", pow10(scale - left_scale)));
            self.emit("mul $t0, $t0, $t1");
        }
        if right_scale < scale {
            self.emit(format!("li $t1, {}", pow10(scale - right_scale)));
            self.emit("mul $v0, $v0, $t1");
        }
        let branch = match op {
            BinaryOperator::Equal => "beq",
            BinaryOperator::NotEqual => "bne",
            BinaryOperator::LessThan => "blt",
            BinaryOperator::LessThanOrEqual => "ble",
            BinaryOperator::GreaterThan => "bgt",
            BinaryOperator::GreaterThanOrEqual => "bge",
            _ => {
                self.unsupported(span, "this operator in a condition");
                return;
            }
        };
        self.emit(format!("{branch} $t0, $v0, {label}"));
    }
}

fn negate(op: BinaryOperator) -> BinaryOperator {
    match op {
        BinaryOperator::Equal => BinaryOperator::NotEqual,
        BinaryOperator::NotEqual => BinaryOperator::Equal,
        BinaryOperator::LessThan => BinaryOperator::GreaterThanOrEqual,
        BinaryOperator::LessThanOrEqual => BinaryOperator::GreaterThan,
        BinaryOperator::GreaterThan => BinaryOperator::LessThanOrEqual,
        BinaryOperator::GreaterThanOrEqual => BinaryOperator::LessThan,
        other => other,
    }
}
