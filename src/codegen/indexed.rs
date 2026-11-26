//! Indexed fixed-record files implemented over MARS sequential file syscalls.

use super::Generator;
use super::expr::{Class, Place, class_of, name_and_indices};
use crate::parser::{BinaryOperator, Expression, ReadStatement, StartStatement, WriteStatement};
use crate::source::Span;

impl Generator<'_> {
    fn indexed_key(&mut self, file: &str, expression: &Expression) -> Option<(usize, usize, bool)> {
        let Some((name, indices)) = name_and_indices(expression) else {
            self.unsupported(super::expression_span(expression), "indexed file key");
            return None;
        };
        let Some(Place::Item(resolved)) = self.place(name) else {
            self.unsupported(super::expression_span(expression), "indexed file key");
            return None;
        };
        let signed = resolved
            .item()
            .picture
            .as_ref()
            .is_some_and(|info| info.signed);
        if resolved.file != Some(file)
            || !indices.is_empty()
            || (signed && class_of(resolved.item()) != Class::Plain)
        {
            self.unsupported(super::expression_span(expression), "indexed file key");
            return None;
        }
        Some((resolved.item().offset, resolved.item().element_size, signed))
    }

    fn indexed_keys(&mut self, file: &str, span: Span) -> Vec<(usize, usize, bool, bool)> {
        let Some(control) = self.file_controls.get(&file.to_ascii_uppercase()) else {
            self.unsupported(span, "indexed file without SELECT");
            return Vec::new();
        };
        let Some(primary) = &control.record_key else {
            self.unsupported(span, "indexed file without a record key");
            return Vec::new();
        };
        let expressions: Vec<(Expression, bool)> = std::iter::once((primary.clone(), false))
            .chain(
                control
                    .alternate_keys
                    .iter()
                    .map(|key| (key.expression.clone(), key.duplicates)),
            )
            .collect();
        expressions
            .iter()
            .filter_map(|(expression, duplicates)| {
                self.indexed_key(file, expression)
                    .map(|(offset, length, signed)| (offset, length, signed, *duplicates))
            })
            .collect()
    }

    pub(super) fn open_indexed(&mut self, name: &str, span: Span, mode: &str) {
        let Some(number) = self.file_number(name, span) else {
            return;
        };
        let keys = self.indexed_keys(name, span);
        let Some(control) = self.file_controls.get(&name.to_ascii_uppercase()) else {
            self.unsupported(span, "indexed file without SELECT");
            return;
        };
        let Some(filename) = &control.assign_to else {
            self.unsupported(span, "indexed file without ASSIGN");
            return;
        };
        let flags = match mode {
            "INPUT" | "I-O" => 0,
            "OUTPUT" => 1,
            _ => {
                self.unsupported(span, &format!("OPEN {mode} indexed file"));
                return;
            }
        };
        self.needs_indexed = true;
        if !self.gen_text(filename) {
            return;
        }
        let top = self.new_label("idx_path_copy");
        let trim = self.new_label("idx_path_trim");
        let terminate = self.new_label("idx_path_end");
        let failed = self.new_label("idx_open_failed");
        let load_failed = self.new_label("idx_load_failed");
        let already = self.new_label("idx_open_already");
        let done = self.new_label("idx_open_done");
        self.emit(format!("lw $t0, file_fd_{number}"));
        self.emit(format!("bgez $t0, {already}"));
        self.emit(format!("la $t0, file_path_{number}"));
        self.emit("move $t1, $a0");
        self.emit("move $t2, $a1");
        self.emit("li $t3, 0");
        self.label(&top);
        self.emit(format!("beqz $t2, {trim}"));
        self.emit("lbu $t4, 0($t1)");
        self.emit(format!("beqz $t4, {trim}"));
        self.emit("li $t5, 255");
        self.emit(format!("bge $t3, $t5, {failed}"));
        self.emit("sb $t4, 0($t0)");
        self.emit("addiu $t0, $t0, 1");
        self.emit("addiu $t1, $t1, 1");
        self.emit("addiu $t2, $t2, -1");
        self.emit("addiu $t3, $t3, 1");
        self.emit(format!("j {top}"));
        self.label(&trim);
        self.emit(format!("beqz $t3, {terminate}"));
        self.emit("lbu $t4, -1($t0)");
        self.emit("li $t5, 32");
        self.emit(format!("bne $t4, $t5, {terminate}"));
        self.emit("addiu $t0, $t0, -1");
        self.emit("addiu $t3, $t3, -1");
        self.emit(format!("j {trim}"));
        self.label(&terminate);
        self.emit("sb $zero, 0($t0)");
        self.emit(format!("la $a0, file_path_{number}"));
        self.emit(format!("li $a1, {flags}"));
        self.emit("li $a2, 0");
        self.emit("li $v0, 13");
        self.emit("syscall");
        self.emit(format!("sw $v0, file_fd_{number}"));
        self.emit(format!("bltz $v0, {failed}"));
        self.emit(format!("sw $zero, file_index_{number}"));
        self.emit(format!("sw $zero, file_cursor_{number}"));
        self.emit(format!("sw $zero, file_cursor_pending_{number}"));
        if let Some((offset, length, signed, _)) = keys.first() {
            self.emit(format!("li $t0, {offset}"));
            self.emit(format!("sw $t0, file_cursor_offset_{number}"));
            self.emit(format!("li $t0, {length}"));
            self.emit(format!("sw $t0, file_cursor_length_{number}"));
            self.emit(format!("li $t0, {}", u8::from(*signed)));
            self.emit(format!("sw $t0, file_cursor_signed_{number}"));
        }
        if flags == 0 {
            self.emit("move $a0, $v0");
            self.emit(format!("la $a1, file_index_{number}"));
            self.emit(format!("li $a2, {}", self.layout.files[number].record_size));
            self.emit("jal rt_index_load");
            self.emit(format!("bltz $v0, {load_failed}"));
            if mode == "I-O" {
                self.emit(format!("lw $a0, file_fd_{number}"));
                self.emit("li $v0, 16");
                self.emit("syscall");
                self.emit(format!("la $a0, file_path_{number}"));
                self.emit("li $a1, 9");
                self.emit("li $a2, 0");
                self.emit("li $v0, 13");
                self.emit("syscall");
                self.emit(format!("sw $v0, file_fd_{number}"));
                self.emit(format!("bltz $v0, {failed}"));
            }
        }
        self.emit(format!(
            "li $t0, {}",
            match mode {
                "INPUT" => 1,
                "OUTPUT" => 2,
                _ => 3,
            }
        ));
        self.emit(format!("sw $t0, file_mode_{number}"));
        self.file_status(name, span, "0");
        self.emit(format!("j {done}"));
        self.label(&load_failed);
        self.emit(format!("lw $a0, file_fd_{number}"));
        self.emit("li $v0, 16");
        self.emit("syscall");
        self.emit("li $t0, -1");
        self.emit(format!("sw $t0, file_fd_{number}"));
        self.label(&failed);
        self.file_status(name, span, "30");
        self.emit(format!("j {done}"));
        self.label(&already);
        self.file_status(name, span, "41");
        self.label(&done);
    }

    pub(super) fn write_indexed(&mut self, node: &WriteStatement) {
        if node.advancing.is_some()
            || node.handlers.at_end.is_some()
            || node.handlers.not_at_end.is_some()
        {
            self.unsupported(node.span, "indexed WRITE with advancing or AT END");
            return;
        }
        let Some((name, _)) = name_and_indices(&node.record) else {
            self.unsupported(node.span, "indexed WRITE record");
            return;
        };
        let Some(Place::Item(resolved)) = self.place(name) else {
            self.unsupported(node.span, "indexed WRITE record");
            return;
        };
        let Some(file) = resolved.file.map(str::to_string) else {
            self.unsupported(node.span, "indexed WRITE record");
            return;
        };
        let Some(number) = self.file_number(&file, node.span) else {
            return;
        };
        let size = resolved.item().element_size;
        let keys = self.indexed_keys(&file, node.span);
        self.needs_indexed = true;
        let bad_mode = self.new_label("idx_write_mode");
        let duplicate = self.new_label("idx_write_duplicate");
        let failed = self.new_label("idx_write_failed");
        let done = self.new_label("idx_write_done");
        let duplicate_allowed = keys
            .iter()
            .any(|(_, _, _, duplicates)| *duplicates)
            .then(|| self.new_word("idx_duplicate_allowed"));
        self.emit(format!("lw $t0, file_mode_{number}"));
        self.emit("li $t1, 2");
        self.emit(format!("beq $t0, $t1, {done}_mode_ok"));
        self.emit("li $t1, 3");
        self.emit(format!("bne $t0, $t1, {bad_mode}"));
        self.label(&format!("{done}_mode_ok"));
        if let Some(from) = &node.from {
            self.move_one(from, &node.record, node.span);
        }
        if let Some(flag) = &duplicate_allowed {
            self.emit(format!("sw $zero, {flag}"));
        }
        for (offset, length, signed, duplicates) in keys {
            self.emit(format!("lw $a0, file_index_{number}"));
            self.emit(format!("la $a1, file_buf_{number}+{offset}"));
            self.emit(format!("li $a2, {offset}"));
            self.emit(format!("li $a3, {length}"));
            self.emit("addiu $sp, $sp, -8");
            self.emit("sw $zero, 0($sp)");
            self.emit(format!("li $t0, {}", u8::from(signed)));
            self.emit("sw $t0, 4($sp)");
            self.emit("jal rt_index_select");
            self.emit("addiu $sp, $sp, 8");
            if duplicates {
                let no_duplicate = self.new_label("idx_no_duplicate");
                self.emit(format!("beqz $v0, {no_duplicate}"));
                self.emit("li $t0, 1");
                self.emit(format!(
                    "sw $t0, {}",
                    duplicate_allowed.as_ref().expect("duplicate flag exists")
                ));
                self.label(&no_duplicate);
            } else {
                self.emit(format!("bnez $v0, {duplicate}"));
            }
        }
        self.emit(format!("lw $a0, file_fd_{number}"));
        self.emit(format!("la $a1, file_buf_{number}"));
        self.emit(format!("li $a2, {size}"));
        self.emit("li $v0, 15");
        self.emit("syscall");
        self.emit(format!("li $t0, {size}"));
        self.emit(format!("bne $v0, $t0, {failed}"));
        self.emit(format!("la $a0, file_index_{number}"));
        self.emit(format!("la $a1, file_buf_{number}"));
        self.emit(format!("li $a2, {size}"));
        self.emit("jal rt_index_append");
        if let Some(flag) = duplicate_allowed {
            let duplicate_status = self.new_label("idx_write_duplicate_status");
            let status_done = self.new_label("idx_write_status_done");
            self.emit(format!("lw $t0, {flag}"));
            self.emit(format!("bnez $t0, {duplicate_status}"));
            self.file_status(&file, node.span, "0");
            self.emit(format!("j {status_done}"));
            self.label(&duplicate_status);
            self.file_status_handled(&file, node.span, "2", true);
            self.label(&status_done);
        } else {
            self.file_status(&file, node.span, "0");
        }
        if let Some(body) = &node.handlers.not_invalid_key {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&duplicate);
        self.file_status_handled(&file, node.span, "22", node.handlers.invalid_key.is_some());
        if let Some(body) = &node.handlers.invalid_key {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&bad_mode);
        self.file_status(&file, node.span, "48");
        self.emit(format!("j {done}"));
        self.label(&failed);
        self.file_status(&file, node.span, "30");
        self.label(&done);
    }

    pub(super) fn read_indexed(&mut self, node: &ReadStatement) {
        let Some(key) = &node.key else {
            self.read_indexed_next(node);
            return;
        };
        if node.handlers.at_end.is_some() || node.handlers.not_at_end.is_some() {
            self.unsupported(node.span, "keyed READ with AT END");
            return;
        }
        let file = &node.file.0;
        let Some(number) = self.file_number(file, node.span) else {
            return;
        };
        let Some((offset, length, signed)) = self.indexed_key(file, key) else {
            return;
        };
        if !self.indexed_keys(file, node.span).iter().any(
            |&(key_offset, key_length, key_signed, _)| {
                (key_offset, key_length, key_signed) == (offset, length, signed)
            },
        ) {
            self.unsupported(node.span, "READ using a non-key field");
            return;
        }
        let record = &self.layout.files[number].records[0];
        let record_expr = Expression::Identifier(record.name.clone(), node.span);
        let size = record.element_size;
        self.needs_indexed = true;
        let bad_mode = self.new_label("idx_read_mode");
        let missing = self.new_label("idx_read_missing");
        let done = self.new_label("idx_read_done");
        self.emit(format!("lw $t0, file_mode_{number}"));
        self.emit("li $t1, 1");
        self.emit(format!("beq $t0, $t1, {done}_mode_ok"));
        self.emit("li $t1, 3");
        self.emit(format!("bne $t0, $t1, {bad_mode}"));
        self.label(&format!("{done}_mode_ok"));
        if !self.gen_text(key) {
            return;
        }
        self.emit("move $a1, $a0");
        self.emit(format!("lw $a0, file_index_{number}"));
        self.emit(format!("li $a2, {offset}"));
        self.emit(format!("li $a3, {length}"));
        self.emit("addiu $sp, $sp, -8");
        self.emit("sw $zero, 0($sp)");
        self.emit(format!("li $t0, {}", u8::from(signed)));
        self.emit("sw $t0, 4($sp)");
        self.emit("jal rt_index_select");
        self.emit("addiu $sp, $sp, 8");
        self.emit(format!("beqz $v0, {missing}"));
        self.emit(format!("sw $v0, file_cursor_{number}"));
        self.emit(format!("sw $zero, file_cursor_pending_{number}"));
        self.emit(format!("li $t0, {offset}"));
        self.emit(format!("sw $t0, file_cursor_offset_{number}"));
        self.emit(format!("li $t0, {length}"));
        self.emit(format!("sw $t0, file_cursor_length_{number}"));
        self.emit(format!("li $t0, {}", u8::from(signed)));
        self.emit(format!("sw $t0, file_cursor_signed_{number}"));
        self.emit("addiu $a0, $v0, 4");
        self.emit(format!("li $a1, {size}"));
        self.emit(format!("la $a2, file_buf_{number}"));
        self.emit(format!("li $a3, {size}"));
        self.emit("jal rt_copy_pad");
        if let Some(into) = &node.into {
            self.move_one(&record_expr, into, node.span);
        }
        self.index_read_status(file, node.span, number);
        if let Some(body) = &node.handlers.not_invalid_key {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&missing);
        self.file_status_handled(file, node.span, "23", node.handlers.invalid_key.is_some());
        if let Some(body) = &node.handlers.invalid_key {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&bad_mode);
        self.file_status(file, node.span, "47");
        self.label(&done);
    }

    fn read_indexed_next(&mut self, node: &ReadStatement) {
        if node.handlers.invalid_key.is_some() || node.handlers.not_invalid_key.is_some() {
            self.unsupported(node.span, "sequential indexed READ with INVALID KEY");
            return;
        }
        let file = &node.file.0;
        let Some(number) = self.file_number(file, node.span) else {
            return;
        };
        let Some(record) = self.layout.files[number].records.first() else {
            self.unsupported(node.span, "indexed READ without a record");
            return;
        };
        let record_expr = Expression::Identifier(record.name.clone(), node.span);
        let size = record.element_size;
        self.needs_indexed = true;
        let bad_mode = self.new_label("idx_next_mode");
        let seek = self.new_label("idx_next_seek");
        let first = self.new_label("idx_next_first");
        let select = self.new_label("idx_next_select");
        let found = self.new_label("idx_next_found");
        let at_end = self.new_label("idx_next_end");
        let done = self.new_label("idx_next_done");
        self.emit(format!("lw $t0, file_mode_{number}"));
        self.emit("li $t1, 1");
        self.emit(format!("beq $t0, $t1, {done}_mode_ok"));
        self.emit("li $t1, 3");
        self.emit(format!("bne $t0, $t1, {bad_mode}"));
        self.label(&format!("{done}_mode_ok"));
        self.emit(format!("lw $t0, file_cursor_pending_{number}"));
        self.emit(format!("beqz $t0, {seek}"));
        self.emit(format!("sw $zero, file_cursor_pending_{number}"));
        self.emit(format!("lw $v0, file_cursor_{number}"));
        self.emit(format!("j {found}"));
        self.label(&seek);
        self.emit(format!("lw $a0, file_index_{number}"));
        self.emit(format!("lw $a2, file_cursor_offset_{number}"));
        self.emit(format!("lw $a3, file_cursor_length_{number}"));
        self.emit(format!("lw $a1, file_cursor_{number}"));
        self.emit(format!("beqz $a1, {first}"));
        self.emit("li $t0, 6");
        self.emit(format!("j {select}"));
        self.label(&first);
        self.emit("li $t0, 5");
        self.label(&select);
        self.emit("addiu $sp, $sp, -8");
        self.emit("sw $t0, 0($sp)");
        self.emit(format!("lw $t0, file_cursor_signed_{number}"));
        self.emit("sw $t0, 4($sp)");
        self.emit("jal rt_index_select");
        self.emit("addiu $sp, $sp, 8");
        self.label(&found);
        self.emit(format!("beqz $v0, {at_end}"));
        self.emit(format!("sw $v0, file_cursor_{number}"));
        self.emit("addiu $a0, $v0, 4");
        self.emit(format!("li $a1, {size}"));
        self.emit(format!("la $a2, file_buf_{number}"));
        self.emit(format!("li $a3, {size}"));
        self.emit("jal rt_copy_pad");
        if let Some(into) = &node.into {
            self.move_one(&record_expr, into, node.span);
        }
        self.index_read_status(file, node.span, number);
        if let Some(body) = &node.handlers.not_at_end {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&at_end);
        if node.handlers.at_end.is_none()
            || self
                .file_controls
                .get(&file.to_ascii_uppercase())
                .is_some_and(|control| control.file_status.is_some())
        {
            self.file_status(file, node.span, "10");
        }
        if let Some(body) = &node.handlers.at_end {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&bad_mode);
        self.file_status(file, node.span, "47");
        self.label(&done);
    }

    pub(super) fn start_indexed(&mut self, node: &StartStatement) {
        let file = &node.file.0;
        if !self
            .file_controls
            .get(&file.to_ascii_uppercase())
            .is_some_and(|control| control.organization.as_deref() == Some("INDEXED"))
        {
            self.unsupported(node.span, "START of a non-indexed file");
            return;
        }
        let Some((op, key)) = &node.key else {
            self.unsupported(node.span, "START without an explicit key");
            return;
        };
        let relation = match op {
            BinaryOperator::Equal => 0,
            BinaryOperator::GreaterThan => 1,
            BinaryOperator::GreaterThanOrEqual => 2,
            BinaryOperator::LessThan => 3,
            BinaryOperator::LessThanOrEqual => 4,
            _ => {
                self.unsupported(node.span, "START key relation");
                return;
            }
        };
        let Some(number) = self.file_number(file, node.span) else {
            return;
        };
        let Some((offset, length, signed)) = self.indexed_key(file, key) else {
            return;
        };
        if !self.indexed_keys(file, node.span).iter().any(
            |&(key_offset, key_length, key_signed, _)| {
                (key_offset, key_length, key_signed) == (offset, length, signed)
            },
        ) {
            self.unsupported(node.span, "START using a non-key field");
            return;
        }
        self.needs_indexed = true;
        let missing = self.new_label("idx_start_missing");
        let bad_mode = self.new_label("idx_start_mode");
        let done = self.new_label("idx_start_done");
        self.emit(format!("lw $t0, file_mode_{number}"));
        self.emit("li $t1, 1");
        self.emit(format!("beq $t0, $t1, {done}_mode_ok"));
        self.emit("li $t1, 3");
        self.emit(format!("bne $t0, $t1, {bad_mode}"));
        self.label(&format!("{done}_mode_ok"));
        if !self.gen_text(key) {
            return;
        }
        self.emit("move $a1, $a0");
        self.emit(format!("lw $a0, file_index_{number}"));
        self.emit(format!("li $a2, {offset}"));
        self.emit(format!("li $a3, {length}"));
        self.emit("addiu $sp, $sp, -8");
        self.emit(format!("li $t0, {relation}"));
        self.emit("sw $t0, 0($sp)");
        self.emit(format!("li $t0, {}", u8::from(signed)));
        self.emit("sw $t0, 4($sp)");
        self.emit("jal rt_index_select");
        self.emit("addiu $sp, $sp, 8");
        self.emit(format!("sw $v0, file_cursor_{number}"));
        self.emit(format!("beqz $v0, {missing}"));
        self.emit("li $t0, 1");
        self.emit(format!("sw $t0, file_cursor_pending_{number}"));
        self.emit(format!("li $t0, {offset}"));
        self.emit(format!("sw $t0, file_cursor_offset_{number}"));
        self.emit(format!("li $t0, {length}"));
        self.emit(format!("sw $t0, file_cursor_length_{number}"));
        self.emit(format!("li $t0, {}", u8::from(signed)));
        self.emit(format!("sw $t0, file_cursor_signed_{number}"));
        self.file_status(file, node.span, "0");
        if let Some(body) = &node.handlers.not_invalid_key {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&missing);
        self.emit(format!("sw $zero, file_cursor_pending_{number}"));
        self.file_status_handled(file, node.span, "23", node.handlers.invalid_key.is_some());
        if let Some(body) = &node.handlers.invalid_key {
            self.statements(body);
        }
        self.emit(format!("j {done}"));
        self.label(&bad_mode);
        self.file_status(file, node.span, "47");
        self.label(&done);
    }

    fn index_read_status(&mut self, file: &str, span: Span, number: usize) {
        let has_duplicates = self
            .file_controls
            .get(&file.to_ascii_uppercase())
            .is_some_and(|control| control.alternate_keys.iter().any(|key| key.duplicates));
        if !has_duplicates {
            self.file_status(file, span, "0");
            return;
        }
        let unique = self.new_label("idx_read_unique");
        let done = self.new_label("idx_read_status_done");
        self.emit(format!("lw $a0, file_index_{number}"));
        self.emit(format!("lw $a1, file_cursor_{number}"));
        self.emit(format!("lw $a2, file_cursor_offset_{number}"));
        self.emit(format!("lw $a3, file_cursor_length_{number}"));
        self.emit("addiu $sp, $sp, -8");
        self.emit("li $t0, 6");
        self.emit("sw $t0, 0($sp)");
        self.emit(format!("lw $t0, file_cursor_signed_{number}"));
        self.emit("sw $t0, 4($sp)");
        self.emit("jal rt_index_select");
        self.emit("addiu $sp, $sp, 8");
        self.emit(format!("beqz $v0, {unique}"));
        self.emit(format!("lw $t0, file_cursor_offset_{number}"));
        self.emit("addiu $a0, $v0, 4");
        self.emit("addu $a0, $a0, $t0");
        self.emit(format!("lw $a1, file_cursor_{number}"));
        self.emit("addiu $a1, $a1, 4");
        self.emit("addu $a1, $a1, $t0");
        self.emit(format!("lw $a2, file_cursor_length_{number}"));
        self.emit(format!("lw $a3, file_cursor_signed_{number}"));
        self.emit("jal rt_index_compare");
        self.emit(format!("bnez $v0, {unique}"));
        self.file_status_handled(file, span, "2", true);
        self.emit(format!("j {done}"));
        self.label(&unique);
        self.file_status(file, span, "0");
        self.label(&done);
    }
}

