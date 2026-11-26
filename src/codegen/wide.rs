//! DISPLAY numeric moves without converting through a 32-bit integer.

use super::data::encode_numeric;
use super::expr::{Class, Place, class_of, name_and_indices};
use super::{Generator, expression_span};
use crate::parser::{BinaryOperator, Expression, UnaryOperator};
use crate::source::Span;

struct RationalValue {
    numerator: String,
    numerator_size: usize,
    denominator: String,
    denominator_size: usize,
}

fn literal_number(expression: &Expression) -> Option<String> {
    match expression {
        Expression::NumericLiteral(value, _) => Some(value.clone()),
        Expression::UnaryOp {
            op: UnaryOperator::Minus,
            operand,
            ..
        } => literal_number(operand).map(|value| format!("-{value}")),
        Expression::UnaryOp {
            op: UnaryOperator::Plus,
            operand,
            ..
        } => literal_number(operand),
        Expression::FigurativeConstant(value, _)
            if matches!(value.as_str(), "ZERO" | "ZEROS" | "ZEROES") =>
        {
            Some("0".to_string())
        }
        _ => None,
    }
}

impl Generator<'_> {
    fn rational_shape(&self, expression: &Expression) -> Option<(usize, usize, bool)> {
        if let Some((digits, scale)) = self.wide_shape(expression) {
            return Some((
                digits,
                scale.saturating_add(1),
                self.is_wide_number(expression),
            ));
        }
        match expression {
            Expression::UnaryOp { operand, .. } => {
                let (numerator, denominator, wide) = self.rational_shape(operand)?;
                Some((numerator.saturating_add(1), denominator, wide))
            }
            Expression::BinaryOp {
                left, op, right, ..
            } => {
                let (ln, ld, lw) = self.rational_shape(left)?;
                let (rn, rd, rw) = self.rational_shape(right)?;
                let (numerator, denominator) = match op {
                    BinaryOperator::Add | BinaryOperator::Subtract => (
                        ln.saturating_add(rd)
                            .max(rn.saturating_add(ld))
                            .saturating_add(1),
                        ld.saturating_add(rd),
                    ),
                    BinaryOperator::Multiply => (ln.saturating_add(rn), ld.saturating_add(rd)),
                    BinaryOperator::Divide => (ln.saturating_add(rd), ld.saturating_add(rn)),
                    _ => return None,
                };
                Some((numerator, denominator, lw || rw))
            }
            _ => None,
        }
    }

    fn rational_buffer(&mut self, width: usize, allocation: &mut usize) -> String {
        let aligned = (width + 3) & !3;
        *allocation += aligned;
        let address = self.new_word("wide_expression_buffer");
        self.emit(format!("li $t0, {aligned}"));
        self.emit("subu $sp, $sp, $t0");
        self.emit(format!("sw $sp, {address}"));
        self.emit("move $a0, $sp");
        self.emit(format!("li $a1, {width}"));
        self.emit("li $a2, 48");
        self.emit("jal rt_fill");
        address
    }

    fn rational_product(
        &mut self,
        left: &str,
        left_size: usize,
        right: &str,
        right_size: usize,
        allocation: &mut usize,
    ) -> String {
        let output = self.rational_buffer(left_size + right_size, allocation);
        self.emit(format!("lw $a0, {left}"));
        self.emit(format!("li $a1, {left_size}"));
        self.emit(format!("lw $a2, {right}"));
        self.emit(format!("li $a3, {right_size}"));
        self.emit(format!("lw $t0, {output}"));
        self.emit("addiu $sp, $sp, -4");
        self.emit("sw $t0, 0($sp)");
        self.emit("jal rt_wide_product");
        self.emit("addiu $sp, $sp, 4");
        output
    }

    fn rational_copy(
        &mut self,
        source: &str,
        size: usize,
        target: &str,
        width: usize,
        scale: usize,
    ) {
        self.emit(format!("lw $a0, {source}"));
        self.emit(format!("li $a1, {size}"));
        self.emit(format!("lw $a2, {target}"));
        self.emit(format!("li $a3, {width}"));
        self.emit("addiu $sp, $sp, -12");
        self.emit("sw $zero, 0($sp)");
        self.emit(format!("li $t0, {scale}"));
        self.emit("sw $t0, 4($sp)");
        self.emit("li $t0, 1");
        self.emit("sw $t0, 8($sp)");
        self.emit("jal rt_wide_move");
        self.emit("addiu $sp, $sp, 12");
    }

    fn rational_expression(
        &mut self,
        expression: &Expression,
        allocation: &mut usize,
        done: &str,
    ) -> Option<RationalValue> {
        if let Some((size, scale)) = self.wide_shape(expression) {
            self.wide_operand(expression)?;
            let numerator = self.new_word("wide_expression_numerator");
            self.emit(format!("sw $a0, {numerator}"));
            let denominator = self.new_word("wide_expression_denominator");
            let one = self.literal(format!("1{}", "0".repeat(scale)).as_bytes());
            self.emit(format!("la $t0, {one}"));
            self.emit(format!("sw $t0, {denominator}"));
            return Some(RationalValue {
                numerator,
                numerator_size: size,
                denominator,
                denominator_size: scale + 1,
            });
        }
        if let Expression::UnaryOp { op, operand, .. } = expression {
            let mut value = self.rational_expression(operand, allocation, done)?;
            if *op == UnaryOperator::Minus {
                let minus = self.literal(b"q");
                let address = self.new_word("wide_expression_minus");
                self.emit(format!("la $t0, {minus}"));
                self.emit(format!("sw $t0, {address}"));
                value.numerator = self.rational_product(
                    &value.numerator,
                    value.numerator_size,
                    &address,
                    1,
                    allocation,
                );
                value.numerator_size += 1;
            }
            return Some(value);
        }
        let Expression::BinaryOp {
            left, op, right, ..
        } = expression
        else {
            return None;
        };
        let left = self.rational_expression(left, allocation, done)?;
        let right = self.rational_expression(right, allocation, done)?;
        if *op == BinaryOperator::Divide {
            let check = self.new_label("wide_expression_check_divisor");
            let nonzero = self.new_label("wide_expression_nonzero");
            let digit = self.new_label("wide_expression_digit");
            self.emit(format!("lw $t0, {}", right.numerator));
            self.emit(format!("li $t1, {}", right.numerator_size));
            self.label(&check);
            self.emit("lbu $t2, 0($t0)");
            self.emit("sltiu $t3, $t2, 0x70");
            self.emit(format!("bnez $t3, {digit}"));
            self.emit("addiu $t2, $t2, -0x40");
            self.label(&digit);
            self.emit("li $t3, 48");
            self.emit(format!("bne $t2, $t3, {nonzero}"));
            self.emit("addiu $t0, $t0, 1");
            self.emit("addiu $t1, $t1, -1");
            self.emit(format!("bnez $t1, {check}"));
            self.rational_divide_error(*allocation, done);
            self.label(&nonzero);
        }
        let (numerator, numerator_size, denominator, denominator_size) = match op {
            BinaryOperator::Multiply | BinaryOperator::Divide => {
                let (right_num, right_num_size, right_den, right_den_size) =
                    if *op == BinaryOperator::Multiply {
                        (
                            &right.numerator,
                            right.numerator_size,
                            &right.denominator,
                            right.denominator_size,
                        )
                    } else {
                        (
                            &right.denominator,
                            right.denominator_size,
                            &right.numerator,
                            right.numerator_size,
                        )
                    };
                let numerator = self.rational_product(
                    &left.numerator,
                    left.numerator_size,
                    right_num,
                    right_num_size,
                    allocation,
                );
                let denominator = self.rational_product(
                    &left.denominator,
                    left.denominator_size,
                    right_den,
                    right_den_size,
                    allocation,
                );
                (
                    numerator,
                    left.numerator_size + right_num_size,
                    denominator,
                    left.denominator_size + right_den_size,
                )
            }
            BinaryOperator::Add | BinaryOperator::Subtract => {
                let left_size = left.numerator_size + right.denominator_size;
                let right_size = right.numerator_size + left.denominator_size;
                let left_product = self.rational_product(
                    &left.numerator,
                    left.numerator_size,
                    &right.denominator,
                    right.denominator_size,
                    allocation,
                );
                let right_product = self.rational_product(
                    &right.numerator,
                    right.numerator_size,
                    &left.denominator,
                    left.denominator_size,
                    allocation,
                );
                let width = left_size.max(right_size) + 1;
                let numerator = self.rational_buffer(width, allocation);
                let scratch = self.rational_buffer(width, allocation);
                self.rational_copy(&left_product, left_size, &numerator, width, 0);
                self.rational_copy(&right_product, right_size, &scratch, width, 0);
                self.emit(format!("lw $a0, {numerator}"));
                self.emit(format!("lw $a1, {scratch}"));
                self.emit(format!("li $a2, {width}"));
                self.emit(format!(
                    "li $a3, {}",
                    u8::from(*op == BinaryOperator::Subtract)
                ));
                self.emit("jal rt_wide_add");
                let denominator = self.rational_product(
                    &left.denominator,
                    left.denominator_size,
                    &right.denominator,
                    right.denominator_size,
                    allocation,
                );
                (
                    numerator,
                    width,
                    denominator,
                    left.denominator_size + right.denominator_size,
                )
            }
            _ => return None,
        };
        Some(RationalValue {
            numerator,
            numerator_size,
            denominator,
            denominator_size,
        })
    }

    fn rational_divide_error(&mut self, allocation: usize, done: &str) {
        if self.size_check {
            self.emit("li $t0, 1");
            self.emit("sw $t0, size_err");
            self.emit(format!("li $t0, {allocation}"));
            self.emit("addu $sp, $sp, $t0");
            self.emit(format!("j {done}"));
        } else {
            let message = self.literal(b"Division by zero in wide expression\n");
            self.emit(format!("la $a0, {message}"));
            self.emit("li $v0, 4");
            self.emit("syscall");
            self.emit("li $v0, 10");
            self.emit("syscall");
        }
    }

    pub(super) fn assign_wide_expression(
        &mut self,
        target: &Expression,
        expression: &Expression,
    ) -> bool {
        let Some((name, _)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(item)) = self.place(name) else {
            return false;
        };
        if class_of(item.item()) != Class::Plain {
            return false;
        }
        let Some(info) = item.item().picture.clone() else {
            return false;
        };
        let Some((numerator, denominator, wide)) = self.rational_shape(expression) else {
            return false;
        };
        if !wide && !self.is_wide_number(target) {
            return false;
        }
        let Some(width) = numerator
            .checked_add(info.scale)
            .and_then(|size| size.max(denominator).checked_add(1))
        else {
            self.unsupported(
                expression_span(expression),
                "wide expression exceeds supported precision",
            );
            return true;
        };
        if width > 512 {
            self.unsupported(
                expression_span(expression),
                "wide expression exceeds 512 digit precision",
            );
            return true;
        }
        self.needs_wide = true;
        self.needs_wide_arithmetic = true;
        let mut allocation = 0;
        let done = self.new_label("wide_expression_done");
        let Some(value) = self.rational_expression(expression, &mut allocation, &done) else {
            return true;
        };
        let dividend = self.rational_buffer(width, &mut allocation);
        let divisor = self.rational_buffer(width, &mut allocation);
        let remainder = self.rational_buffer(width, &mut allocation);
        let quotient = self.rational_buffer(width, &mut allocation);
        self.rational_copy(
            &value.numerator,
            value.numerator_size,
            &dividend,
            width,
            info.scale,
        );
        self.rational_copy(
            &value.denominator,
            value.denominator_size,
            &divisor,
            width,
            0,
        );
        self.emit(format!("lw $a0, {dividend}"));
        self.emit(format!("lw $a1, {divisor}"));
        self.emit(format!("lw $a2, {quotient}"));
        self.emit(format!("lw $a3, {remainder}"));
        self.emit("addiu $sp, $sp, -4");
        self.emit(format!("li $t0, {width}"));
        self.emit("sw $t0, 0($sp)");
        self.emit("jal rt_wide_divide");
        self.emit("addiu $sp, $sp, 4");
        let success = self.new_label("wide_expression_divided");
        self.emit(format!("beqz $v0, {success}"));
        self.rational_divide_error(allocation, &done);
        self.label(&success);
        self.store_wide_result(target, width, info.scale, 0, allocation);
        self.label(&done);
        true
    }

    pub(super) fn accept_wide(&mut self, target: &Expression) -> bool {
        let Some((name, indices)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(resolved)) = self.place(name) else {
            return false;
        };
        let Some(info) = resolved.item().picture.clone() else {
            return false;
        };
        if class_of(resolved.item()) != Class::Plain || info.digits <= 9 {
            return false;
        }
        self.needs_wide = true;
        if self
            .gen_addr(name, indices, expression_span(target))
            .is_some()
        {
            self.emit(format!("li $a1, {}", info.size));
            self.emit(format!("li $a2, {}", info.scale));
            self.emit(format!("li $a3, {}", u8::from(info.signed)));
            self.emit("jal rt_wide_accept");
        }
        true
    }

    pub(super) fn increment_wide(&mut self, target: &Expression, operand: &Expression) -> bool {
        let Some((name, indices)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(resolved)) = self.place(name) else {
            return false;
        };
        let Some(info) = resolved.item().picture.clone() else {
            return false;
        };
        if class_of(resolved.item()) != Class::Plain || info.digits <= 9 {
            return false;
        }
        if literal_number(operand).as_deref() != Some("1") || info.signed || info.scale != 0 {
            return false;
        }
        self.needs_wide = true;
        if self
            .gen_addr(name, indices, expression_span(target))
            .is_none()
        {
            return true;
        }
        self.emit(format!("li $a1, {}", info.size));
        self.emit(format!("li $a2, {}", u8::from(self.size_check)));
        self.emit("jal rt_wide_increment");
        if self.size_check {
            let done = self.new_label("wide_increment_ok");
            self.emit(format!("beqz $v0, {done}"));
            self.emit("li $t0, 1");
            self.emit("sw $t0, size_err");
            self.label(&done);
        }
        true
    }

    pub(super) fn is_wide_number(&self, expression: &Expression) -> bool {
        if let Some(value) = literal_number(expression) {
            return value.bytes().filter(u8::is_ascii_digit).count() > 9;
        }
        name_and_indices(expression)
            .and_then(|(name, _)| match self.place(name) {
                Some(Place::Item(item)) if class_of(item.item()) == Class::Plain => {
                    item.item().picture.as_ref()
                }
                _ => None,
            })
            .is_some_and(|info| info.digits > 9)
    }

    fn wide_shape(&self, expression: &Expression) -> Option<(usize, usize)> {
        if let Some(value) = literal_number(expression) {
            let unsigned = value.trim_start_matches(['+', '-']);
            let digits = unsigned.bytes().filter(u8::is_ascii_digit).count();
            let scale = unsigned.split_once('.').map_or(0, |(_, frac)| frac.len());
            return Some((digits, scale));
        }
        let (name, _) = name_and_indices(expression)?;
        let Place::Item(item) = self.place(name)? else {
            return None;
        };
        if class_of(item.item()) != Class::Plain {
            return None;
        }
        let info = item.item().picture.as_ref()?;
        Some((info.digits, info.scale))
    }

    pub(super) fn assign_wide_sum(&mut self, target: &Expression, expression: &Expression) -> bool {
        fn terms<'a>(
            expression: &'a Expression,
            negative: bool,
            output: &mut Vec<(&'a Expression, bool)>,
        ) -> bool {
            match expression {
                Expression::BinaryOp {
                    left, op, right, ..
                } if *op == BinaryOperator::Add => {
                    terms(left, negative, output) && terms(right, negative, output)
                }
                Expression::BinaryOp {
                    left, op, right, ..
                } if *op == BinaryOperator::Subtract => {
                    terms(left, negative, output) && terms(right, !negative, output)
                }
                Expression::UnaryOp { op, operand, .. }
                    if *op == UnaryOperator::Minus && literal_number(expression).is_none() =>
                {
                    terms(operand, !negative, output)
                }
                Expression::UnaryOp { op, operand, .. }
                    if *op == UnaryOperator::Plus && literal_number(expression).is_none() =>
                {
                    terms(operand, negative, output)
                }
                Expression::BinaryOp { .. } | Expression::UnaryOp { .. }
                    if literal_number(expression).is_none() =>
                {
                    false
                }
                _ => {
                    output.push((expression, negative));
                    true
                }
            }
        }

        let Some((name, _)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(item)) = self.place(name) else {
            return false;
        };
        if class_of(item.item()) != Class::Plain {
            return false;
        }
        let Some(info) = item.item().picture.clone() else {
            return false;
        };
        let mut operands = Vec::new();
        if !terms(expression, false, &mut operands)
            || operands
                .iter()
                .any(|(term, _)| self.wide_shape(term).is_none())
            || (!self.is_wide_number(target)
                && !operands.iter().any(|(term, _)| self.is_wide_number(term)))
        {
            return false;
        }
        let mut scale = info.scale;
        let mut integer = info.integer_digits();
        for (term, _) in &operands {
            let (digits, term_scale) = self.wide_shape(term).expect("numeric operand");
            scale = scale.max(term_scale);
            integer = integer.max(digits.saturating_sub(term_scale));
        }
        integer += operands.len().to_string().len();
        let width = integer + scale;
        let aligned = (width + 3) & !3;
        self.needs_wide = true;
        self.needs_wide_arithmetic = true;
        self.emit(format!("li $t0, {}", aligned * 2));
        self.emit("subu $sp, $sp, $t0");
        self.emit("move $a0, $sp");
        self.emit(format!("li $a1, {width}"));
        self.emit("li $a2, 48");
        self.emit("jal rt_fill");
        for (term, negative) in operands {
            let Some((size, term_scale)) = self.wide_operand(term) else {
                self.emit(format!("li $t0, {}", aligned * 2));
                self.emit("addu $sp, $sp, $t0");
                return true;
            };
            self.emit(format!("li $a1, {size}"));
            self.emit(format!("li $t0, {aligned}"));
            self.emit("addu $a2, $sp, $t0");
            self.emit(format!("li $a3, {width}"));
            self.emit("addiu $sp, $sp, -12");
            self.emit(format!("li $t0, {term_scale}"));
            self.emit("sw $t0, 0($sp)");
            self.emit(format!("li $t0, {scale}"));
            self.emit("sw $t0, 4($sp)");
            self.emit("li $t0, 1");
            self.emit("sw $t0, 8($sp)");
            self.emit("jal rt_wide_move");
            self.emit("addiu $sp, $sp, 12");
            self.emit("move $a0, $sp");
            self.emit(format!("li $t0, {aligned}"));
            self.emit("addu $a1, $sp, $t0");
            self.emit(format!("li $a2, {width}"));
            self.emit(format!("li $a3, {}", u8::from(negative)));
            self.emit("jal rt_wide_add");
        }
        self.store_wide_result(target, width, scale, 0, aligned * 2);
        true
    }

    pub(super) fn assign_wide_product(
        &mut self,
        target: &Expression,
        expression: &Expression,
    ) -> bool {
        let Expression::BinaryOp {
            left,
            op: BinaryOperator::Multiply,
            right,
            ..
        } = expression
        else {
            return false;
        };
        let Some((name, _)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(item)) = self.place(name) else {
            return false;
        };
        if class_of(item.item()) != Class::Plain {
            return false;
        }
        let Some((left_size, left_scale)) = self.wide_shape(left) else {
            return false;
        };
        let Some((right_size, right_scale)) = self.wide_shape(right) else {
            return false;
        };
        if !self.is_wide_number(target) && !self.is_wide_number(left) && !self.is_wide_number(right)
        {
            return false;
        }
        let width = left_size + right_size;
        let aligned = (width + 3) & !3;
        let left_address = self.new_word("wide_product_left");
        self.needs_wide = true;
        self.needs_wide_arithmetic = true;
        self.emit(format!("li $t0, {aligned}"));
        self.emit("subu $sp, $sp, $t0");
        self.emit("move $a0, $sp");
        self.emit(format!("li $a1, {width}"));
        self.emit("li $a2, 48");
        self.emit("jal rt_fill");
        if self.wide_operand(left).is_none() {
            self.emit(format!("li $t0, {aligned}"));
            self.emit("addu $sp, $sp, $t0");
            return true;
        }
        self.emit(format!("sw $a0, {left_address}"));
        if self.wide_operand(right).is_none() {
            self.emit(format!("li $t0, {aligned}"));
            self.emit("addu $sp, $sp, $t0");
            return true;
        }
        self.emit("move $a2, $a0");
        self.emit(format!("lw $a0, {left_address}"));
        self.emit(format!("li $a1, {left_size}"));
        self.emit(format!("li $a3, {right_size}"));
        self.emit("move $t0, $sp");
        self.emit("addiu $sp, $sp, -4");
        self.emit("sw $t0, 0($sp)");
        self.emit("jal rt_wide_product");
        self.emit("addiu $sp, $sp, 4");
        self.store_wide_result(target, width, left_scale + right_scale, 0, aligned);
        true
    }

    pub(super) fn assign_wide_quotient(
        &mut self,
        target: &Expression,
        expression: &Expression,
        remainder: Option<&Expression>,
        saved_divisor: Option<&str>,
    ) -> bool {
        let Expression::BinaryOp {
            left,
            op: BinaryOperator::Divide,
            right,
            ..
        } = expression
        else {
            return false;
        };
        let Some((name, _)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(item)) = self.place(name) else {
            return false;
        };
        if class_of(item.item()) != Class::Plain {
            return false;
        }
        let Some(info) = item.item().picture.clone() else {
            return false;
        };
        if let Some(remainder) = remainder {
            let Some((name, _)) = name_and_indices(remainder) else {
                return false;
            };
            let Some(Place::Item(item)) = self.place(name) else {
                return false;
            };
            if class_of(item.item()) != Class::Plain || item.item().picture.is_none() {
                return false;
            }
        }
        let Some((left_size, left_scale)) = self.wide_shape(left) else {
            return false;
        };
        let Some((right_size, right_scale)) = self.wide_shape(right) else {
            return false;
        };
        if saved_divisor.is_none()
            && !self.is_wide_number(target)
            && !self.is_wide_number(left)
            && !self.is_wide_number(right)
        {
            return false;
        }
        let shift = info.scale + right_scale;
        let numerator_zeros = shift.saturating_sub(left_scale);
        let denominator_zeros = left_scale.saturating_sub(shift);
        let width = (left_size + numerator_zeros).max(right_size + denominator_zeros) + 1;
        let aligned = (width + 3) & !3;
        let allocation = aligned * 4;
        self.needs_wide = true;
        self.needs_wide_arithmetic = true;
        self.emit(format!("li $t0, {allocation}"));
        self.emit("subu $sp, $sp, $t0");
        self.emit("move $a0, $sp");
        self.emit(format!("li $a1, {allocation}"));
        self.emit("li $a2, 48");
        self.emit("jal rt_fill");
        for (operand, size, zeros, offset, saved) in [
            (left.as_ref(), left_size, numerator_zeros, aligned, None),
            (
                right.as_ref(),
                right_size,
                denominator_zeros,
                aligned * 2,
                saved_divisor,
            ),
        ] {
            if let Some(saved) = saved {
                self.emit(format!("lw $a0, {saved}"));
            } else if self.wide_operand(operand).is_none() {
                self.emit(format!("li $t0, {allocation}"));
                self.emit("addu $sp, $sp, $t0");
                return true;
            }
            self.emit(format!("li $a1, {size}"));
            self.emit(format!("li $t0, {offset}"));
            self.emit("addu $a2, $sp, $t0");
            self.emit(format!("li $a3, {width}"));
            self.emit("addiu $sp, $sp, -12");
            self.emit("sw $zero, 0($sp)");
            self.emit(format!("li $t0, {zeros}"));
            self.emit("sw $t0, 4($sp)");
            self.emit("li $t0, 1");
            self.emit("sw $t0, 8($sp)");
            self.emit("jal rt_wide_move");
            self.emit("addiu $sp, $sp, 12");
        }
        self.emit(format!("li $t0, {aligned}"));
        self.emit("addu $a0, $sp, $t0");
        self.emit(format!("li $t0, {}", aligned * 2));
        self.emit("addu $a1, $sp, $t0");
        self.emit("move $a2, $sp");
        self.emit(format!("li $t0, {}", aligned * 3));
        self.emit("addu $a3, $sp, $t0");
        self.emit("addiu $sp, $sp, -4");
        self.emit(format!("li $t0, {width}"));
        self.emit("sw $t0, 0($sp)");
        self.emit("jal rt_wide_divide");
        self.emit("addiu $sp, $sp, 4");
        let divided = self.new_label("wide_divided");
        let done = self.new_label("wide_divide_done");
        self.emit(format!("beqz $v0, {divided}"));
        self.emit("li $t0, 1");
        self.emit("sw $t0, size_err");
        self.emit(format!("li $t0, {allocation}"));
        self.emit("addu $sp, $sp, $t0");
        self.emit(format!("j {done}"));
        self.label(&divided);
        if let Some(remainder) = remainder {
            self.store_wide_result(target, width, info.scale, 0, 0);
            self.store_wide_result(
                remainder,
                width,
                left_scale.max(info.scale + right_scale),
                aligned * 3,
                allocation,
            );
        } else {
            self.store_wide_result(target, width, info.scale, 0, allocation);
        }
        self.label(&done);
        true
    }

    pub(super) fn save_wide_operand(&mut self, operand: &Expression) -> Option<(String, usize)> {
        let (size, _) = self.wide_shape(operand)?;
        self.wide_operand(operand)?;
        let allocation = (size + 3) & !3;
        let address = self.new_word("wide_saved_operand");
        self.emit(format!("li $t0, {allocation}"));
        self.emit("subu $sp, $sp, $t0");
        self.emit("move $a2, $sp");
        self.emit(format!("li $a3, {size}"));
        self.emit("jal rt_copy_pad");
        self.emit(format!("sw $sp, {address}"));
        Some((address, allocation))
    }

    fn store_wide_result(
        &mut self,
        target: &Expression,
        width: usize,
        scale: usize,
        source_offset: usize,
        allocation: usize,
    ) {
        let (name, indices) = name_and_indices(target).expect("numeric target");
        let Some(Place::Item(item)) = self.place(name) else {
            return;
        };
        let Some(info) = item.item().picture.clone() else {
            return;
        };
        let done = self.new_label("wide_result_done");
        if self.guard {
            self.emit("lw $t0, size_err");
            self.emit(format!("bnez $t0, {done}"));
        }
        if self.size_check {
            let check = self.new_label("wide_sum_check");
            let fits = self.new_label("wide_sum_fits");
            self.emit(format!("li $t0, {source_offset}"));
            self.emit("addu $t0, $sp, $t0");
            self.emit(format!(
                "li $t1, {}",
                width
                    .saturating_sub(scale)
                    .saturating_sub(info.integer_digits())
            ));
            self.label(&check);
            self.emit(format!("beqz $t1, {fits}"));
            self.emit("lbu $t2, 0($t0)");
            self.emit("li $t3, 48");
            let overflow = self.new_label("wide_sum_overflow");
            self.emit(format!("bne $t2, $t3, {overflow}"));
            self.emit("addiu $t0, $t0, 1");
            self.emit("addiu $t1, $t1, -1");
            self.emit(format!("j {check}"));
            self.label(&overflow);
            self.emit("li $t0, 1");
            self.emit("sw $t0, size_err");
            self.emit(format!("j {done}"));
            self.label(&fits);
        }
        if self
            .gen_addr(name, indices, expression_span(target))
            .is_some()
        {
            self.emit("move $a2, $a0");
            self.emit(format!("li $t0, {source_offset}"));
            self.emit("addu $a0, $sp, $t0");
            self.emit(format!("li $a1, {width}"));
            self.emit(format!("li $a3, {}", info.size));
            self.emit("addiu $sp, $sp, -12");
            self.emit(format!("li $t0, {scale}"));
            self.emit("sw $t0, 0($sp)");
            self.emit(format!("li $t0, {}", info.scale));
            self.emit("sw $t0, 4($sp)");
            self.emit(format!("li $t0, {}", u8::from(info.signed)));
            self.emit("sw $t0, 8($sp)");
            self.emit("jal rt_wide_move");
            self.emit("addiu $sp, $sp, 12");
        }
        self.label(&done);
        if allocation > 0 {
            self.emit(format!("li $t0, {allocation}"));
            self.emit("addu $sp, $sp, $t0");
        }
    }

    pub(super) fn gen_stored_quotient(&mut self, target: &Expression) -> u32 {
        let Some((name, indices)) = name_and_indices(target) else {
            return self.gen_num(target);
        };
        let Some(Place::Item(item)) = self.place(name) else {
            return self.gen_num(target);
        };
        let Some(info) = item.item().picture.clone() else {
            return self.gen_num(target);
        };
        if class_of(item.item()) != Class::Plain || info.digits <= 9 {
            return self.gen_num(target);
        }
        // The quotient was just stored from a 32-bit result; read it back to
        // preserve truncation to the receiving picture before the remainder.
        if self
            .gen_addr(name, indices, expression_span(target))
            .is_some()
        {
            self.emit(format!("li $a1, {}", info.size));
            self.emit("jal rt_num_load");
        }
        info.scale as u32
    }

    fn wide_operand(&mut self, expression: &Expression) -> Option<(usize, usize)> {
        if let Some(value) = literal_number(expression) {
            let negative = value.starts_with('-');
            let unsigned = value.trim_start_matches(['+', '-']);
            let scale = unsigned.split_once('.').map_or(0, |(_, frac)| frac.len());
            let mut digits: Vec<u8> = unsigned.bytes().filter(u8::is_ascii_digit).collect();
            if negative && let Some(last) = digits.last_mut() {
                *last += 0x40;
            }
            let size = digits.len();
            let label = self.literal(&digits);
            self.emit(format!("la $a0, {label}"));
            self.emit(format!("li $a1, {size}"));
            return Some((size, scale));
        }
        let Some((name, indices)) = name_and_indices(expression) else {
            self.unsupported(
                expression_span(expression),
                "wide numeric comparison operand",
            );
            return None;
        };
        let Some(Place::Item(resolved)) = self.place(name) else {
            self.unsupported(
                expression_span(expression),
                "wide numeric comparison operand",
            );
            return None;
        };
        if class_of(resolved.item()) != Class::Plain {
            self.unsupported(
                expression_span(expression),
                "wide numeric comparison operand",
            );
            return None;
        }
        let Some(info) = resolved.item().picture.clone() else {
            self.unsupported(
                expression_span(expression),
                "wide numeric comparison operand",
            );
            return None;
        };
        self.gen_addr(name, indices, expression_span(expression))?;
        self.emit(format!("li $a1, {}", info.size));
        Some((info.size, info.scale))
    }

    pub(super) fn move_wide_edited(&mut self, source: &Expression, target: &Expression) -> bool {
        if !self.is_wide_number(source) {
            return false;
        }
        let Some((name, indices)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(item)) = self.place(name) else {
            return false;
        };
        if class_of(item.item()) != Class::Edited {
            return false;
        }
        let Some(info) = item.item().picture.clone() else {
            return false;
        };
        if info.digits == 0 {
            self.unsupported(
                expression_span(target),
                "numeric editing without digit positions",
            );
            return true;
        }
        self.edit_digits_size = self.edit_digits_size.max(info.digits);
        self.needs_wide = true;
        let Some((source_size, source_scale)) = self.wide_operand(source) else {
            return true;
        };
        self.emit("addiu $sp, $sp, -8");
        self.emit("sw $a0, 0($sp)");
        if self
            .gen_addr(name, indices, expression_span(target))
            .is_none()
        {
            self.emit("addiu $sp, $sp, 8");
            return true;
        }
        self.emit("sw $a0, 4($sp)");
        self.emit("lw $a0, 0($sp)");
        self.emit(format!("li $a1, {source_size}"));
        self.emit("la $a2, ed_digits");
        self.emit(format!("li $a3, {}", info.digits));
        self.emit("addiu $sp, $sp, -12");
        self.emit(format!("li $t0, {source_scale}"));
        self.emit("sw $t0, 0($sp)");
        self.emit(format!("li $t0, {}", info.scale));
        self.emit("sw $t0, 4($sp)");
        self.emit("li $t0, 1");
        self.emit("sw $t0, 8($sp)");
        self.emit("jal rt_wide_move");
        self.emit("addiu $sp, $sp, 12");
        self.emit("lw $a0, 4($sp)");
        self.emit("addiu $sp, $sp, 8");
        self.emit("la $t4, ed_digits");
        self.emit(format!("li $t1, {}", info.digits - 1));
        self.emit("addu $t1, $t4, $t1");
        self.emit("lbu $t0, 0($t1)");
        self.emit("sltiu $t9, $t0, 0x70");
        self.emit("xori $t9, $t9, 1");
        let positive = self.new_label("wide_edit_positive");
        self.emit(format!("beqz $t9, {positive}"));
        self.emit("addiu $t0, $t0, -0x40");
        self.emit("sb $t0, 0($t1)");
        self.label(&positive);
        let convert = self.new_label("wide_edit_convert");
        self.emit("move $t1, $t4");
        self.emit(format!("li $t2, {}", info.digits));
        self.label(&convert);
        self.emit("lbu $t0, 0($t1)");
        self.emit("addiu $t0, $t0, -48");
        self.emit("sb $t0, 0($t1)");
        self.emit("addiu $t1, $t1, 1");
        self.emit("addiu $t2, $t2, -1");
        self.emit(format!("bnez $t2, {convert}"));
        let pattern = self.literal(info.pattern.as_bytes());
        self.emit(format!("la $a1, {pattern}"));
        self.emit("jal rt_edit_walk");
        true
    }

    pub(super) fn wide_relation(
        &mut self,
        left: &Expression,
        op: BinaryOperator,
        right: &Expression,
        label: &str,
        jump_if: bool,
        span: Span,
    ) -> bool {
        if !self.is_wide_number(left) && !self.is_wide_number(right) {
            return false;
        }
        self.needs_wide = true;
        let Some((_, left_scale)) = self.wide_operand(left) else {
            return true;
        };
        self.emit("addiu $sp, $sp, -8");
        self.emit("sw $a0, 0($sp)");
        self.emit("sw $a1, 4($sp)");
        let Some((_, right_scale)) = self.wide_operand(right) else {
            self.emit("addiu $sp, $sp, 8");
            return true;
        };
        self.emit("move $a2, $a0");
        self.emit("move $a3, $a1");
        self.emit("lw $a0, 0($sp)");
        self.emit("lw $a1, 4($sp)");
        self.emit("addiu $sp, $sp, 8");
        self.emit("addiu $sp, $sp, -8");
        self.emit(format!("li $t0, {left_scale}"));
        self.emit("sw $t0, 0($sp)");
        self.emit(format!("li $t0, {right_scale}"));
        self.emit("sw $t0, 4($sp)");
        self.emit("jal rt_wide_compare");
        self.emit("addiu $sp, $sp, 8");
        let branch = match (op, jump_if) {
            (BinaryOperator::Equal, true) | (BinaryOperator::NotEqual, false) => "beqz",
            (BinaryOperator::NotEqual, true) | (BinaryOperator::Equal, false) => "bnez",
            (BinaryOperator::LessThan, true) | (BinaryOperator::GreaterThanOrEqual, false) => {
                "bltz"
            }
            (BinaryOperator::LessThanOrEqual, true) | (BinaryOperator::GreaterThan, false) => {
                "blez"
            }
            (BinaryOperator::GreaterThan, true) | (BinaryOperator::LessThanOrEqual, false) => {
                "bgtz"
            }
            (BinaryOperator::GreaterThanOrEqual, true) | (BinaryOperator::LessThan, false) => {
                "bgez"
            }
            _ => {
                self.unsupported(span, "wide numeric comparison operator");
                return true;
            }
        };
        self.emit(format!("{branch} $v0, {label}"));
        true
    }

    /// Returns true if this move was emitted without 32-bit numeric conversion.
    pub(super) fn move_wide(&mut self, source: &Expression, target: &Expression) -> bool {
        let Some((target_name, target_indices)) = name_and_indices(target) else {
            return false;
        };
        let Some(Place::Item(target_item)) = self.place(target_name) else {
            return false;
        };
        if class_of(target_item.item()) != Class::Plain {
            return false;
        }
        let Some(target_info) = target_item.item().picture.clone() else {
            return false;
        };
        if let Some(value) = literal_number(source) {
            let digits = value.bytes().filter(u8::is_ascii_digit).count();
            if target_info.digits <= 9 && digits <= 9 {
                return false;
            }
            let bytes = encode_numeric(&target_info, &value);
            let label = self.literal(&bytes);
            if self
                .gen_addr(target_name, target_indices, expression_span(target))
                .is_some()
            {
                self.emit("move $a2, $a0");
                self.emit(format!("li $a3, {}", target_info.size));
                self.emit(format!("la $a0, {label}"));
                self.emit(format!("li $a1, {}", bytes.len()));
                self.emit("jal rt_copy_pad");
            }
            return true;
        }
        let Some((source_name, source_indices)) = name_and_indices(source) else {
            return false;
        };
        let Some(Place::Item(source_item)) = self.place(source_name) else {
            return false;
        };
        if class_of(source_item.item()) != Class::Plain {
            return false;
        }
        let Some(source_info) = source_item.item().picture.clone() else {
            return false;
        };
        if source_info.digits <= 9 && target_info.digits <= 9 {
            return false;
        }
        self.needs_wide = true;
        if self
            .gen_addr(source_name, source_indices, expression_span(source))
            .is_none()
        {
            return true;
        }
        self.emit("addiu $sp, $sp, -4");
        self.emit("sw $a0, 0($sp)");
        if self
            .gen_addr(target_name, target_indices, expression_span(target))
            .is_none()
        {
            self.emit("addiu $sp, $sp, 4");
            return true;
        }
        self.emit("move $a2, $a0");
        self.emit("lw $a0, 0($sp)");
        self.emit("addiu $sp, $sp, 4");
        self.emit(format!("li $a1, {}", source_info.size));
        self.emit(format!("li $a3, {}", target_info.size));
        self.emit("addiu $sp, $sp, -12");
        self.emit(format!("li $t0, {}", source_info.scale));
        self.emit("sw $t0, 0($sp)");
        self.emit(format!("li $t0, {}", target_info.scale));
        self.emit("sw $t0, 4($sp)");
        self.emit(format!("li $t0, {}", u8::from(target_info.signed)));
        self.emit("sw $t0, 8($sp)");
        self.emit("jal rt_wide_move");
        self.emit("addiu $sp, $sp, 12");
        true
    }
}

