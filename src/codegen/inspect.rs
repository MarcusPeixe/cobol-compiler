//! INSPECT code generation.

use super::expr::{Place, name_and_indices};
use super::{Generator, expression_span};
use crate::parser::{
    Expression, InspectConvert, InspectMode, InspectRegion, InspectReplace, InspectStatement,
};

impl Generator<'_> {
    fn inspect_length(&self, expression: &Expression) -> Option<usize> {
        match expression {
            Expression::StringLiteral(text, _) | Expression::NumericLiteral(text, _) => {
                Some(text.len())
            }
            Expression::FigurativeConstant(..) => Some(1),
            _ => {
                let (name, _) = name_and_indices(expression)?;
                match self.place(name)? {
                    Place::Item(resolved) => Some(resolved.item().element_size),
                    Place::Index(_) => None,
                }
            }
        }
    }

    fn inspect_capacity(&self, expression: &Expression) -> Option<usize> {
        if let Expression::RefMod { name, .. } = expression {
            return match self.place(name)? {
                Place::Item(resolved) => Some(resolved.item().element_size),
                Place::Index(_) => None,
            };
        }
        self.inspect_length(expression)
    }

    fn inspect_invalid(&mut self, invalid: &str, valid: &str) {
        self.emit(format!("j {valid}"));
        self.label(invalid);
        let message = self.literal(b"Invalid INSPECT operand length or reference modification\n");
        self.emit(format!("la $a0, {message}"));
        self.emit("li $v0, 4");
        self.emit("syscall");
        self.emit("li $v0, 10");
        self.emit("syscall");
        self.label(valid);
    }

    fn inspect_text(&mut self, expression: &Expression) -> bool {
        let Some(capacity) = self.inspect_capacity(expression).filter(|size| *size > 0) else {
            self.unsupported(
                expression_span(expression),
                "empty or unsupported INSPECT operand",
            );
            return false;
        };
        if !self.gen_text(expression) {
            return false;
        }
        if let Expression::RefMod { name, span, .. } = expression {
            self.emit("addiu $sp, $sp, -8");
            self.emit("sw $a0, 0($sp)");
            self.emit("sw $a1, 4($sp)");
            if self.gen_addr(name, &[], *span).is_none() {
                return false;
            }
            self.emit("move $t0, $a0");
            self.emit("lw $a0, 0($sp)");
            self.emit("lw $a1, 4($sp)");
            self.emit("addiu $sp, $sp, 8");
            let invalid = self.new_label("inspect_invalid");
            let valid = self.new_label("inspect_valid");
            self.emit(format!("blez $a1, {invalid}"));
            self.emit(format!("bltu $a0, $t0, {invalid}"));
            self.emit(format!("li $t1, {capacity}"));
            self.emit("addu $t1, $t0, $t1");
            self.emit(format!("bgeu $a0, $t1, {invalid}"));
            self.emit("subu $t1, $t1, $a0");
            self.emit(format!("bgtu $a1, $t1, {invalid}"));
            self.inspect_invalid(&invalid, &valid);
        }
        true
    }

    fn inspect_region(
        &mut self,
        target: &Expression,
        region: &InspectRegion,
        addr: &str,
        len: &str,
    ) {
        if !self.inspect_text(target) {
            return;
        }
        self.emit(format!("sw $a0, {addr}"));
        self.emit(format!("sw $a1, {len}"));
        for (is_after, boundary) in [(true, &region.after), (false, &region.before)] {
            let Some(boundary) = boundary else { continue };
            if !self.inspect_text(boundary) {
                continue;
            }
            self.emit("move $a2, $a0");
            self.emit("move $a3, $a1");
            self.emit("addiu $sp, $sp, -4");
            self.emit("sw $a3, 0($sp)");
            self.emit(format!("lw $a0, {addr}"));
            self.emit(format!("lw $a1, {len}"));
            self.emit("jal rt_inspect_find");
            self.emit("lw $t2, 0($sp)");
            self.emit("addiu $sp, $sp, 4");
            let found = self.new_label("inspect_boundary");
            let done = self.new_label("inspect_boundary_end");
            self.emit(format!("bgez $v0, {found}"));
            if is_after {
                self.emit(format!("sw $zero, {len}"));
            }
            self.emit(format!("j {done}"));
            self.label(&found);
            if is_after {
                self.emit("addu $v0, $v0, $t2");
                self.emit(format!("lw $t0, {addr}"));
                self.emit("addu $t0, $t0, $v0");
                self.emit(format!("sw $t0, {addr}"));
                self.emit(format!("lw $t0, {len}"));
                self.emit("subu $t0, $t0, $v0");
                self.emit(format!("sw $t0, {len}"));
            } else {
                self.emit(format!("sw $v0, {len}"));
            }
            self.label(&done);
        }
    }

    pub(super) fn inspect(&mut self, node: &InspectStatement) {
        if !matches!(
            node.target,
            Expression::Identifier(..) | Expression::Subscript { .. } | Expression::RefMod { .. }
        ) {
            self.unsupported(node.span, "INSPECT target");
            return;
        }
        self.needs_inspect = true;
        let addr = self.new_word("inspect_addr");
        let len = self.new_word("inspect_len");
        for tally in &node.tallying {
            self.inspect_region(&node.target, &tally.region, &addr, &len);
            if tally.mode == InspectMode::Characters {
                self.emit(format!("lw $v0, {len}"));
            } else if let Some(pattern) = &tally.pattern {
                self.inspect_matches(&addr, &len, pattern, tally.mode, None);
            } else {
                self.unsupported(node.span, "INSPECT TALLYING without a pattern");
                continue;
            }
            self.push_v0();
            let scale = self.gen_num(&tally.counter);
            self.pop("$t0");
            if scale > 0 {
                self.emit(format!("li $t1, {}", super::expr::pow10(scale)));
                self.emit("mul $t0, $t0, $t1");
            }
            self.emit("addu $v0, $v0, $t0");
            self.store_num(&tally.counter, scale);
        }
        if node.replacing.len() > 1
            || node.replacing.iter().any(|replace| {
                self.inspect_length(&replace.by).is_none()
                    || replace
                        .pattern
                        .as_ref()
                        .is_some_and(|pattern| self.inspect_length(pattern).is_none())
            })
        {
            self.inspect_replacing(&node.target, &node.replacing);
        } else {
            for replace in &node.replacing {
                let wanted = match replace.mode {
                    InspectMode::Characters => Some(1),
                    _ => replace
                        .pattern
                        .as_ref()
                        .and_then(|p| self.inspect_length(p)),
                };
                if wanted.is_none()
                    || wanted != self.inspect_length(&replace.by)
                    || wanted == Some(0)
                {
                    self.unsupported(
                        node.span,
                        "INSPECT replacement of unequal or unknown length",
                    );
                    continue;
                }
                self.inspect_region(&node.target, &replace.region, &addr, &len);
                if replace.mode == InspectMode::Characters {
                    if self.gen_text(&replace.by) {
                        self.emit("lbu $a2, 0($a0)");
                        self.emit(format!("lw $a0, {addr}"));
                        self.emit(format!("lw $a1, {len}"));
                        self.emit("jal rt_fill");
                    }
                } else if let Some(pattern) = &replace.pattern {
                    self.inspect_matches(&addr, &len, pattern, replace.mode, Some(&replace.by));
                }
            }
        }
        if let Some(convert) = &node.converting {
            let size = self.inspect_length(&convert.from);
            if size.is_none() || self.inspect_length(&convert.to).is_none() {
                self.inspect_convert_dynamic(&node.target, convert, &addr, &len);
                return;
            }
            if size.is_none() || size != self.inspect_length(&convert.to) || size == Some(0) {
                self.unsupported(node.span, "INSPECT conversion of unequal or unknown length");
                return;
            }
            self.inspect_region(&node.target, &convert.region, &addr, &len);
            if !self.gen_text(&convert.from) {
                return;
            }
            self.push_v0();
            self.emit("sw $a0, 0($sp)");
            if !self.gen_text(&convert.to) {
                self.pop("$t0");
                return;
            }
            self.emit("move $a3, $a0");
            self.emit("move $a2, $a1");
            self.pop("$a1");
            self.emit(format!("lw $a0, {addr}"));
            self.emit(format!("lw $v1, {len}"));
            self.emit("jal rt_inspect_convert");
        }
    }

    fn inspect_replacing(&mut self, target: &Expression, replacements: &[InspectReplace]) {
        struct Phrase {
            mode: InspectMode,
            start: String,
            end: String,
            pattern: Option<String>,
            replacement: String,
            size: String,
            state: String,
        }

        let cursor = self.new_word("inspect_cursor");
        let end = self.new_word("inspect_end");
        if !self.inspect_text(target) {
            return;
        }
        self.emit(format!("sw $a0, {cursor}"));
        self.emit("addu $t0, $a0, $a1");
        self.emit(format!("sw $t0, {end}"));
        let mut phrases = Vec::new();
        let mut stack_size = 0;
        for replace in replacements {
            let fixed_size = if replace.mode == InspectMode::Characters {
                Some(1)
            } else {
                replace
                    .pattern
                    .as_ref()
                    .and_then(|pattern| self.inspect_length(pattern))
            };
            if let (Some(expected), Some(actual)) = (fixed_size, self.inspect_length(&replace.by))
                && expected != actual
            {
                self.unsupported(
                    expression_span(&replace.by),
                    "INSPECT replacement of unequal length",
                );
                return;
            }
            let size = match replace.mode {
                InspectMode::Characters => Some(1),
                _ => replace
                    .pattern
                    .as_ref()
                    .and_then(|pattern| self.inspect_capacity(pattern)),
            };
            let Some(size) = size.filter(|size| *size > 0) else {
                self.unsupported(
                    expression_span(target),
                    "empty or unsupported INSPECT pattern",
                );
                return;
            };
            let Some(replacement_size) =
                self.inspect_capacity(&replace.by).filter(|size| *size > 0)
            else {
                self.unsupported(
                    expression_span(&replace.by),
                    "empty or unsupported INSPECT replacement",
                );
                return;
            };
            let start = self.new_word("inspect_start");
            let length = self.new_word("inspect_region_len");
            let end = self.new_word("inspect_region_end");
            let state = self.new_word("inspect_state");
            self.inspect_region(target, &replace.region, &start, &length);
            self.emit(format!("lw $t0, {start}"));
            self.emit(format!("lw $t1, {length}"));
            self.emit("addu $t1, $t0, $t1");
            self.emit(format!("sw $t1, {end}"));
            if replace.mode == InspectMode::Leading {
                self.emit(format!("sw $t0, {state}"));
            } else {
                self.emit(format!("sw $zero, {state}"));
            }
            let (pattern, pattern_size) = if let Some(pattern) = &replace.pattern {
                let Some((label, length)) = self.inspect_snapshot(pattern, size) else {
                    return;
                };
                stack_size += (size + 3) & !3;
                (Some(label), Some(length))
            } else {
                (None, None)
            };
            let Some((replacement, replacement_length)) =
                self.inspect_snapshot(&replace.by, replacement_size)
            else {
                return;
            };
            stack_size += (replacement_size + 3) & !3;
            let invalid = self.new_label("inspect_invalid");
            let valid = self.new_label("inspect_valid");
            self.emit(format!("lw $t0, {replacement_length}"));
            if let Some(pattern_size) = &pattern_size {
                self.emit(format!("lw $t1, {pattern_size}"));
                self.emit(format!("bne $t0, $t1, {invalid}"));
            } else {
                self.emit("li $t1, 1");
                self.emit(format!("bne $t0, $t1, {invalid}"));
            }
            self.inspect_invalid(&invalid, &valid);
            phrases.push(Phrase {
                mode: replace.mode,
                start,
                end,
                pattern,
                replacement,
                size: replacement_length,
                state,
            });
        }
        let top = self.new_label("inspect_replace_top");
        let done = self.new_label("inspect_replace_done");
        self.label(&top);
        self.emit(format!("lw $t0, {cursor}"));
        self.emit(format!("lw $t1, {end}"));
        self.emit(format!("bgeu $t0, $t1, {done}"));
        for phrase in &phrases {
            let next = self.new_label("inspect_phrase_next");
            let mismatch = self.new_label("inspect_phrase_mismatch");
            self.emit(format!("lw $t0, {cursor}"));
            self.emit(format!("lw $t1, {}", phrase.start));
            self.emit(format!("bltu $t0, $t1, {next}"));
            self.emit(format!("lw $t1, {}", phrase.end));
            self.emit(format!("bgeu $t0, $t1, {next}"));
            if phrase.mode == InspectMode::First {
                self.emit(format!("lw $t1, {}", phrase.state));
                self.emit(format!("bnez $t1, {next}"));
            }
            if phrase.mode == InspectMode::Leading {
                self.emit(format!("lw $t1, {}", phrase.state));
                self.emit(format!("bne $t0, $t1, {mismatch}"));
            }
            self.emit(format!("lw $t2, {}", phrase.size));
            self.emit("addu $t2, $t0, $t2");
            self.emit(format!("lw $t1, {}", phrase.end));
            self.emit(format!("bgtu $t2, $t1, {mismatch}"));
            if let Some(pattern) = &phrase.pattern {
                self.emit("move $a0, $t0");
                self.emit(format!("lw $a1, {}", phrase.size));
                self.emit(format!("lw $a2, {pattern}"));
                self.emit(format!("lw $a3, {}", phrase.size));
                self.emit("jal rt_inspect_find");
                self.emit(format!("bnez $v0, {mismatch}"));
            }
            self.emit(format!("lw $a0, {}", phrase.replacement));
            self.emit(format!("lw $a1, {}", phrase.size));
            self.emit(format!("lw $a2, {cursor}"));
            self.emit(format!("lw $a3, {}", phrase.size));
            self.emit("jal rt_copy_pad");
            self.emit(format!("lw $t0, {cursor}"));
            self.emit(format!("lw $t1, {}", phrase.size));
            self.emit("addu $t0, $t0, $t1");
            self.emit(format!("sw $t0, {cursor}"));
            if phrase.mode == InspectMode::Leading {
                self.emit(format!("sw $t0, {}", phrase.state));
            } else if phrase.mode == InspectMode::First {
                self.emit("li $t0, 1");
                self.emit(format!("sw $t0, {}", phrase.state));
            }
            self.emit(format!("j {top}"));
            self.label(&mismatch);
            if phrase.mode == InspectMode::Leading {
                self.emit(format!("sw $zero, {}", phrase.state));
            }
            self.label(&next);
        }
        self.emit(format!("lw $t0, {cursor}"));
        self.emit("addiu $t0, $t0, 1");
        self.emit(format!("sw $t0, {cursor}"));
        self.emit(format!("j {top}"));
        self.label(&done);
        if stack_size > 0 {
            self.emit(format!("addiu $sp, $sp, {stack_size}"));
        }
    }

    fn inspect_convert_dynamic(
        &mut self,
        target: &Expression,
        convert: &InspectConvert,
        addr: &str,
        len: &str,
    ) {
        let Some(from_size) = self
            .inspect_capacity(&convert.from)
            .filter(|size| *size > 0)
        else {
            self.unsupported(
                expression_span(&convert.from),
                "unsupported INSPECT conversion",
            );
            return;
        };
        let Some(to_size) = self.inspect_capacity(&convert.to).filter(|size| *size > 0) else {
            self.unsupported(
                expression_span(&convert.to),
                "unsupported INSPECT conversion",
            );
            return;
        };
        self.inspect_region(target, &convert.region, addr, len);
        let Some((from, from_len)) = self.inspect_snapshot(&convert.from, from_size) else {
            return;
        };
        let Some((to, to_len)) = self.inspect_snapshot(&convert.to, to_size) else {
            return;
        };
        let invalid = self.new_label("inspect_invalid");
        let valid = self.new_label("inspect_valid");
        self.emit(format!("lw $t0, {from_len}"));
        self.emit(format!("lw $t1, {to_len}"));
        self.emit(format!("bne $t0, $t1, {invalid}"));
        self.inspect_invalid(&invalid, &valid);
        self.emit(format!("lw $a0, {addr}"));
        self.emit(format!("lw $a1, {from}"));
        self.emit(format!("lw $a2, {from_len}"));
        self.emit(format!("lw $a3, {to}"));
        self.emit(format!("lw $v1, {len}"));
        self.emit("jal rt_inspect_convert");
        let stack_size = ((from_size + 3) & !3) + ((to_size + 3) & !3);
        self.emit(format!("addiu $sp, $sp, {stack_size}"));
    }

    fn inspect_snapshot(
        &mut self,
        expression: &Expression,
        size: usize,
    ) -> Option<(String, String)> {
        if !self.inspect_text(expression) {
            return None;
        }
        let address = self.new_word("inspect_operand");
        let length = self.new_word("inspect_operand_len");
        self.emit(format!("sw $a1, {length}"));
        let stack_size = (size + 3) & !3;
        self.emit(format!("addiu $sp, $sp, -{stack_size}"));
        self.emit("move $a2, $sp");
        self.emit("move $a3, $a1");
        self.emit("jal rt_copy_pad");
        self.emit(format!("sw $sp, {address}"));
        Some((address, length))
    }

    fn inspect_matches(
        &mut self,
        addr: &str,
        len: &str,
        pattern: &Expression,
        mode: InspectMode,
        replacement: Option<&Expression>,
    ) {
        let pattern_addr = self.new_word("inspect_pattern");
        let pattern_len = self.new_word("inspect_pattern_len");
        let count = self.new_word("inspect_count");
        let replacement_addr = self.new_word("inspect_replacement");
        if !self.inspect_text(pattern) {
            return;
        }
        self.emit(format!("sw $a0, {pattern_addr}"));
        self.emit(format!("sw $a1, {pattern_len}"));
        if let Some(replacement) = replacement {
            if !self.gen_text(replacement) {
                return;
            }
            self.emit(format!("sw $a0, {replacement_addr}"));
        }
        self.emit(format!("sw $zero, {count}"));
        let top = self.new_label("inspect_top");
        let done = self.new_label("inspect_done");
        self.label(&top);
        self.emit(format!("lw $a0, {addr}"));
        self.emit(format!("lw $a1, {len}"));
        self.emit(format!("lw $a2, {pattern_addr}"));
        self.emit(format!("lw $a3, {pattern_len}"));
        self.emit(format!("beqz $a3, {done}"));
        self.emit("jal rt_inspect_find");
        self.emit(format!("bltz $v0, {done}"));
        if mode == InspectMode::Leading {
            self.emit(format!("bnez $v0, {done}"));
        }
        self.emit(format!("lw $t0, {addr}"));
        self.emit("addu $t0, $t0, $v0");
        self.emit(format!("lw $t1, {pattern_len}"));
        self.emit(format!("lw $t2, {len}"));
        self.emit("subu $t2, $t2, $v0");
        self.emit("subu $t2, $t2, $t1");
        self.emit("addu $t3, $t0, $t1");
        self.emit(format!("sw $t3, {addr}"));
        self.emit(format!("sw $t2, {len}"));
        if replacement.is_some() {
            self.emit(format!("lw $a0, {replacement_addr}"));
            self.emit("move $a1, $t1");
            self.emit("move $a2, $t0");
            self.emit("move $a3, $t1");
            self.emit("jal rt_copy_pad");
        }
        self.emit(format!("lw $t0, {count}"));
        self.emit("addiu $t0, $t0, 1");
        self.emit(format!("sw $t0, {count}"));
        if mode == InspectMode::First {
            self.emit(format!("j {done}"));
        } else {
            self.emit(format!("j {top}"));
        }
        self.label(&done);
        self.emit(format!("lw $v0, {count}"));
    }
}

