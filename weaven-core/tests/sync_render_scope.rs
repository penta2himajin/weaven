/// Sync Scope vs. Render Scope formalization tests (§11.7).
///
/// Validates:
///   1. SmNetworkPolicy registration on World
///   2. policy_filtered_diff: SyncPolicy drives what's included in diffs
///   3. scoped_snapshot: render-scope snapshot from an explicit SM set
///   4. interest_region_sms: spatial interest region query (§8.4)

use weaven_core::*;
use weaven_core::network::*;
use std::collections::{BTreeMap, BTreeSet};

// ── helpers ────────────────────────────────────────────────────────────────

fn make_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [StateId(0), StateId(1)].into_iter().collect(),
        initial_state: StateId(0),
        transitions: vec![Transition {
            id: TransitionId(id.0 * 10),
            source: StateId(0), target: StateId(1),
            priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("go") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports:  vec![],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn two_snapshots() -> (WorldSnapshot, WorldSnapshot) {
    let mut world = World::new();
    let a = SmId(1);
    let b = SmId(2);
    world.register_sm(make_sm(a));
    world.register_sm(make_sm(b));
    world.instances.get_mut(&a).unwrap().context.set("hp", 100.0);
    world.instances.get_mut(&b).unwrap().context.set("hp", 80.0);

    let before = snapshot(&world);

    // Advance: a transitions, b updates context only
    world.instances.get_mut(&a).unwrap().active_state = StateId(1);
    world.instances.get_mut(&a).unwrap().context.set("hp", 90.0);
    world.instances.get_mut(&b).unwrap().context.set("hp", 70.0);
    world.tick += 1;

    let after = snapshot(&world);
    (before, after)
}

// ── 1. Policy registration ─────────────────────────────────────────────────

#[test]
fn test_register_network_policy() {
    let mut world = World::new();
    world.register_sm(make_sm(SmId(1)));

    world.register_network_policy(SmNetworkPolicy {
        sm_id: SmId(1),
        authority: Authority::Server,
        sync_policy: SyncPolicy::StateSync,
        reconciliation: ReconciliationPolicy::Snap,
    });

    let p = world.network_policies.get(&SmId(1)).unwrap();
    assert!(matches!(p.sync_policy, SyncPolicy::StateSync));
}

// ── 2. policy_filtered_diff ────────────────────────────────────────────────

/// SyncPolicy::None → SM excluded from diff entirely.
#[test]
fn test_policy_none_excludes_sm() {
    let (before, after) = two_snapshots();
    let diffs = diff_snapshots(&before, &after);
    assert!(!diffs.is_empty(), "diffs should exist before filtering");

    let mut policies = BTreeMap::new();
    for d in &diffs {
        policies.insert(SmId(d.sm_id), SmNetworkPolicy {
            sm_id: SmId(d.sm_id),
            authority: Authority::Server,
            sync_policy: SyncPolicy::None,
            reconciliation: ReconciliationPolicy::Snap,
        });
    }

    let filtered = policy_filtered_diff(&diffs, &policies);
    assert!(filtered.is_empty(),
        "SyncPolicy::None should exclude all SMs from diff");
}

/// SyncPolicy::StateSync → context_changes stripped, state change preserved.
#[test]
fn test_policy_state_sync_strips_context() {
    let (before, after) = two_snapshots();
    let diffs = diff_snapshots(&before, &after);
    // SM(1) has both state change and context change.
    assert!(diffs.iter().any(|d| d.sm_id == 1 && !d.context_changes.is_empty()));

    let mut policies = BTreeMap::new();
    policies.insert(SmId(1), SmNetworkPolicy {
        sm_id: SmId(1),
        authority: Authority::Server,
        sync_policy: SyncPolicy::StateSync,
        reconciliation: ReconciliationPolicy::Snap,
    });
    // SM(2) uses ContextSync to keep it in the diff.
    policies.insert(SmId(2), SmNetworkPolicy {
        sm_id: SmId(2),
        authority: Authority::Server,
        sync_policy: SyncPolicy::ContextSync { fields: vec!["hp".to_string()] },
        reconciliation: ReconciliationPolicy::Snap,
    });

    let filtered = policy_filtered_diff(&diffs, &policies);
    let sm1 = filtered.iter().find(|d| d.sm_id == 1).unwrap();
    assert!(sm1.context_changes.is_empty(),
        "StateSync: context_changes must be stripped");
    assert_ne!(sm1.prev_state, sm1.new_state,
        "StateSync: state change must be preserved");

    let sm2 = filtered.iter().find(|d| d.sm_id == 2).unwrap();
    assert!(sm2.context_changes.contains_key("hp"),
        "ContextSync: declared field must be present");
}

/// SyncPolicy::ContextSync{fields} → only declared fields survive.
#[test]
fn test_policy_context_sync_field_filter() {
    let mut world = World::new();
    let sm = SmId(1);
    world.register_sm(make_sm(sm));
    world.instances.get_mut(&sm).unwrap().context.set("hp", 100.0);
    world.instances.get_mut(&sm).unwrap().context.set("mp", 50.0);
    world.instances.get_mut(&sm).unwrap().context.set("stamina", 30.0);
    let before = snapshot(&world);

    world.instances.get_mut(&sm).unwrap().context.set("hp", 90.0);
    world.instances.get_mut(&sm).unwrap().context.set("mp", 40.0);
    world.instances.get_mut(&sm).unwrap().context.set("stamina", 25.0);
    world.tick += 1;
    let after = snapshot(&world);

    let diffs = diff_snapshots(&before, &after);
    let mut policies = BTreeMap::new();
    policies.insert(sm, SmNetworkPolicy {
        sm_id: sm,
        authority: Authority::Owner,
        sync_policy: SyncPolicy::ContextSync { fields: vec!["hp".to_string()] },
        reconciliation: ReconciliationPolicy::Interpolate { blend_ticks: 3 },
    });

    let filtered = policy_filtered_diff(&diffs, &policies);
    let d = filtered.iter().find(|d| d.sm_id == 1).unwrap();
    assert!(d.context_changes.contains_key("hp"), "hp should survive");
    assert!(!d.context_changes.contains_key("mp"), "mp should be stripped");
    assert!(!d.context_changes.contains_key("stamina"), "stamina should be stripped");
}

/// SyncPolicy::InputSync → SM excluded from diff (clients simulate locally).
#[test]
fn test_policy_input_sync_excludes_sm() {
    let (before, after) = two_snapshots();
    let diffs = diff_snapshots(&before, &after);

    let mut policies = BTreeMap::new();
    for d in &diffs {
        policies.insert(SmId(d.sm_id), SmNetworkPolicy {
            sm_id: SmId(d.sm_id),
            authority: Authority::Server,
            sync_policy: SyncPolicy::InputSync,
            reconciliation: ReconciliationPolicy::Snap,
        });
    }

    let filtered = policy_filtered_diff(&diffs, &policies);
    assert!(filtered.is_empty(),
        "InputSync: all clients run identical simulation, no state diff needed");
}

/// SMs without a registered policy are included unchanged (permissive default).
#[test]
fn test_policy_missing_policy_passes_through() {
    let (before, after) = two_snapshots();
    let diffs = diff_snapshots(&before, &after);
    let policies: BTreeMap<SmId, SmNetworkPolicy> = BTreeMap::new(); // no policies

    let filtered = policy_filtered_diff(&diffs, &policies);
    assert_eq!(filtered.len(), diffs.len(),
        "SMs without policy should pass through unchanged");
}

// ── 3. scoped_snapshot ────────────────────────────────────────────────────

/// scoped_snapshot only captures the specified SMs.
#[test]
fn test_scoped_snapshot_filters_sms() {
    let mut world = World::new();
    for i in 1u32..=5 {
        world.register_sm(make_sm(SmId(i)));
        world.instances.get_mut(&SmId(i)).unwrap().context.set("v", i as f64);
    }

    let scope: BTreeSet<SmId> = [SmId(2), SmId(4)].into_iter().collect();
    let snap = scoped_snapshot(&world, &scope);

    assert_eq!(snap.instances.len(), 2,
        "scoped_snapshot should contain exactly 2 SM instances");
    let ids: BTreeSet<u32> = snap.instances.iter().map(|s| s.sm_id).collect();
    assert!(ids.contains(&2) && ids.contains(&4));
    assert!(!ids.contains(&1) && !ids.contains(&3) && !ids.contains(&5));
}

/// scoped_snapshot with empty scope returns empty snapshot.
#[test]
fn test_scoped_snapshot_empty_scope() {
    let mut world = World::new();
    world.register_sm(make_sm(SmId(1)));
    let snap = scoped_snapshot(&world, &BTreeSet::new());
    assert!(snap.instances.is_empty());
}

// ── 4. interest_region_sms ────────────────────────────────────────────────

/// interest_region_sms returns SMs within the given radius.
#[test]
fn test_interest_region_sms_basic() {
    let mut world = World::new();
    world.enable_spatial(10.0);

    let near   = SmId(1);
    let far    = SmId(2);
    let origin = SmId(3);

    for id in [near, far, origin] { world.register_sm(make_sm(id)); }
    world.set_position(near,   1.0, 0.0);
    world.set_position(far,  100.0, 0.0);
    world.set_position(origin, 0.0, 0.0);

    // Query centered at origin with radius 15.0
    let visible = interest_region_sms(&world, 0.0, 0.0, 15.0);

    assert!(visible.contains(&near),   "near SM should be in interest region");
    assert!(visible.contains(&origin), "origin SM should be in interest region");
    assert!(!visible.contains(&far),   "far SM should be outside interest region");
}

/// interest_region_sms returns empty set when spatial index is absent (Tier 1).
#[test]
fn test_interest_region_sms_no_spatial_index() {
    let mut world = World::new();
    world.register_sm(make_sm(SmId(1)));
    // No enable_spatial() called → Tier 1

    let visible = interest_region_sms(&world, 0.0, 0.0, 100.0);
    assert!(visible.is_empty(),
        "Without spatial index, interest_region_sms returns empty (caller uses own spatial query)");
}