pub(super) const ARITHMETIC_RUNTIME: &str = r#"
# Divide equal-width DISPLAY magnitudes with decimal long division.
# $a0=numerator, $a1=denominator, $a2=quotient, $a3=remainder; width on stack.
# Returns 1 for division by zero.
rt_wide_divide:
    lw $t0, 0($sp)
    addiu $sp, $sp, -40
    sw $s0, 0($sp)
    sw $s1, 4($sp)
    sw $s2, 8($sp)
    sw $s3, 12($sp)
    sw $s4, 16($sp)
    sw $s5, 20($sp)
    sw $s6, 24($sp)
    sw $s7, 28($sp)
    move $s0, $a0
    move $s1, $a1
    move $s2, $a2
    move $s3, $a3
    move $s4, $t0
    addu $t0, $s0, $s4
    lbu $t1, -1($t0)
    sltiu $t2, $t1, 0x70
    xori $t4, $t2, 1
    sw $t4, 36($sp)
    bnez $t2, rt_wide_divide_numerator_positive
    addiu $t1, $t1, -0x40
    sb $t1, -1($t0)
rt_wide_divide_numerator_positive:
    addu $t0, $s1, $s4
    lbu $t1, -1($t0)
    sltiu $t3, $t1, 0x70
    bnez $t3, rt_wide_divide_denominator_positive
    addiu $t1, $t1, -0x40
    sb $t1, -1($t0)
