//! Sequential file I/O for MARS.

use super::Generator;
use super::expr::{Kind, Place, name_and_indices};
use crate::parser::{
    CloseStatement, Expression, OpenStatement, ReadStatement, WriteAdvancing, WriteStatement,
};
use crate::source::Span;

impl Generator<'_> {
    pub(super) fn file_number(&mut self, name: &str, span: Span) -> Option<usize> {
        let number = self
            .layout
            .files
            .iter()
            .position(|file| file.name.eq_ignore_ascii_case(name));
        if number.is_none() {
            self.unsupported(span, &format!("file {name} without a record layout"));
        }
        number
    }

    pub(super) fn file_status(&mut self, name: &str, span: Span, value: &str) {
        self.file_status_handled(name, span, value, false);
    }

    pub(super) fn file_status_handled(
        &mut self,
        name: &str,
        span: Span,
        value: &str,
        handled: bool,
    ) {
        if let Some(status) = self
            .file_controls
            .get(&name.to_ascii_uppercase())
            .and_then(|control| control.file_status.as_ref())
        {
            if self.kind(status) == Kind::Text {
                self.move_one(
                    &Expression::StringLiteral(format!("{value:0>2}"), span),
                    status,
                    span,
                );
            } else {
                self.assign(status, &Expression::NumericLiteral(value.to_string(), span));
            }
        } else if value != "0" && !handled {
            let message = self.literal(format!("File {name}: I/O error\n").as_bytes());
            self.emit(format!("la $a0, {message}"));
            self.emit("li $v0, 4");
            self.emit("syscall");
            self.emit("li $v0, 10");
            self.emit("syscall");
        }
    }

    pub(super) fn open(&mut self, node: &OpenStatement) {
        for group in &node.groups {
            for (name, span) in &group.files {
                let Some(number) = self.file_number(name, *span) else {
                    continue;
                };
                let Some(control) = self.file_controls.get(&name.to_ascii_uppercase()) else {
                    self.unsupported(*span, &format!("OPEN {name} without SELECT"));
                    continue;
                };
                if control.organization.as_deref() == Some("INDEXED") {
                    self.open_indexed(name, *span, &group.mode);
                    continue;
                }
                if control
                    .organization
                    .as_deref()
                    .is_some_and(|kind| !kind.eq_ignore_ascii_case("SEQUENTIAL"))
                {
                    self.unsupported(*span, "OPEN of a non-sequential file");
                    continue;
                }
                let flags = match group.mode.as_str() {
                    "INPUT" => 0,
                    "OUTPUT" => 1,
                    "EXTEND" => 9,
                    _ => {
                        self.unsupported(*span, &format!("OPEN {} file", group.mode));
                        continue;
                    }
                };
                let Some(filename) = &control.assign_to else {
                    self.unsupported(*span, &format!("OPEN {name} without ASSIGN"));
                    continue;
                };
                if !self.gen_text(filename) {
                    continue;
                }
                let top = self.new_label("filename_copy");
                let end = self.new_label("filename_end");
                let trim = self.new_label("filename_trim");
                let terminate = self.new_label("filename_terminate");
                let failed = self.new_label("open_failed");
                let done = self.new_label("open_done");
                let already_open = self.new_label("open_already_open");
                self.emit(format!("lw $t0, file_fd_{number}"));
                self.emit(format!("bgez $t0, {already_open}"));
                self.emit("la $t0, tmp_buf");
                self.emit("move $t1, $a0");
                self.emit("move $t2, $a1");
                self.emit("li $t3, 0");
                self.label(&top);
                self.emit(format!("beqz $t2, {end}"));
                self.emit("lbu $t4, 0($t1)");
                self.emit(format!("beqz $t4, {end}"));
                self.emit("li $t5, 255");
                self.emit(format!("bge $t3, $t5, {failed}"));
                self.emit("sb $t4, 0($t0)");
                self.emit("addiu $t0, $t0, 1");
                self.emit("addiu $t1, $t1, 1");
                self.emit("addiu $t2, $t2, -1");
                self.emit("addiu $t3, $t3, 1");
                self.emit(format!("j {top}"));
                self.label(&end);
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
                self.emit("la $a0, tmp_buf");
                self.emit(format!("li $a1, {flags}"));
                self.emit("li $a2, 0");
                self.emit("li $v0, 13");
                self.emit("syscall");
                self.emit(format!("sw $v0, file_fd_{number}"));
                self.emit(format!("bltz $v0, {failed}"));
                self.emit(format!("li $t0, {}", if flags == 0 { 1 } else { 2 }));
                self.emit(format!("sw $t0, file_mode_{number}"));
                self.file_status(name, *span, "0");
                self.emit(format!("j {done}"));
                self.label(&failed);
                self.file_status(name, *span, "30");
                self.emit(format!("j {done}"));
                self.label(&already_open);
                self.file_status(name, *span, "41");
                self.label(&done);
            }
        }
    }

    pub(super) fn close(&mut self, node: &CloseStatement) {
        for (name, span) in &node.files {
            let Some(number) = self.file_number(name, *span) else {
                continue;
            };
            let failed = self.new_label("close_failed");
            let done = self.new_label("close_done");
            self.emit(format!("lw $a0, file_fd_{number}"));
            self.emit(format!("bltz $a0, {failed}"));
            self.emit("li $v0, 16");
            self.emit("syscall");
            self.emit("li $t0, -1");
            self.emit(format!("sw $t0, file_fd_{number}"));
            self.emit(format!("sw $zero, file_mode_{number}"));
            self.file_status(name, *span, "0");
            self.emit(format!("j {done}"));
            self.label(&failed);
            self.file_status(name, *span, "42");
            self.label(&done);
        }
    }

    pub(super) fn write(&mut self, node: &WriteStatement) {
        if let Some((name, _)) = name_and_indices(&node.record)
            && let Some(Place::Item(resolved)) = self.place(name)
            && let Some(file) = resolved.file
            && self
                .file_controls
                .get(file)
                .is_some_and(|control| control.organization.as_deref() == Some("INDEXED"))
        {
            self.write_indexed(node);
            return;
        }
        if node.handlers.invalid_key.is_some()
            || node.handlers.not_invalid_key.is_some()
            || node.handlers.at_end.is_some()
            || node.handlers.not_at_end.is_some()
        {
            self.unsupported(node.span, "sequential WRITE with handlers");
            return;
        }
        let Some((name, _)) = name_and_indices(&node.record) else {
            self.unsupported(node.span, "WRITE record");
            return;
        };
        let Some(Place::Item(resolved)) = self.place(name) else {
            self.unsupported(node.span, "WRITE record");
            return;
        };
        let Some(file) = resolved.file else {
            self.unsupported(node.span, "WRITE of a working-storage item");
            return;
        };
        let file = file.to_string();
        let size = resolved.item().element_size;
        let Some(number) = self.file_number(&file, node.span) else {
            return;
        };
        if let Some(from) = &node.from {
            self.move_one(from, &node.record, node.span);
        }
        let failed = self.new_label("write_failed");
        let bad_mode = self.new_label("write_bad_mode");
        let done = self.new_label("write_done");
        self.emit(format!("lw $t0, file_mode_{number}"));
        self.emit("li $t1, 2");
        self.emit(format!("bne $t0, $t1, {bad_mode}"));
        let advancing_state = node
            .advancing
            .as_ref()
            .map(|advancing| self.prepare_write_advancing(advancing, &failed));
        if let Some(advancing) = &node.advancing
            && !advancing.before
            && let Some((count, byte)) = &advancing_state
        {
            self.write_advancing(number, count, byte, &failed);
        }
        if !self.gen_text(&node.record) {
            return;
        }
        self.emit("move $a1, $a0");
        self.emit(format!("li $a2, {size}"));
        self.emit(format!("lw $a0, file_fd_{number}"));
        self.emit("li $v0, 15");
        self.emit("syscall");
        self.emit(format!("li $t0, {size}"));
        self.emit(format!("bne $v0, $t0, {failed}"));
        if let Some(advancing) = &node.advancing
            && advancing.before
            && let Some((count, byte)) = &advancing_state
        {
            self.write_advancing(number, count, byte, &failed);
        }
        self.file_status(&file, node.span, "0");
        self.emit(format!("j {done}"));
        self.label(&failed);
        self.file_status(&file, node.span, "30");
        self.emit(format!("j {done}"));
        self.label(&bad_mode);
        self.file_status(&file, node.span, "48");
        self.label(&done);
    }

    fn prepare_write_advancing(
        &mut self,
        advancing: &WriteAdvancing,
        failed: &str,
    ) -> (String, String) {
        let count = self.new_word("write_advancing_count");
        let character = if let Some(lines) = &advancing.lines {
            let scale = self.gen_num(lines);
            self.rescale(scale, 0);
            self.emit(format!("bltz $v0, {failed}"));
            self.emit(format!("sw $v0, {count}"));
            b'\n'
        } else {
            self.emit("li $t0, 1");
            self.emit(format!("sw $t0, {count}"));
            b'\x0c'
        };
        let byte = self.literal(&[character]);
        (count, byte)
    }

    fn write_advancing(&mut self, number: usize, count: &str, byte: &str, failed: &str) {
        let top = self.new_label("write_advancing");
        let done = self.new_label("write_advancing_done");
        self.label(&top);
        self.emit(format!("lw $t0, {count}"));
        self.emit(format!("beqz $t0, {done}"));
        self.emit(format!("lw $a0, file_fd_{number}"));
        self.emit(format!("la $a1, {byte}"));
        self.emit("li $a2, 1");
        self.emit("li $v0, 15");
        self.emit("syscall");
        self.emit("li $t0, 1");
        self.emit(format!("bne $v0, $t0, {failed}"));
        self.emit(format!("lw $t0, {count}"));
        self.emit("addiu $t0, $t0, -1");
        self.emit(format!("sw $t0, {count}"));
        self.emit(format!("j {top}"));
        self.label(&done);
    }

    pub(super) fn read(&mut self, node: &ReadStatement) {
        if self
            .file_controls
            .get(&node.file.0.to_ascii_uppercase())
            .is_some_and(|control| control.organization.as_deref() == Some("INDEXED"))
        {
            self.read_indexed(node);
            return;
        }
        if node.key.is_some()
            || node.handlers.invalid_key.is_some()
            || node.handlers.not_invalid_key.is_some()
        {
            self.unsupported(node.span, "keyed READ");
            return;
        }
        let name = &node.file.0;
        let Some(number) = self.file_number(name, node.span) else {
            return;
        };
        if self
            .file_controls
            .get(&name.to_ascii_uppercase())
            .and_then(|control| control.organization.as_deref())
            .is_some_and(|kind| !kind.eq_ignore_ascii_case("SEQUENTIAL"))
        {
            self.unsupported(node.span, "READ of a non-sequential file");
            return;
        }
        let file = &self.layout.files[number];
        let Some(record) = file.records.first() else {
            self.unsupported(node.span, "READ without a record");
            return;
        };
        let record_expr = Expression::Identifier(record.name.clone(), node.span);
        let size = file.record_size;
        let at_end = self.new_label("read_at_end");
        let failed = self.new_label("read_failed");
        let partial = self.new_label("read_partial");
        let bad_mode = self.new_label("read_bad_mode");
        let done = self.new_label("read_done");
        self.emit(format!("lw $t0, file_mode_{number}"));
        self.emit("li $t1, 1");
        self.emit(format!("bne $t0, $t1, {bad_mode}"));
        self.emit(format!("lw $a0, file_fd_{number}"));
        self.emit(format!("la $a1, file_buf_{number}"));
        self.emit(format!("li $a2, {size}"));
        self.emit("li $v0, 14");
        self.emit("syscall");
        self.emit(format!("bltz $v0, {failed}"));
        self.emit(format!("beqz $v0, {at_end}"));
        self.emit(format!("li $t0, {size}"));
        self.emit(format!("bne $v0, $t0, {partial}"));
        if let Some(into) = &node.into {
            self.move_one(&record_expr, into, node.span);
        }
        self.file_status(name, node.span, "0");
        if let Some(statements) = &node.handlers.not_at_end {
            self.statements(statements);
        }
        self.emit(format!("j {done}"));
        self.label(&at_end);
        if node.handlers.at_end.is_none()
            || self
                .file_controls
                .get(&name.to_ascii_uppercase())
                .is_some_and(|control| control.file_status.is_some())
        {
            self.file_status(name, node.span, "10");
        }
        if let Some(statements) = &node.handlers.at_end {
            self.statements(statements);
        }
        self.emit(format!("j {done}"));
        self.label(&failed);
        self.file_status(name, node.span, "30");
        self.emit(format!("j {done}"));
        self.label(&partial);
        self.file_status(name, node.span, "04");
        self.emit(format!("j {done}"));
        self.label(&bad_mode);
        self.file_status(name, node.span, "47");
        self.label(&done);
    }
}