pub(super) const RUNTIME: &str = r#"
# Compare $a2 bytes at $a0 and $a1; $a3 selects signed DISPLAY ordering.
rt_index_compare:
    bnez $a3, rt_index_compare_numeric
    move $t0, $zero
rt_index_compare_loop:
    beq $t0, $a2, rt_index_compare_equal
    addu $t1, $a0, $t0
    lbu $t2, 0($t1)
    addu $t1, $a1, $t0
    lbu $t3, 0($t1)
    blt $t2, $t3, rt_index_compare_less
    bgt $t2, $t3, rt_index_compare_greater
    addiu $t0, $t0, 1
    j rt_index_compare_loop
rt_index_compare_less:
    li $v0, -1
    jr $ra
rt_index_compare_greater:
    li $v0, 1
    jr $ra
rt_index_compare_equal:
    move $v0, $zero
    jr $ra

rt_index_compare_numeric:
    addu $t0, $a0, $a2
    lbu $t4, -1($t0)
    sltiu $t4, $t4, 0x70
    xori $t4, $t4, 1
    addu $t0, $a1, $a2
    lbu $t5, -1($t0)
    sltiu $t5, $t5, 0x70
    xori $t5, $t5, 1
    move $t0, $zero
    move $t6, $zero
    move $t7, $zero
    move $t8, $zero