rt_wide_divide_denominator_positive:
    xor $t2, $t2, $t3
    sw $t2, 32($sp)
    move $t0, $zero
rt_wide_divide_check_zero:
    beq $t0, $s4, rt_wide_divide_zero
    addu $t1, $s1, $t0
    lbu $t2, 0($t1)
    li $t3, 48
    bne $t2, $t3, rt_wide_divide_begin
    addiu $t0, $t0, 1
    j rt_wide_divide_check_zero
rt_wide_divide_begin:
    move $s5, $zero
rt_wide_divide_next_digit:
    beq $s5, $s4, rt_wide_divide_sign
    move $s6, $zero
    addiu $t3, $s4, -1
rt_wide_divide_shift:
    beq $s6, $t3, rt_wide_divide_append
    addu $t0, $s3, $s6
    lbu $t1, 1($t0)
    sb $t1, 0($t0)
    addiu $s6, $s6, 1
    j rt_wide_divide_shift
rt_wide_divide_append:
    addu $t0, $s0, $s5
    lbu $t1, 0($t0)
    addu $t0, $s3, $s4
    sb $t1, -1($t0)
    move $s7, $zero
rt_wide_divide_compare:
    move $s6, $zero
rt_wide_divide_compare_loop:
    beq $s6, $s4, rt_wide_divide_subtract
    addu $t0, $s3, $s6
    lbu $t1, 0($t0)
    addu $t0, $s1, $s6
    lbu $t2, 0($t0)
    bltu $t1, $t2, rt_wide_divide_store_digit
    bgtu $t1, $t2, rt_wide_divide_subtract
    addiu $s6, $s6, 1
    j rt_wide_divide_compare_loop
