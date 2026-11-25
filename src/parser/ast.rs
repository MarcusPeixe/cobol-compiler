use crate::source::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub identification_division: IdentificationDivision,
    pub environment_division: Option<EnvironmentDivision>,
    pub data_division: Option<DataDivision>,
    pub procedure_division: Option<ProcedureDivision>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentificationDivision {
    pub span: Span,
    pub program_id: String,
    pub author: Option<String>,
    pub installation: Option<String>,
    pub date_written: Option<String>,
    pub date_compiled: Option<String>,
    pub security: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentDivision {
    pub span: Span,
    pub configuration_section: Option<ConfigurationSection>,
    pub input_output_section: Option<InputOutputSection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationSection {
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputOutputSection {
    pub span: Span,
    pub file_controls: Vec<FileControl>,
}

/// A `SELECT` entry of the FILE-CONTROL paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct FileControl {
    pub span: Span,
    pub file_name: String,
    pub assign_to: Option<Expression>,
    pub organization: Option<String>,
    pub access_mode: Option<String>,
    pub record_key: Option<Expression>,
    pub alternate_keys: Vec<Expression>,
    pub file_status: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataDivision {
    pub span: Span,
    pub file_section: Option<FileSection>,
    pub working_storage_section: Option<WorkingStorageSection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileSection {
    pub span: Span,
    pub file_descriptions: Vec<FileDescription>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileDescription {
    pub span: Span,
    pub file_name: String,
    pub clauses: Vec<FileClause>,
    pub records: Vec<DataItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileClause {
    BlockContains(String, Span),
    RecordContains(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkingStorageSection {
    pub span: Span,
    pub data_items: Vec<DataItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataItem {
    pub span: Span,
    pub level: u8,
    pub name: String,
    pub picture: Option<String>,
    pub value: Option<Literal>,
    pub occurs: Option<String>,
    pub indexed_by: Vec<String>,
    pub children: Vec<DataItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Numeric(String, Span),
    String(String, Span),
    FigurativeConstant(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureDivision {
    pub span: Span,
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    pub span: Span,
    pub name: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Move(MoveStatement),
    Compute(ComputeStatement),
    Display(DisplayStatement),
    Accept(AcceptStatement),
    Perform(PerformStatement),
    If(IfStatement),
    Add(AddStatement),
    Subtract(SubtractStatement),
    Multiply(MultiplyStatement),
    Divide(DivideStatement),
    StopRun(Span),
    GoTo(GoToStatement),
    Paragraph(String, Span),
    Set(SetStatement),
    Search(SearchStatement),
    Inspect(InspectStatement),
    Evaluate(EvaluateStatement),
    Open(OpenStatement),
    Close(CloseStatement),
    Read(ReadStatement),
    Write(WriteStatement),
    Start(StartStatement),
    Initialize(InitializeStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitializeStatement {
    pub span: Span,
    pub targets: Vec<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetKind {
    To,
    UpBy,
    DownBy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetStatement {
    pub span: Span,
    pub targets: Vec<Expression>,
    pub kind: SetKind,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchStatement {
    pub span: Span,
    /// `SEARCH ALL` (binary search) instead of a serial search.
    pub all: bool,
    pub table: Expression,
    pub varying: Option<Expression>,
    pub at_end: Option<Vec<Statement>>,
    pub whens: Vec<SearchWhen>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchWhen {
    pub span: Span,
    pub condition: Condition,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectMode {
    Characters,
    All,
    Leading,
    First,
}

/// Optional `BEFORE INITIAL` / `AFTER INITIAL` limits of an INSPECT phrase.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InspectRegion {
    pub before: Option<Expression>,
    pub after: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspectTally {
    pub counter: Expression,
    pub mode: InspectMode,
    pub pattern: Option<Expression>,
    pub region: InspectRegion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspectReplace {
    pub mode: InspectMode,
    pub pattern: Option<Expression>,
    pub by: Expression,
    pub region: InspectRegion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspectConvert {
    pub from: Expression,
    pub to: Expression,
    pub region: InspectRegion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InspectStatement {
    pub span: Span,
    pub target: Expression,
    pub tallying: Vec<InspectTally>,
    pub replacing: Vec<InspectReplace>,
    pub converting: Option<InspectConvert>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluateStatement {
    pub span: Span,
    pub subject: Expression,
    pub whens: Vec<EvaluateWhen>,
}

/// One `WHEN` branch (stacked `WHEN` clauses share it); `WHEN OTHER` has `other` set and no choices.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluateWhen {
    pub span: Span,
    pub choices: Vec<EvaluateChoice>,
    pub other: bool,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EvaluateChoice {
    Value(Expression),
    /// `a THRU b`.
    Range(Expression, Expression),
    /// A condition, used when the subject is `TRUE` or `FALSE`.
    Condition(Condition),
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenStatement {
    pub span: Span,
    pub groups: Vec<OpenGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenGroup {
    /// `INPUT`, `OUTPUT`, `I-O` or `EXTEND`.
    pub mode: String,
    pub files: Vec<(String, Span)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloseStatement {
    pub span: Span,
    pub files: Vec<(String, Span)>,
}

/// Imperative clauses shared by READ, WRITE and START.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IoHandlers {
    pub at_end: Option<Vec<Statement>>,
    pub not_at_end: Option<Vec<Statement>>,
    pub invalid_key: Option<Vec<Statement>>,
    pub not_invalid_key: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadStatement {
    pub span: Span,
    pub file: (String, Span),
    pub into: Option<Expression>,
    pub key: Option<Expression>,
    pub handlers: IoHandlers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WriteStatement {
    pub span: Span,
    pub record: Expression,
    pub from: Option<Expression>,
    pub advancing: Option<WriteAdvancing>,
    pub handlers: IoHandlers,
}

/// `BEFORE|AFTER ADVANCING n LINES` or `... PAGE` (`lines` is `None`).
#[derive(Debug, Clone, PartialEq)]
pub struct WriteAdvancing {
    pub before: bool,
    pub lines: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartStatement {
    pub span: Span,
    pub file: (String, Span),
    pub key: Option<(BinaryOperator, Expression)>,
    pub handlers: IoHandlers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddStatement {
    pub span: Span,
    pub operands: Vec<Expression>,
    pub targets: Vec<Expression>,
    pub giving: bool,
    pub giving_targets: Vec<Expression>,
    pub on_size_error: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubtractStatement {
    pub span: Span,
    pub operands: Vec<Expression>,
    pub from: Vec<Expression>,
    pub giving: Vec<Expression>,
    pub on_size_error: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MultiplyStatement {
    pub span: Span,
    pub factor: Expression,
    pub by: Vec<Expression>,
    pub giving: Vec<Expression>,
    pub on_size_error: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DivideStatement {
    pub span: Span,
    pub dividend: Expression,
    pub divisor: Expression,
    pub giving: Vec<Expression>,
    pub remainder: Option<Expression>,
    pub on_size_error: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveStatement {
    pub span: Span,
    pub source: Expression,
    pub targets: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputeStatement {
    pub span: Span,
    pub target: Expression,
    pub expression: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayStatement {
    pub span: Span,
    pub items: Vec<DisplayItem>,
    pub no_advancing: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayItem {
    Expression(Expression),
    String(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcceptStatement {
    pub span: Span,
    pub target: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformStatement {
    ProcedureName(String, Span),
    /// `PERFORM first THRU last`.
    Thru {
        first: String,
        last: String,
        span: Span,
    },
    Times {
        times: Box<Expression>,
        paragraph: String,
        span: Span,
    },
    Until {
        condition: Box<Condition>,
        paragraph: String,
        span: Span,
    },
    Varying(PerformVarying),
    Inline(PerformInline),
}

/// `PERFORM [paragraph] VARYING v FROM a BY b UNTIL cond [statements END-PERFORM]`.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformVarying {
    pub span: Span,
    pub paragraph: Option<String>,
    pub variable: Expression,
    pub from: Expression,
    pub by: Expression,
    pub condition: Condition,
    pub body: Vec<Statement>,
}

/// `PERFORM [n TIMES | UNTIL cond] statements END-PERFORM`.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformInline {
    pub span: Span,
    pub times: Option<Expression>,
    pub condition: Option<Condition>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub span: Span,
    pub condition: Condition,
    pub then_statements: Vec<Statement>,
    pub else_statements: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoToStatement {
    pub span: Span,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(String, Span),
    NumericLiteral(String, Span),
    StringLiteral(String, Span),
    FigurativeConstant(String, Span),
    Subscript {
        name: String,
        indices: Vec<Expression>,
        span: Span,
    },
    /// Reference modification `name(start:length)`.
    RefMod {
        name: String,
        start: Box<Expression>,
        length: Option<Box<Expression>>,
        span: Span,
    },
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Relation {
        left: Expression,
        op: BinaryOperator,
        right: Expression,
        span: Span,
    },
    And {
        left: Box<Condition>,
        right: Box<Condition>,
        span: Span,
    },
    Or {
        left: Box<Condition>,
        right: Box<Condition>,
        span: Span,
    },
    Not {
        condition: Box<Condition>,
        span: Span,
    },
}