rt_index_compare_numeric_loop:
    beq $t0, $a2, rt_index_compare_numeric_sign
    addu $t1, $a0, $t0
    lbu $t2, 0($t1)
    addu $t1, $a1, $t0
    lbu $t3, 0($t1)
    addiu $t1, $a2, -1
    bne $t0, $t1, rt_index_compare_numeric_digits
    beqz $t4, rt_index_compare_numeric_right
    addiu $t2, $t2, -0x40
rt_index_compare_numeric_right:
    beqz $t5, rt_index_compare_numeric_digits
    addiu $t3, $t3, -0x40
rt_index_compare_numeric_digits:
    addiu $t2, $t2, -48
    addiu $t3, $t3, -48
    or $t7, $t7, $t2
    or $t8, $t8, $t3
    bnez $t6, rt_index_compare_numeric_next
    slt $t1, $t2, $t3
    slt $t9, $t3, $t2
    subu $t6, $t9, $t1
rt_index_compare_numeric_next:
    addiu $t0, $t0, 1
    j rt_index_compare_numeric_loop
rt_index_compare_numeric_sign:
    sltu $t7, $zero, $t7
    sltu $t8, $zero, $t8
    and $t4, $t4, $t7
    and $t5, $t5, $t8
    beq $t4, $t5, rt_index_compare_numeric_same_sign
    subu $v0, $t5, $t4
    jr $ra
