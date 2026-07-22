//! Span registration for statement, function and branch counters.

use oxc_span::Span;

#[cfg(feature = "satellite-eager-compose")]
use super::RegistrationKey;
use super::{BranchKind, BranchRecord, CoverageTransform, FunctionRecord};

/// Fold key for a branch after eager-compose resolution. The umbrella span
/// and branch kind are deliberately excluded to match Istanbul's arm-vector
/// merge identity.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[cfg(feature = "satellite-eager-compose")]
pub(super) struct BranchKey {
    source: u32,
    arms: Vec<(u32, u32, u32, u32)>,
}

/// One branch arm collected before the umbrella id is chosen.
pub(super) struct PendingArm {
    pub(super) location_span: Option<Span>,
    pub(super) body_span: Span,
}

impl PendingArm {
    pub(super) const fn new(span: Span) -> Self {
        Self { location_span: Some(span), body_span: span }
    }

    pub(super) const fn with_body(location_span: Span, body_span: Span) -> Self {
        Self { location_span: Some(location_span), body_span }
    }

    pub(super) const fn empty_with_body(body_span: Span) -> Self {
        Self { location_span: None, body_span }
    }
}

/// A branch whose arms are collected but whose id is not yet assigned.
pub(super) struct PendingBranch {
    pub(super) kind: BranchKind,
    pub(super) umbrella_span: Span,
    /// When false, every arm must survive because emitted helpers use fixed
    /// arm indices.
    pub(super) gate_arms: bool,
    pub(super) arms: Vec<PendingArm>,
}

/// The umbrella id and per-input-arm surviving slot.
pub(super) struct BranchRegistration {
    pub(super) branch_id: usize,
    path_indices: Vec<Option<usize>>,
}

impl BranchRegistration {
    pub(super) fn slot(&self, arm: usize) -> Option<usize> {
        self.path_indices[arm]
    }
}

#[cfg(feature = "satellite-eager-compose")]
const fn key_parts(key: RegistrationKey) -> (u32, u32, u32, u32) {
    (key.start_line, key.start_column, key.end_line, key.end_column)
}

impl CoverageTransform<'_, '_> {
    #[cfg(feature = "satellite-eager-compose")]
    fn span_maps(&self, span: Span) -> bool {
        self.registration_policy.as_ref().is_none_or(|policy| policy.span_maps(span))
    }

    #[cfg(feature = "satellite-eager-compose")]
    fn registration_key(&self, span: Span) -> Option<RegistrationKey> {
        self.registration_policy.as_ref().and_then(|policy| policy.registration_key(span))
    }