rt_wide_divide_subtract:
    move $s6, $s4
    move $t4, $zero
rt_wide_divide_subtract_loop:
    beqz $s6, rt_wide_divide_subtracted
    addiu $s6, $s6, -1
    addu $t0, $s3, $s6
    lbu $t1, 0($t0)
    addu $t2, $s1, $s6
    lbu $t2, 0($t2)
    subu $t1, $t1, $t2
    subu $t1, $t1, $t4
    move $t4, $zero
    bgez $t1, rt_wide_divide_subtract_digit
    addiu $t1, $t1, 10
    li $t4, 1
rt_wide_divide_subtract_digit:
    addiu $t1, $t1, 48
    sb $t1, 0($t0)
    j rt_wide_divide_subtract_loop
rt_wide_divide_subtracted:
    addiu $s7, $s7, 1
    j rt_wide_divide_compare
rt_wide_divide_store_digit:
    addu $t0, $s2, $s5
    addiu $t1, $s7, 48
    sb $t1, 0($t0)
    addiu $s5, $s5, 1
    j rt_wide_divide_next_digit
rt_wide_divide_sign:
    lw $t0, 36($sp)
    beqz $t0, rt_wide_divide_quotient_sign
    move $t0, $zero
rt_wide_divide_remainder_nonzero:
    beq $t0, $s4, rt_wide_divide_quotient_sign
    addu $t1, $s3, $t0
    lbu $t2, 0($t1)
    li $t3, 48
    bne $t2, $t3, rt_wide_divide_remainder_negative
    addiu $t0, $t0, 1
    j rt_wide_divide_remainder_nonzero