rt_index_compare_numeric_same_sign:
    move $v0, $t6
    beqz $t4, rt_index_compare_numeric_done
    negu $v0, $v0
rt_index_compare_numeric_done:
    jr $ra

# Select =, >, >=, <, <=, first, or next (0..6), preserving duplicate order.
# $a0=head, $a1=key (or cursor for next), $a2=offset, $a3=length;
# relation and signed-key flag are on the stack.
rt_index_select:
    lw $t0, 0($sp)
    lw $t1, 4($sp)
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
    move $s4, $t0
    move $s7, $t1
    move $s5, $zero
    move $s6, $s0
rt_index_select_loop:
    beqz $s6, rt_index_select_done
    li $t0, 5
    beq $s4, $t0, rt_index_select_candidate
    addiu $a0, $s6, 4
    addu $a0, $a0, $s2
    move $a1, $s1
    li $t0, 6
    bne $s4, $t0, rt_index_select_compare
    addiu $a1, $s1, 4
    addu $a1, $a1, $s2
rt_index_select_compare:
    move $a2, $s3
    move $a3, $s7
    jal rt_index_compare
    li $t0, 6
    beq $s4, $t0, rt_index_select_after_cursor
    beqz $s4, rt_index_select_equal
    li $t0, 1
    beq $s4, $t0, rt_index_select_gt
    li $t0, 2
    beq $s4, $t0, rt_index_select_ge
    li $t0, 3
    beq $s4, $t0, rt_index_select_lt
    blez $v0, rt_index_select_candidate
    j rt_index_select_next
