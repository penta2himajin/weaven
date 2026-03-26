/// Network API (§8) — Snapshot/Restore, State Diff, Input Injection.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::types::*;

// §8.1 Authority
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Authority { Server, Owner, Local }

// §8.2 Sync Policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncPolicy {
    InputSync,
    StateSync,
    ContextSync { fields: Vec<String> },
    None,
}

// §8.3 Reconciliation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationPolicy {
    Snap,
    Interpolate { blend_ticks: u32 },
    Rewind,
}

/// Per-SM network policy declaration.
#[derive(Debug, Clone)]
pub struct SmNetworkPolicy {
    pub sm_id:         SmId,
    pub authority:     Authority,
    pub sync_policy:   SyncPolicy,
    pub reconciliation: ReconciliationPolicy,
}

// ---------------------------------------------------------------------------
// Snapshot / Restore (§8.3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmInstanceSnapshot {
    pub sm_id:        u32,
    pub tick:         u64,
    pub active_state: u32,
    pub context:      BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick:       u64,
    pub instances:  Vec<SmInstanceSnapshot>,
    pub active_set: Vec<u32>,
}

impl WorldSnapshot {
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("snapshot serialization")
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Take a snapshot of the current world state.
pub fn snapshot(world: &World) -> WorldSnapshot {
    WorldSnapshot {
        tick: world.tick,
        instances: world.instances.iter().map(|(&sm_id, inst)| SmInstanceSnapshot {
            sm_id:        sm_id.0,
            tick:         world.tick,
            active_state: inst.active_state.0,
            context:      inst.context.scalars.clone(),
        }).collect(),
        active_set: world.active_set.iter().map(|id| id.0).collect(),
    }
}

/// Restore world state from a snapshot (design-time data unchanged).
pub fn restore(world: &mut World, snap: &WorldSnapshot) {
    world.tick = snap.tick;
    world.active_set.clear();
    for id in &snap.active_set { world.active_set.insert(SmId(*id)); }
    for s in &snap.instances {
        if let Some(inst) = world.instances.get_mut(&SmId(s.sm_id)) {
            inst.active_state = StateId(s.active_state);
            inst.context.scalars = s.context.clone();
            inst.context.arrays.clear();
            inst.pending_signals.clear();
        }
    }
    world.signal_queue.clear();
    world.pending_system_commands.clear();
}

// ---------------------------------------------------------------------------
// State Diff (§8)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmStateDiff {
    pub sm_id:           u32,
    pub prev_state:      u32,
    pub new_state:       u32,
    pub context_changes: BTreeMap<String, f64>,
}

/// Compute which SMs changed between two snapshots.
pub fn diff_snapshots(before: &WorldSnapshot, after: &WorldSnapshot) -> Vec<SmStateDiff> {
    let before_map: BTreeMap<u32, &SmInstanceSnapshot> =
        before.instances.iter().map(|s| (s.sm_id, s)).collect();
    after.instances.iter().filter_map(|a| {
        let b = before_map.get(&a.sm_id)?;
        let ctx_changes: BTreeMap<String, f64> = a.context.iter()
            .filter(|(k, v)| b.context.get(*k).map_or(true, |bv| bv != *v))
            .map(|(k, v)| (k.clone(), *v)).collect();
        if b.active_state != a.active_state || !ctx_changes.is_empty() {
            Some(SmStateDiff {
                sm_id: a.sm_id,
                prev_state: b.active_state,
                new_state:  a.active_state,
                context_changes: ctx_changes,
            })
        } else { None }
    }).collect()
}

// ---------------------------------------------------------------------------
// Input Injection & Input Buffer
// ---------------------------------------------------------------------------

/// A tagged input for delivery at a specific tick (lock-step / rollback).
#[derive(Debug, Clone)]
pub struct TaggedInput {
    pub tick:        u64,
    pub target_sm:   SmId,
    pub target_port: PortId,
    pub signal:      Signal,
}

/// Input buffer for rollback networking (configurable history depth).
#[derive(Debug)]
pub struct InputBuffer {
    pub history:       BTreeMap<u64, Vec<TaggedInput>>,
    pub history_depth: u32,
}

impl InputBuffer {
    pub fn new(history_depth: u32) -> Self {
        Self { history: BTreeMap::new(), history_depth }
    }

    pub fn push(&mut self, input: TaggedInput) {
        self.history.entry(input.tick).or_default().push(input);
        let cutoff = self.history.keys().rev()
            .nth(self.history_depth as usize).copied();
        if let Some(oldest) = cutoff {
            self.history.retain(|&t, _| t >= oldest);
        }
    }