pub(super) const RUNTIME: &str = r#"
# Find the first occurrence of $a2/$a3 in $a0/$a1; return offset or -1.
rt_inspect_find:
    li $t0, 0
    subu $t1, $a1, $a3
rt_inspect_find_next:
    bgt $t0, $t1, rt_inspect_find_missing
    li $t2, 0
rt_inspect_find_compare:
    beq $t2, $a3, rt_inspect_find_found
    addu $t3, $a0, $t0
    addu $t3, $t3, $t2
    lbu $t4, 0($t3)
    addu $t3, $a2, $t2
    lbu $t5, 0($t3)
    bne $t4, $t5, rt_inspect_find_advance
    addiu $t2, $t2, 1
    j rt_inspect_find_compare
rt_inspect_find_advance:
    addiu $t0, $t0, 1
    j rt_inspect_find_next
rt_inspect_find_found:
    move $v0, $t0
    jr $ra
rt_inspect_find_missing:
    li $v0, -1
    jr $ra

# Convert $v1 bytes at $a0 using $a2 bytes of mapping $a1 -> $a3.
rt_inspect_convert:
    li $t0, 0
rt_inspect_convert_next:
    beq $t0, $v1, rt_inspect_convert_done
    addu $t1, $a0, $t0
    lbu $t2, 0($t1)
    li $t3, 0
rt_inspect_convert_lookup:
    beq $t3, $a2, rt_inspect_convert_advance
    addu $t4, $a1, $t3
    lbu $t5, 0($t4)
    beq $t2, $t5, rt_inspect_convert_replace
    addiu $t3, $t3, 1
    j rt_inspect_convert_lookup
rt_inspect_convert_replace:
    addu $t4, $a3, $t3
    lbu $t5, 0($t4)
    sb $t5, 0($t1)
rt_inspect_convert_advance:
    addiu $t0, $t0, 1
    j rt_inspect_convert_next
rt_inspect_convert_done:
    jr $ra
"#;
