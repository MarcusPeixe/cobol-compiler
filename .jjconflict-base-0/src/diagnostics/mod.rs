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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::FileId;
    use miette::Diagnostic as _;

    #[test]
    fn severity_ordering_and_letters() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Severe);
        let letters: Vec<char> = [
            Severity::Info,
            Severity::Warning,
            Severity::Error,
            Severity::Severe,
        ]
        .iter()
        .map(|s| s.letter())
        .collect();
        assert_eq!(letters, ['I', 'W', 'E', 'S']);
    }

    #[test]
    fn only_error_and_severe_are_errors() {
        assert!(!Diagnostic::new(Severity::Info, "x", "m").is_error());
        assert!(!Diagnostic::new(Severity::Warning, "x", "m").is_error());
        assert!(Diagnostic::error("x", "m").is_error());
        assert!(Diagnostic::new(Severity::Severe, "x", "m").is_error());
    }

    #[test]
    fn builder_and_display() {
        let span = Span::new(FileId(0), 3, 7);
        let d = Diagnostic::error("c", "boom")
            .with_span(span)
            .with_label("here")
            .with_help("try");
        assert_eq!(d.to_string(), "boom");
        assert_eq!(d.span, Some(span));
        let labels: Vec<_> = d.labels().unwrap().collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].offset(), 3);
        assert_eq!(labels[0].len(), 4);
        assert_eq!(labels[0].label(), Some("here"));
        assert_eq!(d.help().unwrap().to_string(), "try");
        assert_eq!(d.code().unwrap().to_string(), "c");
    }

    #[test]
    fn report_attaches_source_when_span_present() {
        let mut map = SourceMap::default();
        let id = map.add_file("a.cob", "hello world".to_string());
        let d = Diagnostic::error("c", "boom").with_span(Span::new(id, 0, 5));
        let rendered = format!("{:?}", d.into_report(&map));
        assert!(rendered.contains("boom"));
    }
}
