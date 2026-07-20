//! Resolution of a source map's `sources` entries to coverage file paths.

use std::collections::BTreeSet;

use srcmap_sourcemap::SourceMap;

/// Coverage path for `sources[0]`, or `None` when that entry is empty.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn resolve_primary_source(sm: &SourceMap) -> Option<String> {
    resolve_source_path(sm, 0)
}

/// Coverage path for one `sources` entry, `None` when it is absent or empty.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn resolve_source_path(sm: &SourceMap, source: u32) -> Option<String> {
    let source = sm.sources.get(source as usize)?;
    if source.is_empty() {
        return None;
    }
    let root = sm.source_root.as_deref().unwrap_or("");
    if root.is_empty() {
        return Some(source.clone());
    }

    // `srcmap-sourcemap::from_json` pre-joins `sourceRoot` and each source via
    // literal concatenation (spec-strict). `istanbul-lib-source-maps` inserts
    // a `/` separator when `sourceRoot` lacks one. Strip the literal prefix
    // and re-join with Istanbul's separator rule so coverage paths match what
    // existing reporters expect.
    let bare = source.strip_prefix(root).unwrap_or(source.as_str());
    if root.ends_with('/') || bare.starts_with('/') {
        Some(format!("{root}{bare}"))
    } else {
        Some(format!("{root}/{bare}"))
    }
}

/// The one coverage path every `sources` entry resolves to, or `None` when the
/// map declares several distinct sources.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn sole_resolved_source_path(sm: &SourceMap) -> Option<String> {
    let mut paths: BTreeSet<String> = sm
        .sources
        .iter()
        .enumerate()
        .filter_map(|(source, _)| {
            let source = u32::try_from(source).ok()?;
            resolve_source_path(sm, source)
        })
        .collect();
    if paths.len() != 1 {
        return None;
    }
    paths.pop_first()
}
