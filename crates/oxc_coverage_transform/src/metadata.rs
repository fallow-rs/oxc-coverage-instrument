//! Coverage records owned by the AST transform kernel.
//!
//! These types deliberately do not depend on Istanbul serialization or the
//! satellite source-map implementation. The standalone adapter converts them
//! to the satellite data model after traversal.

/// A 1-based source line and 0-based UTF-16 column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformPosition {
    pub line: u32,
    pub column: u32,
}

/// Source range recorded by the transform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformLocation {
    pub start: TransformPosition,
    pub end: TransformPosition,
}

/// Function counter metadata, ordered by counter id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRecord {
    pub name: String,
    pub line: u32,
    pub decl: TransformLocation,
    pub loc: TransformLocation,
}

/// Branch counter metadata, ordered by counter id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRecord {
    pub loc: TransformLocation,
    pub line: u32,
    pub branch_type: String,
    pub locations: Vec<TransformLocation>,
}

/// Complete neutral metadata emitted by one transform traversal.
#[derive(Debug)]
pub struct CoverageMetadata {
    pub statements: Vec<TransformLocation>,
    pub functions: Vec<FunctionRecord>,
    pub branches: Vec<BranchRecord>,
    pub branch_arm_body_byte_spans: Vec<Vec<(u32, u32)>>,
    pub logical_branch_ids: Vec<usize>,
    pub used_optional_chain_helper: bool,
    pub function_overlay_conflict: bool,
}

/// Source-map policy supplied by the satellite adapter.
///
/// The transform only needs keep/drop and canonical identity decisions. It
/// does not parse, compose, or serialize source maps itself.
pub trait RegistrationPolicy {
    /// Whether a generated location should receive a counter.
    fn location_maps(&self, location: &TransformLocation) -> bool;

    /// Resolve a generated location to its canonical source and position.
    fn remap_location(&self, location: &TransformLocation) -> Option<(u32, TransformLocation)>;
}
