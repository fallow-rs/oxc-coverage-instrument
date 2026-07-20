//! Caller-supplied knobs for the html reporter and the error returned
//! when a knob is out of range.

/// Tunable knobs for the HTML reporter.
///
/// Construct with [`HtmlOptions::new`] so invalid thresholds cannot bypass
/// CLI-side validation when the library is used directly.
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Percentage cutoff separating green from amber on per-metric
    /// colouring, and the number quoted in the index page's "N of M files
    /// fall below the X% coverage threshold" sentence.
    ///
    /// The amber-to-red boundary stays fixed at 50%. Must be in
    /// `[0.0, 100.0]`. Defaults to `80.0`, Istanbul's traditional value.
    green_threshold: f64,
}

/// Rejected [`HtmlOptions`] argument.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HtmlOptionsError {
    /// The out-of-range threshold the caller passed.
    value: f64,
}

impl std::fmt::Display for HtmlOptionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "html green threshold must be a finite number in [0, 100], got {}", self.value)
    }
}

impl std::error::Error for HtmlOptionsError {}

impl HtmlOptions {
    /// Create options with an explicit green/amber threshold.
    ///
    /// # Errors
    /// Returns [`HtmlOptionsError`] if `green_threshold` is not a finite
    /// number in `[0.0, 100.0]`.
    pub fn new(green_threshold: f64) -> Result<Self, HtmlOptionsError> {
        if !green_threshold.is_finite() || !(0.0..=100.0).contains(&green_threshold) {
            return Err(HtmlOptionsError { value: green_threshold });
        }
        Ok(Self { green_threshold })
    }

    /// The percentage at or above which a metric renders green.
    pub fn green_threshold(&self) -> f64 {
        self.green_threshold
    }
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self { green_threshold: 80.0 }
    }
}