rt_wide_divide_remainder_negative:
    addu $t0, $s3, $s4
    lbu $t1, -1($t0)
    addiu $t1, $t1, 0x40
    sb $t1, -1($t0)
rt_wide_divide_quotient_sign:
    lw $t0, 32($sp)
    beqz $t0, rt_wide_divide_success
    move $t0, $zero
rt_wide_divide_nonzero:
    beq $t0, $s4, rt_wide_divide_success
    addu $t1, $s2, $t0
    lbu $t2, 0($t1)
    li $t3, 48
    bne $t2, $t3, rt_wide_divide_negative
    addiu $t0, $t0, 1
    j rt_wide_divide_nonzero
rt_wide_divide_negative:
    addu $t0, $s2, $s4
    lbu $t1, -1($t0)
    addiu $t1, $t1, 0x40
    sb $t1, -1($t0)
rt_wide_divide_success:
    move $v0, $zero
    j rt_wide_divide_return
rt_wide_divide_zero:
    li $v0, 1
rt_wide_divide_return:
    lw $s0, 0($sp)
    lw $s1, 4($sp)
    lw $s2, 8($sp)
    lw $s3, 12($sp)
    lw $s4, 16($sp)
    lw $s5, 20($sp)
    lw $s6, 24($sp)
    lw $s7, 28($sp)
    addiu $sp, $sp, 40
    jr $ra

