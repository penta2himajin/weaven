/// Error handling and diagnostics for Weaven Core (§11.5).
///
/// Three classes of runtime errors:
///   1. Stale signal   — signal in queue targeting a despawned SM
///   2. Elapse invalid — elapse_fn returned a state not in def.states
///   3. Cascade limit  — max_cascade_depth exceeded

use std::collections::BTreeMap;
use crate::types::{SmId, StateId, ConnectionId, TransitionId};

// ---------------------------------------------------------------------------
// Error / Diagnostic types
// ---------------------------------------------------------------------------

/// A runtime diagnostic produced by Weaven Core.
/// Collected in `TickOutput.diagnostics` each tick.
#[derive(Debug, Clone)]
pub enum WeavenDiagnostic {
    /// A signal in the delivery queue targeted a SM that no longer exists.
    /// The signal was discarded.
    StaleSignal {
        target_sm:   SmId,
        /// Connection the signal originated from, if known.
        source_conn: Option<ConnectionId>,
    },

    /// An elapse_fn returned a state that is not in the SM's declared state set.
    /// Recovery: fell back to the frozen snapshot state.
    ElapseInvalidState {
        sm_id:          SmId,
        returned_state: StateId,
        fallback_state: StateId,
    },

    /// Phase 4 cascade exceeded max_cascade_depth.
    /// Signals still in the queue at the cutoff were discarded or preserved
    /// depending on `CascadeOverflowPolicy`.
    CascadeDepthExceeded {
        tick:         u64,
        depth_reached: u32,
        /// How many signals were still pending when the limit was hit.
        pending_count: usize,
        /// What action was taken.
        action: CascadeOverflowAction,
    },
}

/// What happens to in-flight signals when cascade depth is exceeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CascadeOverflowAction {
    /// Signals discarded, affected SMs left in their current (mid-cascade) state.
    /// Fast but may leave world in a partially-propagated state.
    DiscardAndContinue,
    /// Signals preserved for delivery in the next tick's Phase 4.
    /// Avoids signal loss but delays propagation.
    DeferToNextTick,
}

// ---------------------------------------------------------------------------
// Recovery policies (configurable on World)
// ---------------------------------------------------------------------------

/// Policy for handling cascade depth overflow (§11.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CascadeOverflowPolicy {
    DiscardAndContinue,
    DeferToNextTick,
}

impl Default for CascadeOverflowPolicy {
    fn default() -> Self { Self::DiscardAndContinue }
}

// ---------------------------------------------------------------------------
// Diagnostic collector
// ---------------------------------------------------------------------------

/// Collected diagnostics for one tick. Accessible via `TickOutput.diagnostics`.
#[derive(Debug, Default)]
pub struct TickDiagnostics {
    pub items: Vec<WeavenDiagnostic>,
}

impl TickDiagnostics {
    pub fn push(&mut self, d: WeavenDiagnostic) {
        self.items.push(d);
    }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn len(&self) -> usize     { self.items.len() }

    /// Returns only stale-signal diagnostics.
    pub fn stale_signals(&self) -> impl Iterator<Item = &WeavenDiagnostic> {
        self.items.iter().filter(|d| matches!(d, WeavenDiagnostic::StaleSignal { .. }))
    }

    /// Returns only cascade-overflow diagnostics.
    pub fn cascade_overflows(&self) -> impl Iterator<Item = &WeavenDiagnostic> {
        self.items.iter().filter(|d| matches!(d, WeavenDiagnostic::CascadeDepthExceeded { .. }))
    }
}
