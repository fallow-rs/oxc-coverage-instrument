//! Companion stylesheet and script embedded in the binary and copied
//! next to the emitted pages, so a report directory needs no network
//! and no build step to render.

/// Embedded stylesheet copied to `<output_dir>/base.css`.
pub(super) const BASE_CSS: &str = include_str!("base.css");

/// Palette layer copied to `<output_dir>/coverage-tokens.css`. `base.css`
/// `@import`s it, so a consumer can restyle the report by replacing this
/// one file and leaving the structural rules alone.
pub(super) const COVERAGE_TOKENS_CSS: &str = include_str!("coverage-tokens.css");

/// Embedded enhancement script copied to `<output_dir>/base.js`.
///
/// Provides the sortable index tables and the auto / light / dark theme
/// toggle. Pure DOM API, never assigns HTML strings, never makes a
/// network request, so the page stays compatible with the strict CSP
/// emitted by [`render_page`]. Syntax highlighting is done server-side
/// in Rust via [`syntect`] in the sibling [`highlight`] module.
///
/// [`render_page`]: super::page::render_page
/// [`highlight`]: super::highlight
/// [`syntect`]: https://docs.rs/syntect
pub(super) const BASE_JS: &str = include_str!("base.js");