# Multiply signed DISPLAY operands into an initialized digit buffer.
# $a0=left, $a1=left width, $a2=right, $a3=right width; result on stack.
rt_wide_product:
    lw $t0, 0($sp)
    addiu $sp, $sp, -36
    sw $s0, 0($sp)
    sw $s1, 4($sp)
    sw $s2, 8($sp)
    sw $s3, 12($sp)
    sw $s4, 16($sp)
    sw $s5, 20($sp)
    sw $s6, 24($sp)
    sw $s7, 28($sp)
    move $s0, $a0
    move $s1, $a1
    move $s2, $a2
    move $s3, $a3
    move $s4, $t0
    addu $t0, $s0, $s1
    lbu $t1, -1($t0)
    sltiu $t1, $t1, 0x70
    addu $t0, $s2, $s3
    lbu $t2, -1($t0)
    sltiu $t2, $t2, 0x70
    xor $t1, $t1, $t2
    sw $t1, 32($sp)
    addiu $s5, $s1, -1
rt_wide_product_outer:
    bltz $s5, rt_wide_product_sign
    addiu $s6, $s3, -1
    move $s7, $zero
rt_wide_product_inner:
    bltz $s6, rt_wide_product_carry
    addu $t0, $s0, $s5
    lbu $t1, 0($t0)
    addiu $t0, $s1, -1
    bne $s5, $t0, rt_wide_product_left_digit
    sltiu $t0, $t1, 0x70
    bnez $t0, rt_wide_product_left_digit
    addiu $t1, $t1, -0x40
rt_wide_product_left_digit:
    addiu $t1, $t1, -48
    addu $t0, $s2, $s6
    lbu $t2, 0($t0)
    addiu $t0, $s3, -1
    bne $s6, $t0, rt_wide_product_right_digit
    sltiu $t0, $t2, 0x70
    bnez $t0, rt_wide_product_right_digit
    addiu $t2, $t2, -0x40
rt_wide_product_right_digit:
    addiu $t2, $t2, -48
    mul $t1, $t1, $t2
    addu $t0, $s5, $s6
    addu $t0, $t0, $s4
    addiu $t0, $t0, 1
    lbu $t2, 0($t0)
    addiu $t2, $t2, -48
    addu $t1, $t1, $t2
    addu $t1, $t1, $s7
    li $t2, 10
    divu $t1, $t2
    mflo $s7
    mfhi $t1
    addiu $t1, $t1, 48
    sb $t1, 0($t0)
    addiu $s6, $s6, -1
    j rt_wide_product_inner
rt_wide_product_carry:
    addu $t0, $s4, $s5
    addiu $t1, $s7, 48
    sb $t1, 0($t0)
    addiu $s5, $s5, -1
    j rt_wide_product_outer
rt_wide_product_sign:
    lw $t0, 32($sp)
    beqz $t0, rt_wide_product_return
    addu $t0, $s1, $s3
    move $t1, $zero
rt_wide_product_nonzero:
    beq $t1, $t0, rt_wide_product_return
    addu $t2, $s4, $t1
    lbu $t3, 0($t2)
    li $t4, 48
    bne $t3, $t4, rt_wide_product_negative
    addiu $t1, $t1, 1
    j rt_wide_product_nonzero
rt_wide_product_negative:
    addu $t2, $s4, $t0
    lbu $t3, -1($t2)
    addiu $t3, $t3, 0x40
    sb $t3, -1($t2)
rt_wide_product_return:
    lw $s0, 0($sp)
    lw $s1, 4($sp)
    lw $s2, 8($sp)
    lw $s3, 12($sp)
    lw $s4, 16($sp)
    lw $s5, 20($sp)
    lw $s6, 24($sp)
    lw $s7, 28($sp)
    addiu $sp, $sp, 36
    jr $ra

