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
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileClause {
    Organization(String, Span),
    AccessMode(String, Span),
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
    pub children: Vec<DataItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Numeric(String, Span),
    String(String, Span),
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
    Display(DisplayStatement),
    Accept(AcceptStatement),
    Perform(PerformStatement),
    If(IfStatement),
    StopRun(Span),
    GoTo(GoToStatement),
    Paragraph(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveStatement {
    pub span: Span,
    pub source: Expression,
    pub targets: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayStatement {
    pub span: Span,
    pub items: Vec<DisplayItem>,
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
