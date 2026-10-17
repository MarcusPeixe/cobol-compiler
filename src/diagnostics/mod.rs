use std::fmt;

use miette::LabeledSpan;

use crate::source::{SourceMap, Span};

/// Severity levels from the specification (I, W, E, S).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Severe,
}

impl Severity {
    pub fn letter(self) -> char {
        match self {
            Severity::Info => 'I',
            Severity::Warning => 'W',
            Severity::Error => 'E',
            Severity::Severe => 'S',
        }
    }

    fn to_miette(self) -> miette::Severity {
        match self {
            Severity::Info => miette::Severity::Advice,
            Severity::Warning => miette::Severity::Warning,
            Severity::Error | Severity::Severe => miette::Severity::Error,
        }
    }
}

/// A compiler message shared by every phase.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Option<Span>,
    pub label: Option<String>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, code: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity,
            code,
            message: message.into(),
            span: None,
            label: None,
            help: None,
        }
    }

    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Errors and severe errors abort compilation.
    pub fn is_error(&self) -> bool {
        self.severity >= Severity::Error
    }

    /// Converts into a `miette::Report` with the right source file attached.
    pub fn into_report(self, map: &SourceMap) -> miette::Report {
        let file = self.span.map(|s| s.file);
        let report = miette::Report::new(self);
        match file {
            Some(file) => report.with_source_code(map.named_source(file)),
            None => report,
        }
    }
}

impl miette::Diagnostic for Diagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(self.code))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity.to_miette())
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let span = self.span?;
        Some(Box::new(std::iter::once(LabeledSpan::new(
            self.label.clone(),
            span.start,
            span.len(),
        ))))
    }
}