# Add or subtract equal-width signed DISPLAY magnitudes in place.
# $a0=accumulator, $a1=operand, $a2=width, $a3=1 for subtraction.
rt_wide_add:
    addu $t0, $a0, $a2
    addu $t1, $a1, $a2
    lbu $t2, -1($t0)
    lbu $t3, -1($t1)
    sltiu $t4, $t2, 0x70
    xori $t4, $t4, 1
    sltiu $t5, $t3, 0x70
    xori $t5, $t5, 1
    beqz $t4, rt_wide_add_first_positive
    addiu $t2, $t2, -0x40
    sb $t2, -1($t0)
rt_wide_add_first_positive:
    beqz $t5, rt_wide_add_second_positive
    addiu $t3, $t3, -0x40
    sb $t3, -1($t1)
rt_wide_add_second_positive:
    xor $t5, $t5, $a3
    move $t0, $a0
    move $t1, $a1
    beq $t4, $t5, rt_wide_add_same_sign
    move $t7, $zero
rt_wide_add_compare:
    beq $t7, $a2, rt_wide_add_first_larger
    addu $t2, $t0, $t7
    lbu $t8, 0($t2)
    addu $t2, $t1, $t7
    lbu $t9, 0($t2)
    bgtu $t8, $t9, rt_wide_add_first_larger
    bltu $t8, $t9, rt_wide_add_second_larger
    addiu $t7, $t7, 1
    j rt_wide_add_compare
rt_wide_add_second_larger:
    move $t0, $a1
    move $t1, $a0
    move $a3, $t5
    j rt_wide_add_subtract
rt_wide_add_first_larger:
    move $a3, $t4
rt_wide_add_subtract:
    move $t6, $zero
    move $t7, $a2
rt_wide_add_subtract_loop:
    beqz $t7, rt_wide_add_sign
    addiu $t7, $t7, -1
    addu $t2, $t0, $t7
    lbu $t8, 0($t2)
    addu $t2, $t1, $t7
    lbu $t9, 0($t2)
    subu $t8, $t8, $t9
    subu $t8, $t8, $t6
    move $t6, $zero
    bgez $t8, rt_wide_add_subtract_digit
    addiu $t8, $t8, 10
    li $t6, 1
rt_wide_add_subtract_digit:
    addiu $t8, $t8, 48
    addu $t2, $a0, $t7
    sb $t8, 0($t2)
    j rt_wide_add_subtract_loop
rt_wide_add_same_sign:
    move $a3, $t4
    move $t6, $zero
    move $t7, $a2
rt_wide_add_sum_loop:
    beqz $t7, rt_wide_add_sign
    addiu $t7, $t7, -1
    addu $t2, $t0, $t7
    lbu $t8, 0($t2)
    addu $t2, $t1, $t7
    lbu $t9, 0($t2)
    addu $t8, $t8, $t9
    addu $t8, $t8, $t6
    addiu $t8, $t8, -96
    move $t6, $zero
    sltiu $t2, $t8, 10
    bnez $t2, rt_wide_add_sum_digit
    addiu $t8, $t8, -10
    li $t6, 1
rt_wide_add_sum_digit:
    addiu $t8, $t8, 48
    addu $t2, $a0, $t7
    sb $t8, 0($t2)
    j rt_wide_add_sum_loop
rt_wide_add_sign:
    beqz $a3, rt_wide_add_done
    move $t7, $zero
rt_wide_add_nonzero:
    beq $t7, $a2, rt_wide_add_done
    addu $t2, $a0, $t7
    lbu $t8, 0($t2)
    li $t9, 48
    bne $t8, $t9, rt_wide_add_negative
    addiu $t7, $t7, 1
    j rt_wide_add_nonzero
rt_wide_add_negative:
    addu $t2, $a0, $a2
    lbu $t8, -1($t2)
    addiu $t8, $t8, 0x40
    sb $t8, -1($t2)
rt_wide_add_done:
    jr $ra
"#;

pub(super) const RUNTIME: &str = r#"
# Format a 32-bit scaled value as text in fn_buf.
rt_wide_format:
    move $t0, $a0
    slt $t1, $t0, $zero
    beqz $t1, rt_wide_format_positive
    negu $t0, $t0
rt_wide_format_positive:
    la $t2, fn_buf
    addiu $t2, $t2, 255
    sb $zero, 0($t2)
    move $t3, $zero
    li $t4, 10
rt_wide_format_loop:
    divu $t0, $t4
    mflo $t0
    mfhi $t5
    addiu $t5, $t5, 48
    addiu $t2, $t2, -1
    sb $t5, 0($t2)
    addiu $t3, $t3, 1
    beqz $a1, rt_wide_format_continue
    bne $t3, $a1, rt_wide_format_continue
    addiu $t2, $t2, -1
    li $t5, 46
    sb $t5, 0($t2)
rt_wide_format_continue:
    bnez $t0, rt_wide_format_loop
    ble $t3, $a1, rt_wide_format_loop
    beqz $t1, rt_wide_format_done
    addiu $t2, $t2, -1
    li $t5, 45
    sb $t5, 0($t2)
rt_wide_format_done:
    move $a0, $t2
    la $a1, fn_buf
    addiu $a1, $a1, 255
    subu $a1, $a1, $t2
    jr $ra

# Increment an unsigned integer field, optionally preserving it on overflow.
# $a0=address, $a1=size, $a2=check. Returns 1 on overflow.
rt_wide_increment:
    addu $t0, $a0, $a1
    addiu $t0, $t0, -1
    move $t1, $a1
rt_wide_increment_scan:
    beqz $t1, rt_wide_increment_overflow
    lbu $t2, 0($t0)
    li $t3, 57
    bne $t2, $t3, rt_wide_increment_carry
    addiu $t0, $t0, -1
    addiu $t1, $t1, -1
    j rt_wide_increment_scan
rt_wide_increment_overflow:
    li $v0, 1
    bnez $a2, rt_wide_increment_return
    move $t0, $a0
    addiu $t0, $t0, -1
    j rt_wide_increment_zero
rt_wide_increment_carry:
    addiu $t2, $t2, 1
    sb $t2, 0($t0)
    move $v0, $zero
rt_wide_increment_zero:
    addiu $t0, $t0, 1
    addu $t1, $a0, $a1
rt_wide_increment_zero_loop:
    beq $t0, $t1, rt_wide_increment_return
    li $t2, 48
    sb $t2, 0($t0)
    addiu $t0, $t0, 1
    j rt_wide_increment_zero_loop
rt_wide_increment_return:
    jr $ra

# Read a numeric line and align its DISPLAY digits into the target.
rt_wide_accept:
    addiu $sp, $sp, -36
    sw $s0, 0($sp)
    sw $s1, 4($sp)
    sw $s2, 8($sp)
    sw $s3, 12($sp)
    sw $s4, 16($sp)
    sw $s5, 20($sp)
    sw $s6, 24($sp)
    sw $s7, 28($sp)
    sw $ra, 32($sp)
    move $s0, $a0
    move $s1, $a1
    move $s2, $a2
    move $s3, $a3
    move $s4, $zero
    move $s5, $zero
    move $s6, $zero
    move $s7, $zero
    la $a0, tmp_buf
    li $a1, 255
    li $v0, 8
    syscall
    la $t0, tmp_buf
    la $t1, fn_buf
rt_wide_accept_loop:
    lbu $t2, 0($t0)
    beqz $t2, rt_wide_accept_end
    li $t3, 45
    bne $t2, $t3, rt_wide_accept_point
    li $s4, 1
    j rt_wide_accept_next
rt_wide_accept_point:
    li $t3, 46
    bne $t2, $t3, rt_wide_accept_digit
    li $s6, 1
    j rt_wide_accept_next
rt_wide_accept_digit:
    addiu $t3, $t2, -48
    sltiu $t3, $t3, 10
    beqz $t3, rt_wide_accept_next
    sb $t2, 0($t1)
    addiu $t1, $t1, 1
    addiu $s7, $s7, 1
    beqz $s6, rt_wide_accept_next
    addiu $s5, $s5, 1
