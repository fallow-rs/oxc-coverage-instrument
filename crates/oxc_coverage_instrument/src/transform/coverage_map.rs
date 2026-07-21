//! Byte-span to istanbul position conversion, and registration of statement,
//! function and branch entries under the eager-compose remap gate.

use oxc_span::Span;

use oxc_coverage_types::{BranchEntry, FnEntry, Location, Position};

use super::CoverageTransform;

/// Identity of a coverage point after eager-compose resolution: the resolved
/// source index plus the remapped original endpoints. Two generated spans with
/// the same key fold into one entry when the canonicalizing remap merges by
/// location, so the eager gate hands them one shared counter id up front.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct EagerMergeKey {
    source: u32,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
}

impl From<(u32, Location)> for EagerMergeKey {
    fn from((source, location): (u32, Location)) -> Self {
        Self {
            source,
            start_line: location.start.line,
            start_column: location.start.column,
            end_line: location.end.line,
            end_column: location.end.column,
        }
    }
}

impl CoverageTransform<'_, '_> {
    /// Whether an istanbul `Location` survives `getMapping` resolution through
    /// the eager-compose input source map, which is the same decision the
    /// deferred `drop_unmapped` prune makes. Resolution is per-span through
    /// `getMapping` rather than per-endpoint through a greatest-lower-bound
    /// lookup, so the two paths agree by construction. `true` when no remapper
    /// is set, so gating is a strict no-op outside eager mode.
    fn location_maps(&self, loc: &Location) -> bool {
        self.eager_remapper.as_ref().is_none_or(|r| r.location_maps(loc))
    }

    /// The eager merge key for `loc`: `None` outside eager mode, where every
    /// registration keeps its own id.
    fn eager_merge_key(&self, loc: &Location) -> Option<EagerMergeKey> {
        self.eager_remapper.as_ref().and_then(|r| r.remap_location(loc)).map(EagerMergeKey::from)
    }

    pub(super) fn span_to_location(&self, span: Span) -> Location {
        Location {
            start: self.offset_to_position(span.start),
            end: self.offset_to_position(span.end),
        }
    }

    fn offset_to_position(&self, offset: u32) -> Position {
        let line = self.line_offsets.partition_point(|&o| o <= offset).saturating_sub(1);
        let line_start = self.line_offsets[line] as usize;
        let end = (offset as usize).min(self.source.len());
        // Istanbul and Babel report columns as UTF-16 code units (JavaScript
        // string indices), not UTF-8 bytes. For an ASCII source the byte
        // distance equals the UTF-16 distance; otherwise the chars have to be
        // walked and their UTF-16 widths summed.
        let column = if self.source_is_ascii {
            end - line_start
        } else {
            self.source[line_start..end].chars().map(char::len_utf16).sum::<usize>()
        };
        Position {
            line: u32::try_from(line + 1).unwrap_or(u32::MAX),
            column: u32::try_from(column).unwrap_or(u32::MAX),
        }
    }

    /// Register a function entry. In eager mode returns `None` when any of the
    /// four endpoints (`decl` start/end, `loc` start/end) fails to remap,
    /// mirroring `prune_functions`: the entry is not pushed and the caller must
    /// skip the function counter. A function whose remapped `decl` collides
    /// with an earlier one gets that entry's id instead of a new one, matching
    /// the decl-keyed fold `merge_file_coverage` applies; the shared counter
    /// then sums the hits the deferred path would merge. Outside eager mode
    /// this always returns `Some` with a fresh id.
    pub(super) fn add_function(
        &mut self,
        name: String,
        decl_span: Span,
        body_span: Span,
    ) -> Option<usize> {
        let decl = self.span_to_location(decl_span);
        let loc = self.span_to_location(body_span);
        if !self.location_maps(&decl) || !self.location_maps(&loc) {
            return None;
        }
        let key = self.eager_merge_key(&decl);
        if let Some(key) = &key
            && let Some(&id) = self.eager_function_ids.get(key)
        {
            return Some(id);
        }
        let id_num = self.fn_map.len();
        if let Some(key) = key {
            self.eager_function_ids.insert(key, id_num);
        }
        let line = decl.start.line;
        self.fn_map.push(FnEntry { name, line, decl, loc });
        Some(id_num)
    }

