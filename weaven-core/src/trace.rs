//! Debug trace types for the Weaven debugger (§11.3, debugger design §3).
//!
//! Enabled by the `trace` feature flag. When disabled, `TraceCollector`
//! is a zero-sized type and all collection calls are eliminated.

use crate::types::{SmId, StateId, TransitionId, PortId, ConnectionId};
use crate::expr::EvalTreeNode;

// ---------------------------------------------------------------------------
// Phase enum (matches weaven-debugger.als)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "trace", derive(serde::Serialize, serde::Deserialize))]
pub enum Phase {
    Input,
    Evaluate,
    Execute,
    Propagate,
    Lifecycle,
    Output,
}

// ---------------------------------------------------------------------------
// TraceEvent (matches weaven-debugger.als abstract sig TraceEvent)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "trace", derive(serde::Serialize, serde::Deserialize))]
pub enum TraceEvent {
    /// Phase 2/4: Guard evaluated on a Transition.
    GuardEvaluated {
        tick: u64,
        phase: Phase,
        transition: TransitionId,
        sm_id: SmId,
        result: bool,
        /// Context field values at evaluation time (for debugging "why did this guard fail?").
        context_snapshot: Option<Vec<(String, f64)>>,
        /// AST evaluation tree with intermediate values (Phase 8 guard AST visualization).
        eval_tree: Option<EvalTreeNode>,
    },
    /// Phase 2: InteractionRule matched.
    IrMatched {
        tick: u64,
        phase: Phase,
        rule_index: usize,
        participants: Vec<SmId>,
    },
    /// Phase 3/4: Transition fired.
    TransitionFired {
        tick: u64,
        phase: Phase,
        transition: TransitionId,
        sm_id: SmId,
        from_state: StateId,
        to_state: StateId,
    },
    /// Phase 3/4: Signal emitted from an Output Port.
    SignalEmitted {
        tick: u64,
        phase: Phase,
        sm_id: SmId,
        port: PortId,
        target: Option<SmId>,
    },
    /// Phase 4: Each cascade iteration step.
    CascadeStep {
        tick: u64,
        phase: Phase,
        depth: u32,
        queue_size: usize,
    },
    /// Phase 4: Signal blocked by Pipeline Filter.
    PipelineFiltered {
        tick: u64,
        phase: Phase,
        connection: Option<ConnectionId>,
        sm_id: SmId,
        port: PortId,
    },
    /// Phase 4: Individual signal delivered to a target SM during cascade.
    SignalDelivered {
        tick: u64,
        phase: Phase,
        /// Cascade depth at which the signal was delivered.
        depth: u32,
        /// SM that originally emitted the signal (via Connection or spatial routing).
        source_sm: Option<SmId>,
        /// Target SM receiving the signal.
        target_sm: SmId,
        /// Port on the target SM.
        target_port: PortId,
        /// Whether delivery triggered a transition.
        triggered_transition: Option<TransitionId>,
    },
}

// ---------------------------------------------------------------------------
// TraceCollector — zero-cost when trace feature is disabled
// ---------------------------------------------------------------------------

#[cfg(feature = "trace")]
#[derive(Debug, Default)]
pub struct TraceCollector {
    pub events: Vec<TraceEvent>,
}

#[cfg(not(feature = "trace"))]
#[derive(Debug, Default)]
pub struct TraceCollector;

impl TraceCollector {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "trace")]
    #[inline]
    pub fn push(&mut self, event: TraceEvent) {
        self.events.push(event);
    }

    #[cfg(not(feature = "trace"))]
    #[inline(always)]
    pub fn push(&mut self, _event: TraceEvent) {
        // no-op — compiled away
    }

    #[cfg(feature = "trace")]
    pub fn into_events(self) -> Vec<TraceEvent> {
        self.events
    }

    #[cfg(not(feature = "trace"))]
    pub fn into_events(self) -> Vec<TraceEvent> {
        Vec::new()
    }
}
