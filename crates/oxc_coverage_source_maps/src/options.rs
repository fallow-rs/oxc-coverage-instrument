//! Caller-facing options shared by every remap helper.

/// Options for the remap helpers.
///
/// By default a position whose source-map lookup returns `None` keeps its
/// generated coordinates. In a multi-source map an unmapped entry has no safe
/// original owner and is omitted by the map-returning helpers regardless of
/// this setting.
#[derive(Debug, Clone, Copy, Default)]
pub struct RemapOptions {
    /// When `true`, statement / function / branch entries whose positions
    /// cannot be looked up in the source map are pruned, along with their
    /// matching `s` / `f` / `b` / `bT` hit-count slots.
    ///
    /// Drop semantics mirror `istanbul-lib-source-maps`'s
    /// `transformer.js`:
    /// - **statement**: dropped when either `start` or `end` fails to remap.
    /// - **function**: dropped when any of `decl.start`, `decl.end`,
    ///   `loc.start`, `loc.end` fails to remap. A matching
    ///   `x_fallow_functionMap` overlay entry, if present, drops with it so the
    ///   overlay stays 1:1 with `fnMap`.
    /// - **branch**: per-arm prune when either arm endpoint fails to remap;
    ///   the whole branch is dropped when no arms survive or retained arms
    ///   resolve to different sources. An unmapped umbrella `loc` falls back
    ///   to the first retained arm.
    ///
    /// Defaults to `false`.
    pub drop_unmapped: bool,
}