    /// Register a statement location. In eager mode returns `None` when either
    /// endpoint fails to remap, mirroring `prune_statements`: the location is
    /// not pushed and the caller must skip the statement counter. A statement
    /// whose remapped location collides with an earlier one gets that entry's
    /// id instead of a new one, matching the location-keyed fold
    /// `merge_file_coverage` applies; the shared counter then sums the hits the
    /// deferred path would merge. Outside eager mode this always returns `Some`
    /// with a fresh id.
    pub(super) fn add_statement(&mut self, span: Span) -> Option<usize> {
        let loc = self.span_to_location(span);
        if !self.location_maps(&loc) {
            return None;
        }
        let key = self.eager_merge_key(&loc);
        if let Some(key) = &key
            && let Some(&id) = self.eager_statement_ids.get(key)
        {
            return Some(id);
        }
        let id_num = self.statement_map.len();
        if let Some(key) = key {
            self.eager_statement_ids.insert(key, id_num);
        }
        self.statement_map.push(loc);
        Some(id_num)
    }

    /// Register a branch umbrella entry. In eager mode returns `None` when the
    /// umbrella `loc` start/end fails to remap, mirroring the `prune_branches`
    /// outer-loc rule: nothing is pushed and the caller must skip every counter
    /// under this branch. Outside eager mode this always returns `Some`.
    pub(super) fn add_branch(&mut self, branch_type: &str, span: Span) -> Option<usize> {
        let loc = self.span_to_location(span);
        if !self.location_maps(&loc) {
            return None;
        }
        let id_num = self.branch_map.len();
        let line = loc.start.line;
        self.branch_map.push(BranchEntry {
            loc,
            line,
            branch_type: branch_type.to_string(),
            locations: Vec::new(),
        });
        self.branch_arm_body_byte_spans.push(Vec::new());
        Some(id_num)
    }

    /// Register a branch arm. In eager mode returns `None` when the location
    /// span's start/end fails to remap; the arm is then not pushed, so a
    /// partially-unmapped branch keeps only its mapped arms with contiguous
    /// indices. The caller must skip the arm's counter on `None`. Outside eager
    /// mode this always returns `Some`.
    pub(super) fn add_branch_path(&mut self, branch_id: usize, span: Span) -> Option<usize> {
        let location = self.span_to_location(span);
        if !self.location_maps(&location) {
            return None;
        }
        Some(self.add_branch_path_location(branch_id, location, (span.start, span.end)))
    }

    pub(super) fn add_logical_leaf_paths(&mut self, branch_id: usize, leaf_spans: Vec<Span>) {
        for span in leaf_spans {
            // The per-arm gate is bypassed deliberately: leaf wrapping advances
            // `path_idx` per leaf and requires the arm vec to match the wrapped
            // leaves one for one.
            let location = self.span_to_location(span);
            self.add_branch_path_location(branch_id, location, (span.start, span.end));
        }
    }

    /// Record a branch arm whose istanbul-reported location and the underlying
    /// AST body span differ. Today this is only the if-arm 0 case (istanbul
    /// reports the whole `IfStatement`; the body is the consequent statement).
    /// Gating mirrors [`Self::add_branch_path`] (on the location span).
    pub(super) fn add_branch_path_with_body(
        &mut self,
        branch_id: usize,
        location_span: Span,
        body_span: Span,
    ) -> Option<usize> {
        let location = self.span_to_location(location_span);
        if !self.location_maps(&location) {
            return None;
        }
        Some(self.add_branch_path_location(branch_id, location, (body_span.start, body_span.end)))
    }

    pub(super) fn add_branch_path_location(
        &mut self,
        branch_id: usize,
        location: Location,
        body_byte_span: (u32, u32),
    ) -> usize {
        let entry = self
            .branch_map
            .get_mut(branch_id)
            .expect("branch path must reference an existing branch");
        let path_idx = entry.locations.len();
        entry.locations.push(location);
        let body_spans = self
            .branch_arm_body_byte_spans
            .get_mut(branch_id)
            .expect("branch arm body span vec must exist for every branch id");
        body_spans.push(body_byte_span);
        path_idx
    }
}

/// True for nodes whose byte span is `(0, 0)`: nodes synthesized during the
/// transform that have no anchor in the original source, such as the
/// `typeof X === "function" ? X : Object` guards the legacy decorator
/// metadata pass inserts. Registering a branch for one would inflate the
/// branch denominator with a location that maps back to L1:C0 and that the
/// user cannot act on.
pub(super) fn is_synthetic_span(span: Span) -> bool {
    span.start == 0 && span.end == 0
}
