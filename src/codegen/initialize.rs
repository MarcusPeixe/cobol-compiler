//! INITIALIZE of DISPLAY data items.

use super::Generator;
use super::expr::{Class, Place, class_of, name_and_indices};
use crate::parser::InitializeStatement;
use crate::semantic::ItemLayout;
use crate::source::Span;

impl<'a> Generator<'a> {
    pub(super) fn initialize(&mut self, node: &InitializeStatement) {
        for target in &node.targets {
            let Some((name, indices)) = name_and_indices(target) else {
                self.unsupported(node.span, "INITIALIZE target");
                continue;
            };
            let Some(Place::Item(resolved)) = self.place(name) else {
                self.unsupported(node.span, "INITIALIZE target");
                continue;
            };
            let item = resolved.item();
            let repeat_target = item.occurs.is_some() && indices.len() < resolved.tables().len();
            let base = self.new_word("initialize_base");
            if self.gen_addr(name, indices, node.span).is_none() {
                continue;
            }
            self.emit(format!("sw $a0, {base}"));
            self.initialize_item(
                item,
                item.offset,
                &base,
                &mut Vec::new(),
                repeat_target,
                node.span,
            );
        }
    }

    fn initialize_item(
        &mut self,
        item: &'a ItemLayout,
        root_offset: usize,
        base: &str,
        loops: &mut Vec<(String, usize)>,
        repeat: bool,
        span: Span,
    ) {
        if item.name == "FILLER" {
            return;
        }
        let occurrence_loop = if repeat {
            item.occurs.map(|count| {
                let counter = self.new_word("initialize_count");
                let top = self.new_label("initialize_top");
                self.emit(format!("sw $zero, {counter}"));
                self.label(&top);
                loops.push((counter.clone(), item.element_size));
                (counter, top, count)
            })
        } else {
            None
        };
        if item.children.is_empty() {
            self.emit(format!("lw $a0, {base}"));
            let offset = item.offset - root_offset;
            if offset != 0 {
                self.emit(format!("li $t0, {offset}"));
                self.emit("addu $a0, $a0, $t0");
            }
            for (counter, stride) in loops.iter() {
                self.emit(format!("lw $t0, {counter}"));
                self.emit(format!("li $t1, {stride}"));
                self.emit("mul $t0, $t0, $t1");
                self.emit("addu $a0, $a0, $t0");
            }
            if class_of(item) == Class::Edited {
                let Some(info) = &item.picture else {
                    self.unsupported(span, "INITIALIZE of an item without a picture");
                    return;
                };
                let picture = self.literal(info.pattern.as_bytes());
                self.emit(format!("la $a1, {picture}"));
                self.emit("li $a2, 0");
                self.emit("jal rt_edit");
            } else {
                let fill = if class_of(item) == Class::Plain {
                    b'0'
                } else {
                    b' '
                };
                self.emit(format!("li $a1, {}", item.element_size));
                self.emit(format!("li $a2, {fill}"));
                self.emit("jal rt_fill");
            }
        } else {
            for child in &item.children {
                self.initialize_item(child, root_offset, base, loops, true, span);
            }
        }
        if let Some((counter, top, count)) = occurrence_loop {
            loops.pop();
            self.emit(format!("lw $t0, {counter}"));
            self.emit("addiu $t0, $t0, 1");
            self.emit(format!("sw $t0, {counter}"));
            self.emit(format!("li $t1, {count}"));
            self.emit(format!("blt $t0, $t1, {top}"));
        }
    }
}