    pub fn inputs_at(&self, tick: u64) -> &[TaggedInput] {
        self.history.get(&tick).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Apply inputs for the current world tick.
    pub fn apply_tick_inputs(&self, world: &mut World) {
        for input in self.inputs_at(world.tick) {
            world.inject_signal(input.target_sm, input.target_port, input.signal.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Rewind helper (§8.3)
// ---------------------------------------------------------------------------

/// Roll back to `target_tick` (using `base_snapshot`) and re-simulate to `current_tick`.
pub fn rewind_and_resimulate(
    world:          &mut World,
    base_snapshot:  &WorldSnapshot,
    input_buffer:   &InputBuffer,
    target_tick:    u64,
    current_tick:   u64,
) {
    restore(world, base_snapshot);
    world.tick = target_tick;
    for _t in target_tick..current_tick {
        input_buffer.apply_tick_inputs(world);
        crate::tick::tick(world);
    }
}

// ---------------------------------------------------------------------------
// §11.7 Sync Scope / Render Scope formalization
// ---------------------------------------------------------------------------

/// Filter a diff list according to each SM's registered `SyncPolicy`.
///
/// Rules:
/// - `SyncPolicy::None`      → SM excluded entirely.
/// - `SyncPolicy::InputSync` → SM excluded (all clients run identical simulation;
///                             transmitting state diffs would be redundant).
/// - `SyncPolicy::StateSync` → context_changes stripped; state transition kept.
/// - `SyncPolicy::ContextSync { fields }` → context_changes filtered to declared
///                             fields; unlisted fields removed.
/// - No registered policy   → diff passed through unchanged (permissive default).
pub fn policy_filtered_diff(
    diffs: &[SmStateDiff],
    policies: &std::collections::BTreeMap<SmId, SmNetworkPolicy>,
) -> Vec<SmStateDiff> {
    diffs.iter().filter_map(|d| {
        let policy = match policies.get(&SmId(d.sm_id)) {
            None => return Some(d.clone()),   // no policy → pass through
            Some(p) => p,
        };
        match &policy.sync_policy {
            SyncPolicy::None | SyncPolicy::InputSync => None,
            SyncPolicy::StateSync => Some(SmStateDiff {
                context_changes: BTreeMap::new(),
                ..d.clone()
            }),
            SyncPolicy::ContextSync { fields } => {
                let ctx: BTreeMap<String, f64> = d.context_changes.iter()
                    .filter(|(k, _)| fields.contains(k))
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                // Only emit the diff if something changed (state or allowed context).
                if d.prev_state != d.new_state || !ctx.is_empty() {
                    Some(SmStateDiff { context_changes: ctx, ..d.clone() })
                } else {
                    None
                }
            }
        }
    }).collect()
}

/// Take a snapshot of only the SMs in `sm_ids` (render scope / interest region).
///
/// Used by the presentation layer to build a visibility-filtered view of world
/// state — e.g., only the SMs within a client's fog-of-war region.
pub fn scoped_snapshot(
    world: &World,
    sm_ids: &std::collections::BTreeSet<SmId>,
) -> WorldSnapshot {
    let instances = world.instances.iter()
        .filter(|(id, _)| sm_ids.contains(id))
        .map(|(&sm_id, inst)| SmInstanceSnapshot {
            sm_id:        sm_id.0,
            tick:         world.tick,
            active_state: inst.active_state.0,
            context:      inst.context.scalars.clone(),
        })
        .collect();
    let active_set = world.active_set.iter()
        .filter(|id| sm_ids.contains(id))
        .map(|id| id.0)
        .collect();
    WorldSnapshot { tick: world.tick, instances, active_set }
}

/// Return the set of SM IDs whose registered spatial position falls within
/// a circle of `radius` centered at (`cx`, `cy`).
///
/// Returns an empty set when no spatial index is available (Tier 1 deployment).
/// In Tier 1, the host engine is responsible for its own spatial query.
pub fn interest_region_sms(
    world: &World,
    cx: f32,
    cy: f32,
    radius: f32,
) -> std::collections::BTreeSet<SmId> {
    match &world.spatial_index {
        None => std::collections::BTreeSet::new(),
        Some(spatial) => {
            spatial.query_radius(cx as f64, cy as f64, radius as f64)
                .into_iter()
                .collect()
        }
    }
}