rt_index_select_equal:
    beqz $v0, rt_index_select_candidate
    j rt_index_select_next
rt_index_select_gt:
    bgtz $v0, rt_index_select_candidate
    j rt_index_select_next
rt_index_select_ge:
    bgez $v0, rt_index_select_candidate
    j rt_index_select_next
rt_index_select_lt:
    bltz $v0, rt_index_select_candidate
    j rt_index_select_next
rt_index_select_after_cursor:
    bgtz $v0, rt_index_select_candidate
    bnez $v0, rt_index_select_next
    bgtu $s6, $s1, rt_index_select_candidate
    j rt_index_select_next
rt_index_select_candidate:
    beqz $s5, rt_index_select_take
    addiu $a0, $s6, 4
    addu $a0, $a0, $s2
    addiu $a1, $s5, 4
    addu $a1, $a1, $s2
    move $a2, $s3
    move $a3, $s7
    jal rt_index_compare
    bnez $v0, rt_index_select_compare_direction
    bltu $s6, $s5, rt_index_select_take
    j rt_index_select_next
rt_index_select_compare_direction:
    li $t0, 3
    beq $s4, $t0, rt_index_select_max
    li $t0, 4
    beq $s4, $t0, rt_index_select_max
    bltz $v0, rt_index_select_take
    j rt_index_select_next
