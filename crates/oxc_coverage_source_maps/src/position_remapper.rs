//! Standalone position-remap predicate for the eager instrument-time drop gate.

use std::cell::RefCell;

use oxc_coverage_types::{Location, Position};
use srcmap_sourcemap::SourceMap;

use crate::{
    context::{RemapCaches, RemapContext},
    get_mapping::get_mapping_location_cached,
    sources::resolve_primary_source,
};

/// A position-remap predicate over a parsed `inputSourceMap`.
///
/// Lets the instrument crate decide, at AST-transform time, whether a coverage
/// point's positions remap through the input source map, without depending on
/// `srcmap-sourcemap` internals.
pub struct PositionRemapper {
    sm: SourceMap,
    /// Lookup caches shared across every `location_maps` call for this map. The
    /// eager gate queries one node at a time on a hot traversal path, so
    /// without a persistent context the per-`(source, line)` column index would
    /// be rebuilt and discarded on every node. `RefCell` because
    /// `location_maps` takes `&self`: the transform visits with `&self`.
    caches: RefCell<RemapCaches>,
}

impl PositionRemapper {
    /// Parse a source map from JSON.
    ///
    /// Returns `None` when the JSON fails to parse, or when `sources[0]`
    /// resolves to no path (the `resolve_primary_source` bail).
    #[must_use]
    pub fn from_json(input_sm_json: &str) -> Option<Self> {
        let sm = SourceMap::from_json(input_sm_json).ok()?;
        resolve_primary_source(&sm)?;
        Some(Self { sm, caches: RefCell::new(RemapCaches::default()) })
    }

    /// Whether an istanbul `Location` survives `getMapping` resolution, that is
    /// whether `remap_coverage_with_options(.., RemapOptions { drop_unmapped:
    /// true })` would keep the entry.
    ///
    /// The eager AST-level drop gate consults this so it agrees with the
    /// deferred prune by construction: both ask `get_mapping_location`, so a
    /// span that resolves for one resolves for the other. The location is not
    /// mutated; the gate needs only the keep/drop decision, and the position
    /// rewrite happens later in the no-drop `remap_coverage` pass.
    ///
    /// The `line == 0` "unknown" sentinel is a keep no-op, matching the direct
    /// lookup the remap path routes it to.
    #[must_use]
    pub fn location_maps(&self, loc: &Location) -> bool {
        if loc.start.line == 0 || loc.end.line == 0 {
            return self.direct_position_maps(&loc.start) && self.direct_position_maps(&loc.end);
        }
        let mut caches = self.caches.borrow_mut();
        let mut ctx = RemapContext::new(&self.sm, &mut caches);
        get_mapping_location_cached(&mut ctx, loc).is_some()
    }

    /// `true` when the istanbul position (1-based `line`, 0-based UTF-16
    /// `column`) has a direct greatest-lower-bound mapping, or is the
    /// `line == 0` sentinel.
    fn direct_position_maps(&self, pos: &Position) -> bool {
        pos.line == 0 || self.sm.original_position_for(pos.line - 1, pos.column).is_some()
    }
}
