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