rt_wide_accept_next:
    addiu $t0, $t0, 1
    j rt_wide_accept_loop
rt_wide_accept_end:
    bnez $s7, rt_wide_accept_sign
    li $t2, 48
    sb $t2, 0($t1)
    addiu $t1, $t1, 1
    li $s7, 1
rt_wide_accept_sign:
    beqz $s4, rt_wide_accept_move
    lbu $t2, -1($t1)
    addiu $t2, $t2, 0x40
    sb $t2, -1($t1)
rt_wide_accept_move:
    la $a0, fn_buf
    move $a1, $s7
    move $a2, $s0
    move $a3, $s1
    addiu $sp, $sp, -12
    sw $s5, 0($sp)
    sw $s2, 4($sp)
    sw $s3, 8($sp)
    jal rt_wide_move
    addiu $sp, $sp, 12
    lw $s0, 0($sp)
    lw $s1, 4($sp)
    lw $s2, 8($sp)
    lw $s3, 12($sp)
    lw $s4, 16($sp)
    lw $s5, 20($sp)
    lw $s6, 24($sp)
    lw $s7, 28($sp)
    lw $ra, 32($sp)
    addiu $sp, $sp, 36
    jr $ra

# Compare two DISPLAY values, with scales on the stack. Returns -1, 0 or 1.
rt_wide_compare:
    lw $t0, 0($sp)
    lw $t1, 4($sp)
    addiu $sp, $sp, -32
    sw $s0, 0($sp)
    sw $s1, 4($sp)
    sw $s2, 8($sp)
    sw $s3, 12($sp)
    sw $s4, 16($sp)
    sw $s5, 20($sp)
    sw $s6, 24($sp)
    sw $s7, 28($sp)
    move $s0, $a0
    move $s1, $a1
    move $s2, $a2
    move $s3, $a3
    move $s4, $t0
    move $s5, $t1
    addu $t0, $s0, $s1
    lbu $t0, -1($t0)
    sltiu $t0, $t0, 0x70
    xori $s6, $t0, 1
    addu $t0, $s2, $s3
    lbu $t0, -1($t0)
    sltiu $t0, $t0, 0x70
    xori $s7, $t0, 1
    beqz $s6, rt_wide_compare_sign_right
    move $t0, $zero
rt_wide_compare_zero_left:
    beq $t0, $s1, rt_wide_compare_clear_left
    addu $t1, $s0, $t0
    lbu $t1, 0($t1)
    sltiu $t2, $t1, 0x70
    bnez $t2, rt_wide_compare_zero_left_digit
    addiu $t1, $t1, -0x40
rt_wide_compare_zero_left_digit:
    li $t2, 48
    bne $t1, $t2, rt_wide_compare_sign_right
    addiu $t0, $t0, 1
    j rt_wide_compare_zero_left
rt_wide_compare_clear_left:
    move $s6, $zero
rt_wide_compare_sign_right:
    beqz $s7, rt_wide_compare_signs
    move $t0, $zero
rt_wide_compare_zero_right:
    beq $t0, $s3, rt_wide_compare_clear_right
    addu $t1, $s2, $t0
    lbu $t1, 0($t1)
    sltiu $t2, $t1, 0x70
    bnez $t2, rt_wide_compare_zero_right_digit
    addiu $t1, $t1, -0x40
rt_wide_compare_zero_right_digit:
    li $t2, 48
    bne $t1, $t2, rt_wide_compare_signs
    addiu $t0, $t0, 1
    j rt_wide_compare_zero_right
rt_wide_compare_clear_right:
    move $s7, $zero
rt_wide_compare_signs:
    beq $s6, $s7, rt_wide_compare_align
    li $v0, 1
    beqz $s6, rt_wide_compare_return
    li $v0, -1
    j rt_wide_compare_return
rt_wide_compare_align:
    subu $t0, $s1, $s4
    subu $t1, $s3, $s5
    move $t2, $t0
    bge $t0, $t1, rt_wide_compare_int_max
    move $t2, $t1
rt_wide_compare_int_max:
    subu $t0, $t2, $t0
    subu $t1, $t2, $t1
    move $t3, $s4
    bge $s4, $s5, rt_wide_compare_scale_max
    move $t3, $s5
rt_wide_compare_scale_max:
    addu $t3, $t3, $t2
    move $t4, $zero
rt_wide_compare_loop:
    beq $t4, $t3, rt_wide_compare_equal
    subu $t5, $t4, $t0
    li $t6, 48
    bltz $t5, rt_wide_compare_right
    bge $t5, $s1, rt_wide_compare_right
    addu $t5, $s0, $t5
    lbu $t6, 0($t5)
    sltiu $t7, $t6, 0x70
    bnez $t7, rt_wide_compare_right
    addiu $t6, $t6, -0x40
rt_wide_compare_right:
    subu $t5, $t4, $t1
    li $t7, 48
    bltz $t5, rt_wide_compare_digits
    bge $t5, $s3, rt_wide_compare_digits
    addu $t5, $s2, $t5
    lbu $t7, 0($t5)
    sltiu $t8, $t7, 0x70
    bnez $t8, rt_wide_compare_digits
    addiu $t7, $t7, -0x40
rt_wide_compare_digits:
    blt $t6, $t7, rt_wide_compare_less
    bgt $t6, $t7, rt_wide_compare_greater
    addiu $t4, $t4, 1
    j rt_wide_compare_loop
rt_wide_compare_less:
    li $v0, -1
    j rt_wide_compare_invert
rt_wide_compare_greater:
    li $v0, 1
rt_wide_compare_invert:
    beqz $s6, rt_wide_compare_return
    negu $v0, $v0
    j rt_wide_compare_return
rt_wide_compare_equal:
    move $v0, $zero
rt_wide_compare_return:
    lw $s0, 0($sp)
    lw $s1, 4($sp)
    lw $s2, 8($sp)
    lw $s3, 12($sp)
    lw $s4, 16($sp)
    lw $s5, 20($sp)
    lw $s6, 24($sp)
    lw $s7, 28($sp)
    addiu $sp, $sp, 32
    jr $ra

# Move DISPLAY digits with decimal alignment. Scales and destination sign
# flag are passed on the stack; source is copied first to permit overlap.
rt_wide_move:
    lw $t0, 0($sp)
    lw $t1, 4($sp)
    lw $t2, 8($sp)
    subu $t3, $a1, $t0
    subu $t4, $a3, $t1
    subu $t3, $t3, $t4
    addiu $t4, $a1, 3
    li $t5, -4
    and $t4, $t4, $t5
    subu $sp, $sp, $t4
    move $t5, $zero
    move $t6, $zero
rt_wide_copy_source:
    beq $t5, $a1, rt_wide_move_digits
    addu $t7, $a0, $t5
    lbu $t8, 0($t7)
    addiu $t9, $a1, -1
    bne $t5, $t9, rt_wide_copy_source_byte
    sltiu $t9, $t8, 0x70
    bnez $t9, rt_wide_copy_source_byte
    addiu $t8, $t8, -0x40
    li $t6, 1
rt_wide_copy_source_byte:
    addu $t7, $sp, $t5
    sb $t8, 0($t7)
    addiu $t5, $t5, 1
    j rt_wide_copy_source
rt_wide_move_digits:
    move $t5, $zero
rt_wide_move_loop:
    beq $t5, $a3, rt_wide_move_done
    addu $t7, $t5, $t3
    li $t8, 48
    bltz $t7, rt_wide_move_store
    bge $t7, $a1, rt_wide_move_store
    addu $t7, $sp, $t7
    lbu $t8, 0($t7)
rt_wide_move_store:
    beqz $t2, rt_wide_move_unsigned
    beqz $t6, rt_wide_move_unsigned
    addiu $t7, $a3, -1
    bne $t5, $t7, rt_wide_move_unsigned
    addiu $t8, $t8, 0x40
rt_wide_move_unsigned:
    addu $t7, $a2, $t5
    sb $t8, 0($t7)
    addiu $t5, $t5, 1
    j rt_wide_move_loop
rt_wide_move_done:
    addu $sp, $sp, $t4
    jr $ra
"#;
