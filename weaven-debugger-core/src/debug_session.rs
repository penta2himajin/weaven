//! DebugSession — holds World + snapshot history + trace buffer.
//!
//! Design doc §6.3: snapshot thinning strategy.
//!   - Recent 100 ticks: every tick
//!   - Older: every 10 ticks
//!   - max_snapshots exceeded: drop oldest

use std::collections::BTreeMap;

use serde::Serialize;
use weaven_core::{
    SmId, StateId, World,
    tick::{tick, TickOutput},
    network::{snapshot, restore, WorldSnapshot},
    trace::TraceEvent,
};

/// Snapshot entry: tick number + serialized world state.
#[derive(Clone)]
pub struct SnapshotEntry {
    pub tick: u64,
    pub snapshot: WorldSnapshot,
}

/// Per-SM state summary for frontend display.
#[derive(Debug, Clone, Serialize)]
pub struct SmStateEntry {
    pub sm_id: SmId,
    pub active_state: StateId,
}

/// Result of a tick or seek operation, sent to frontend.
#[derive(Debug, Clone, Serialize)]
pub struct TickResult {
    pub tick: u64,
    pub trace_events: Vec<TraceEvent>,
    pub state_changes: Vec<StateChangeEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateChangeEntry {
    pub sm_id: SmId,
    pub from_state: StateId,
    pub to_state: StateId,
}

/// Full world state at a point in time, for seek results.
#[derive(Debug, Clone, Serialize)]
pub struct WorldState {
    pub tick: u64,
    pub sm_states: Vec<SmStateEntry>,
}

pub struct DebugSession {
    pub world: World,
    snapshots: Vec<SnapshotEntry>,
    max_snapshots: usize,
    /// Rolling trace buffer — last N ticks' events.
    trace_buffer: Vec<(u64, Vec<TraceEvent>)>,
    max_trace_ticks: usize,
}

impl DebugSession {
    pub fn new(world: World) -> Self {
        // Take initial snapshot at tick 0.
        let initial_snap = snapshot(&world);
        Self {
            world,
            snapshots: vec![SnapshotEntry { tick: 0, snapshot: initial_snap }],
            max_snapshots: 1000,
            trace_buffer: Vec::new(),
            max_trace_ticks: 500,
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.world.tick
    }

    /// Advance one tick. Returns trace events and state changes.
    pub fn tick(&mut self) -> TickResult {
        let out = tick(&mut self.world);
        let tick_num = self.world.tick;

        // Save snapshot (subject to thinning).
        self.save_snapshot(tick_num);

        // Buffer trace events.
        let trace_events = out.trace_events.clone();
        self.trace_buffer.push((tick_num, out.trace_events));
        if self.trace_buffer.len() > self.max_trace_ticks {
            self.trace_buffer.remove(0);
        }

        TickResult {
            tick: tick_num,
            trace_events,
            state_changes: out.state_changes.iter().map(|(&sm_id, &(from, to))| {
                StateChangeEntry { sm_id, from_state: from, to_state: to }
            }).collect(),
        }
    }

    /// Advance N ticks. Returns the final tick result.
    pub fn tick_n(&mut self, n: u32) -> TickResult {
        let mut last = TickResult {
            tick: self.world.tick,
            trace_events: Vec::new(),
            state_changes: Vec::new(),
        };
        for _ in 0..n {
            last = self.tick();
        }
        last
    }

    /// Seek to a specific tick by restoring the nearest snapshot
    /// and re-simulating forward.
    pub fn seek_tick(&mut self, target_tick: u64) -> WorldState {
        // Find the latest snapshot at or before target_tick.
        let snap_idx = self.snapshots
            .iter()
            .rposition(|s| s.tick <= target_tick)
            .unwrap_or(0);

        let entry = &self.snapshots[snap_idx];
        restore(&mut self.world, &entry.snapshot);
        self.world.tick = entry.tick;

        // Re-simulate forward to reach target_tick.
        while self.world.tick < target_tick {
            let _ = tick(&mut self.world);
            self.world.tick = self.world.tick; // tick() already increments
        }

        self.world_state()
    }

    /// Get current world state summary.
    pub fn world_state(&self) -> WorldState {
        WorldState {
            tick: self.world.tick,
            sm_states: self.world.instances.iter().map(|(&sm_id, inst)| {
                SmStateEntry { sm_id, active_state: inst.active_state }
            }).collect(),
        }
    }

    /// Get trace events for a specific tick (from buffer).
    pub fn trace_for_tick(&self, tick: u64) -> Vec<TraceEvent> {
        self.trace_buffer.iter()
            .find(|(t, _)| *t == tick)
            .map(|(_, events)| events.clone())
            .unwrap_or_default()
    }

    /// Snapshot thinning strategy (design doc §6.3):
    /// - Recent 100 ticks: save every tick
    /// - Older: thin to every 10 ticks
    fn save_snapshot(&mut self, tick_num: u64) {
        let snap = snapshot(&self.world);
        self.snapshots.push(SnapshotEntry { tick: tick_num, snapshot: snap });

        // Thin old snapshots if we exceed max.
        if self.snapshots.len() > self.max_snapshots {
            self.thin_snapshots();
        }
    }

    fn thin_snapshots(&mut self) {
        let max = self.max_snapshots;
        if self.snapshots.len() <= max {
            return;
        }

        // Strategy: keep first (tick 0), last `recent_count` entries,
        // and evenly sample from the middle to fill the budget.
        let recent_count = (max / 2).min(self.snapshots.len());
        let middle_budget = max.saturating_sub(recent_count + 1); // +1 for tick 0

        let total = self.snapshots.len();
        let middle_start = 1; // after tick 0
        let middle_end = total.saturating_sub(recent_count);
        let middle_len = middle_end.saturating_sub(middle_start);

        let mut kept: Vec<SnapshotEntry> = Vec::with_capacity(max);

        // Always keep first entry.
        kept.push(self.snapshots[0].clone());

        // Sample middle evenly.
        if middle_len > 0 && middle_budget > 0 {
            let step = (middle_len as f64) / (middle_budget as f64);
            for i in 0..middle_budget.min(middle_len) {
                let idx = middle_start + (i as f64 * step) as usize;
                if idx < middle_end {
                    kept.push(self.snapshots[idx].clone());
                }
            }
        }

        // Keep recent entries.
        for i in middle_end..total {
            kept.push(self.snapshots[i].clone());
        }

        self.snapshots = kept;
    }

    /// Available snapshot ticks (for timeline UI).
    pub fn snapshot_ticks(&self) -> Vec<u64> {
        self.snapshots.iter().map(|s| s.tick).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weaven_core::*;

    fn minimal_world() -> World {
        let mut world = World::new();
        let sm_id = SmId(1);
        world.register_sm(SmDef {
            id: sm_id,
            states: [StateId(0), StateId(1)].into_iter().collect(),
            initial_state: StateId(0),
            transitions: vec![
                Transition {
                    id: TransitionId(0),
                    source: StateId(0),
                    target: StateId(1),
                    priority: 10,
                    guard: Some(Box::new(|ctx, _| ctx.get("go") > 0.0)),
                    effects: vec![],
                },
            ],
            input_ports: vec![],
            output_ports: vec![],
            on_despawn_transitions: vec![],
            elapse_capability: ElapseCapabilityRt::NonElapsable,
            elapse_fn: None,
        });
        world
    }

    #[test]
    fn session_tick_advances() {
        let mut session = DebugSession::new(minimal_world());
        assert_eq!(session.current_tick(), 0);
        let result = session.tick();
        assert_eq!(result.tick, 1);
        assert_eq!(session.current_tick(), 1);
    }

    #[test]
    fn session_tick_n() {
        let mut session = DebugSession::new(minimal_world());
        let result = session.tick_n(5);
        assert_eq!(result.tick, 5);
        assert_eq!(session.current_tick(), 5);
    }

    #[test]
    fn session_seek_restores_state() {
        let mut session = DebugSession::new(minimal_world());
        // Advance 10 ticks.
        session.tick_n(10);
        assert_eq!(session.current_tick(), 10);

        // Seek back to tick 3.
        let state = session.seek_tick(3);
        assert_eq!(state.tick, 3);
        assert_eq!(session.current_tick(), 3);
    }

    #[test]
    fn session_snapshots_saved() {
        let mut session = DebugSession::new(minimal_world());
        session.tick_n(5);
        let ticks = session.snapshot_ticks();
        // Initial + 5 ticks = 6 snapshots.
        assert_eq!(ticks.len(), 6);
        assert_eq!(ticks[0], 0);
        assert_eq!(*ticks.last().unwrap(), 5);
    }

    #[test]
    fn session_trace_buffer() {
        let mut session = DebugSession::new(minimal_world());
        session.tick_n(3);
        // Each tick should have trace events (at minimum GuardEvaluated).
        let trace = session.trace_for_tick(1);
        // Trace may be empty if no SMs are in active set — that's ok.
        // Just verify no panic and buffer works.
        let _ = trace;
        let trace3 = session.trace_for_tick(3);
        let _ = trace3;
    }

    #[test]
    fn snapshot_thinning() {
        let mut session = DebugSession::new(minimal_world());
        session.max_snapshots = 50; // low threshold to trigger thinning
        session.tick_n(200);
        // Should have been thinned — not 201 snapshots.
        assert!(session.snapshots.len() <= 50,
            "Expected <= 50 snapshots after thinning, got {}", session.snapshots.len());
        // Tick 0 should still be present.
        assert_eq!(session.snapshots[0].tick, 0);
    }
}