rt_index_select_max:
    bgtz $v0, rt_index_select_take
    j rt_index_select_next
rt_index_select_take:
    move $s5, $s6
rt_index_select_next:
    lw $s6, 0($s6)
    j rt_index_select_loop
rt_index_select_done:
    move $v0, $s5
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

# $a0=&head, $a1=record, $a2=record size.
rt_index_append:
    move $t0, $a0
    move $t1, $a1
    move $t2, $a2
    addiu $a0, $a2, 7
    li $t3, -4
    and $a0, $a0, $t3
    li $v0, 9
    syscall
    lw $t3, 0($t0)
    sw $t3, 0($v0)
    sw $v0, 0($t0)
    addiu $t3, $v0, 4
    li $t4, 0
rt_index_append_copy:
    beq $t4, $t2, rt_index_append_done
    addu $t5, $t1, $t4
    lbu $t6, 0($t5)
    addu $t5, $t3, $t4
    sb $t6, 0($t5)
    addiu $t4, $t4, 1
    j rt_index_append_copy
rt_index_append_done:
    jr $ra

# $a0=descriptor, $a1=&head, $a2=record size. Returns 0 or -1.
rt_index_load:
    addiu $sp, $sp, -20
    sw $s0, 0($sp)
    sw $s1, 4($sp)
    sw $s2, 8($sp)
    sw $s3, 12($sp)
    sw $ra, 16($sp)
    move $s0, $a0
    move $s1, $a1
    move $s2, $a2
rt_index_load_next:
    addiu $a0, $s2, 7
    li $t0, -4
    and $a0, $a0, $t0
    li $v0, 9
    syscall
    move $s3, $v0
    move $a0, $s0
    addiu $a1, $s3, 4
    move $a2, $s2
    li $v0, 14
    syscall
    beqz $v0, rt_index_load_done
    bne $v0, $s2, rt_index_load_failed
    lw $t0, 0($s1)
    sw $t0, 0($s3)
    sw $s3, 0($s1)
    j rt_index_load_next
rt_index_load_failed:
    li $v0, -1
    j rt_index_load_return
rt_index_load_done:
    move $v0, $zero
rt_index_load_return:
    lw $s0, 0($sp)
    lw $s1, 4($sp)
    lw $s2, 8($sp)
    lw $s3, 12($sp)
    lw $ra, 16($sp)
    addiu $sp, $sp, 20
    jr $ra
"#;
