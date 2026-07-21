//! Coverage records owned by the AST transform kernel.
//!
//! The kernel keeps source locations as Oxc byte spans. Satellite adapters
//! choose their own coordinate system and serialization after traversal.

use oxc_span::Span;

/// Function counter metadata, ordered by counter id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRecord {
    pub name: String,
    pub decl: Span,
    pub body: Span,
}

/// Kind of branch counter recorded by the transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchKind {
    If,
    Switch,
    Conditional,
    Binary,
    DefaultArgument,
    OptionalChain,
}

impl BranchKind {
    /// Istanbul-compatible branch type name used by the satellite adapter.
    pub const fn as_istanbul_str(self) -> &'static str {
        match self {
            Self::If => "if",
            Self::Switch => "switch",
            Self::Conditional => "cond-expr",
            Self::Binary => "binary-expr",
            Self::DefaultArgument => "default-arg",
            Self::OptionalChain => "optional-chain",
        }
    }
}

/// Branch counter metadata, ordered by counter id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRecord {
    pub kind: BranchKind,
    pub span: Span,
    /// One source span per branch arm. `None` represents Istanbul's empty
    /// synthetic location and is distinct from Oxc's generated `SPAN`.
    pub arms: Vec<Option<Span>>,
}

/// Complete neutral metadata emitted by one transform traversal.
#[derive(Debug)]
pub struct CoverageMetadata {
    pub statements: Vec<Span>,
    pub functions: Vec<FunctionRecord>,
    pub branches: Vec<BranchRecord>,
    /// Body spans parallel to `branches[i].arms`. `None` represents an arm
    /// whose runtime body cannot be resolved, such as a synthetic else arm.
    pub branch_arm_body_spans: Vec<Vec<Option<Span>>>,
    pub logical_branch_ids: Vec<usize>,
    pub used_optional_chain_helper: bool,
    pub function_overlay_conflict: bool,
}

/// Stable identity used only while an adapter folds generated coverage
/// points onto one canonical source-map location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc(hidden)]
pub struct RegistrationKey {
    pub source: u32,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Optional registration policy supplied by a satellite adapter.
///
/// The first Oxc-host path can omit this policy and receive every source span.
/// It exists here only because eager source-map composition must decide which
/// counters survive before their ids are embedded in the AST.
#[doc(hidden)]
pub trait RegistrationPolicy {
    /// Whether a generated span should receive a counter.
    fn span_maps(&self, span: Span) -> bool;

    /// Resolve a generated span to its canonical source-map identity.
    fn registration_key(&self, span: Span) -> Option<RegistrationKey>;
}