    #[cfg_attr(
        not(feature = "satellite-eager-compose"),
        expect(
            clippy::unnecessary_wraps,
            reason = "the satellite feature can reject or fold a function registration"
        )
    )]
    pub(super) fn add_function(&mut self, name: String, decl: Span, body: Span) -> Option<usize> {
        #[cfg(feature = "satellite-eager-compose")]
        let key = {
            if !self.span_maps(decl) || !self.span_maps(body) {
                return None;
            }
            let key = self.registration_key(decl);
            if let Some(key) = &key
                && let Some(&id) = self.eager_function_ids.get(key)
            {
                if let Some(existing) = self.fn_map.get(id)
                    && (existing.name != name || existing.decl != decl || existing.body != body)
                {
                    self.eager_function_overlay_conflict = true;
                }
                return Some(id);
            }
            key
        };
        let id = self.fn_map.len();
        #[cfg(feature = "satellite-eager-compose")]
        if let Some(key) = key {
            self.eager_function_ids.insert(key, id);
        }
        self.fn_map.push(FunctionRecord { name, decl, body });
        Some(id)
    }

    #[cfg_attr(
        not(feature = "satellite-eager-compose"),
        expect(
            clippy::unnecessary_wraps,
            reason = "the satellite feature can reject or fold a statement registration"
        )
    )]
    pub(super) fn add_statement(&mut self, span: Span) -> Option<usize> {
        #[cfg(feature = "satellite-eager-compose")]
        let key = {
            if !self.span_maps(span) {
                return None;
            }
            let key = self.registration_key(span);
            if let Some(key) = &key
                && let Some(&id) = self.eager_statement_ids.get(key)
            {
                return Some(id);
            }
            key
        };
        let id = self.statement_map.len();
        #[cfg(feature = "satellite-eager-compose")]
        if let Some(key) = key {
            self.eager_statement_ids.insert(key, id);
        }
        self.statement_map.push(span);
        Some(id)
    }

    #[cfg(feature = "satellite-eager-compose")]
    fn eager_branch_key(&self, surviving_arms: &[Option<Span>]) -> Option<BranchKey> {
        let policy = self.registration_policy.as_ref()?;
        let mut source = None;
        let mut arms = Vec::with_capacity(surviving_arms.len());
        for span in surviving_arms {
            // Empty Istanbul locations do not participate in source-map
            // identity. They remain in the arm vector as an empty tuple.
            let Some(span) = span else {
                arms.push((0, 0, 0, 0));
                continue;
            };
            let key = policy.registration_key(*span)?;
            if *source.get_or_insert(key.source) != key.source {
                return None;
            }
            arms.push(key_parts(key));
        }
        Some(BranchKey { source: source?, arms })
    }

    #[cfg_attr(
        not(feature = "satellite-eager-compose"),
        expect(
            clippy::unnecessary_wraps,
            reason = "the satellite feature can reject or fold a branch registration"
        )
    )]
    pub(super) fn register_branch(&mut self, pending: PendingBranch) -> Option<BranchRegistration> {
        let PendingBranch { kind, umbrella_span, gate_arms, arms } = pending;
        #[cfg(not(feature = "satellite-eager-compose"))]
        let _ = gate_arms;
        let mut path_indices = Vec::with_capacity(arms.len());
        let mut surviving_arms = Vec::new();
        let mut body_spans = Vec::new();
        for arm in &arms {
            #[cfg(feature = "satellite-eager-compose")]
            if gate_arms && arm.location_span.is_some_and(|span| !self.span_maps(span)) {
                path_indices.push(None);
                continue;
            }
            path_indices.push(Some(surviving_arms.len()));
            surviving_arms.push(arm.location_span);
            body_spans.push((!is_synthetic_span(arm.body_span)).then_some(arm.body_span));
        }

        #[cfg(feature = "satellite-eager-compose")]
        if self.registration_policy.is_some() && surviving_arms.is_empty() {
            return None;
        }

        #[cfg(feature = "satellite-eager-compose")]
        let entry_span = if self.span_maps(umbrella_span) {
            umbrella_span
        } else {
            surviving_arms.iter().flatten().find(|span| self.span_maps(**span)).copied()?
        };
        #[cfg(not(feature = "satellite-eager-compose"))]
        let entry_span = umbrella_span;
        #[cfg(feature = "satellite-eager-compose")]
        let key = self.eager_branch_key(&surviving_arms);
        #[cfg(feature = "satellite-eager-compose")]
        if let Some(key) = &key
            && let Some(&branch_id) = self.eager_branch_ids.get(key)
        {
            return Some(BranchRegistration { branch_id, path_indices });
        }
        let branch_id = self.branch_map.len();
        #[cfg(feature = "satellite-eager-compose")]
        if let Some(key) = key {
            self.eager_branch_ids.insert(key, branch_id);
        }
        self.branch_map.push(BranchRecord { kind, span: entry_span, arms: surviving_arms });
        self.branch_arm_body_spans.push(body_spans);
        Some(BranchRegistration { branch_id, path_indices })
    }
}

/// True for generated nodes whose span is Oxc's reserved empty span.
pub(super) fn is_synthetic_span(span: Span) -> bool {
    span.start == 0 && span.end == 0
}
