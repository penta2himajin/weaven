/// Network API tests (§8): Snapshot/Restore, State Diff, Input Injection, Rewind.

use weaven_core::*;

const SM_A: SmId  = SmId(1);
const SM_B: SmId  = SmId(2);
const S0:   StateId = StateId(0);
const S1:   StateId = StateId(1);
const S2:   StateId = StateId(2);
const PORT: PortId  = PortId(0);
const SIGTYPE: SignalTypeId = SignalTypeId(0);

fn make_sm(id: SmId) -> SmDef {
    SmDef::new(
        id,
        [S0, S1, S2],
        S0,
        vec![
            Transition {
                id: TransitionId(id.0 * 10),
                source: S0, target: S1, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("advance") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            Transition {
                id: TransitionId(id.0 * 10 + 1),
                source: S1, target: S2, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("advance") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        vec![Port::new(PORT, PortKind::Input, SIGTYPE)],
        vec![],
    )
}

fn advance(world: &mut World, sm_id: SmId) {
    if let Some(i) = world.instances.get_mut(&sm_id) { i.context.set("advance", 1.0); }
    world.activate(sm_id);
    tick(world);
    if let Some(i) = world.instances.get_mut(&sm_id) { i.context.set("advance", 0.0); }
}

// ── Snapshot / Restore ─────────────────────────────────────────────────────

#[test]
fn test_snapshot_captures_state() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));

    advance(&mut world, SM_A); // S0 → S1
    let snap = snapshot(&world);

    assert_eq!(snap.tick, world.tick);
    let inst = snap.instances.iter().find(|i| i.sm_id == SM_A.0).unwrap();
    assert_eq!(inst.active_state, S1.0);
}

#[test]
fn test_restore_rolls_back_state() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));

    let snap_t0 = snapshot(&world); // tick 0, S0
    advance(&mut world, SM_A);      // S0 → S1
    advance(&mut world, SM_A);      // S1 → S2

    assert_eq!(world.instances[&SM_A].active_state, S2);

    restore(&mut world, &snap_t0);
    assert_eq!(world.instances[&SM_A].active_state, S0, "restored to S0");
    assert_eq!(world.tick, snap_t0.tick, "tick restored");
}

#[test]
fn test_snapshot_json_round_trip() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));
    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("hp", 75.0); }
    advance(&mut world, SM_A);

    let snap = snapshot(&world);
    let bytes = snap.to_json();
    let restored = WorldSnapshot::from_json(&bytes).unwrap();

    assert_eq!(restored.tick, snap.tick);
    let inst = restored.instances.iter().find(|i| i.sm_id == SM_A.0).unwrap();
    assert_eq!(inst.active_state, S1.0);
    assert_eq!(inst.context.get("hp"), Some(&75.0));
}

#[test]
fn test_restore_clears_signal_queue() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));
    let snap = snapshot(&world);

    // Inject a signal then restore — queue should be cleared
    world.inject_signal(SM_A, PORT, Signal {
        signal_type: SIGTYPE,
        payload: std::collections::BTreeMap::new(),
    });
    assert!(!world.signal_queue.is_empty());

    restore(&mut world, &snap);
    assert!(world.signal_queue.is_empty(), "signal queue cleared on restore");
}

// ── State Diff ──────────────────────────────────────────────────────────────

#[test]
fn test_diff_snapshots_detects_state_change() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));

    let before = snapshot(&world);
    advance(&mut world, SM_A);
    let after = snapshot(&world);

    let diffs = diff_snapshots(&before, &after);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].sm_id, SM_A.0);
    assert_eq!(diffs[0].prev_state, S0.0);
    assert_eq!(diffs[0].new_state, S1.0);
}

#[test]
fn test_diff_snapshots_detects_context_change() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));

    let before = snapshot(&world);
    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("score", 100.0); }
    let after = snapshot(&world);

    let diffs = diff_snapshots(&before, &after);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].context_changes.get("score"), Some(&100.0));
}

#[test]
fn test_diff_snapshots_no_change() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));

    let before = snapshot(&world);
    let after  = snapshot(&world); // no ticks, no changes
    let diffs  = diff_snapshots(&before, &after);
    assert!(diffs.is_empty(), "no changes → empty diff");
}

#[test]
fn test_diff_multiple_sms() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));
    world.register_sm(make_sm(SM_B));

    let before = snapshot(&world);
    advance(&mut world, SM_A); // only SM_A advances
    let after = snapshot(&world);

    let diffs = diff_snapshots(&before, &after);
    assert_eq!(diffs.len(), 1, "only SM_A changed");
    assert_eq!(diffs[0].sm_id, SM_A.0);
}

// ── Input Buffer / Tagged Input ─────────────────────────────────────────────

#[test]
fn test_input_buffer_stores_and_retrieves() {
    let mut buf = InputBuffer::new(10);
    buf.push(TaggedInput {
        tick: 5,
        target_sm:   SM_A,
        target_port: PORT,
        signal: Signal { signal_type: SIGTYPE, payload: std::collections::BTreeMap::new() },
    });

    assert_eq!(buf.inputs_at(5).len(), 1);
    assert_eq!(buf.inputs_at(6).len(), 0);
}

#[test]
fn test_input_buffer_history_pruning() {
    let mut buf = InputBuffer::new(3); // keep 3 ticks
    for t in 0..10u64 {
        buf.push(TaggedInput {
            tick: t, target_sm: SM_A, target_port: PORT,
            signal: Signal { signal_type: SIGTYPE, payload: std::collections::BTreeMap::new() },
        });
    }
    // Should retain roughly the last 3 ticks
    assert!(buf.history.len() <= 4, "history pruned to ~3 ticks, got {}", buf.history.len());
}

#[test]
fn test_input_buffer_apply_injects_signals() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));
    world.tick = 3; // set to tick 3

    let mut buf = InputBuffer::new(10);
    buf.push(TaggedInput {
        tick: 3, target_sm: SM_A, target_port: PORT,
        signal: Signal {
            signal_type: SIGTYPE,
            payload: { let mut p = std::collections::BTreeMap::new(); p.insert("advance".into(), 1.0); p },
        },
    });

    buf.apply_tick_inputs(&mut world);
    assert!(!world.signal_queue.is_empty(), "signal injected for current tick");
}

// ── Rewind ──────────────────────────────────────────────────────────────────

#[test]
fn test_rewind_and_resimulate() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));

    // Tick 0 snapshot: S0
    let snap_t0 = snapshot(&world);

    // Advance to S1
    advance(&mut world, SM_A);
    assert_eq!(world.instances[&SM_A].active_state, S1);
    let tick_after_advance = world.tick;

    // Simulate divergence: client is at S1 but server corrects to stay at S0.
    // Rewind to tick 0, re-simulate 1 tick WITHOUT the advance input.
    let empty_buf = InputBuffer::new(10); // no inputs → SM stays at S0
    rewind_and_resimulate(&mut world, &snap_t0, &empty_buf, 0, tick_after_advance);

    assert_eq!(world.instances[&SM_A].active_state, S0,
        "after rewind+resimulate without inputs, SM stays at S0");
    assert_eq!(world.tick, tick_after_advance,
        "tick counter matches post-rewind target");
}

#[test]
fn test_rewind_with_input_replay() {
    let mut world = World::new();
    world.register_sm(make_sm(SM_A));
    let snap_t0 = snapshot(&world);

    // Build input buffer with advance at tick 1
    let mut buf = InputBuffer::new(10);
    buf.push(TaggedInput {
        tick: 1,
        target_sm: SM_A, target_port: PORT,
        signal: Signal {
            signal_type: SIGTYPE,
            payload: { let mut p = std::collections::BTreeMap::new(); p.insert("advance".into(), 1.0); p },
        },
    });

    // Simulate 2 ticks on original timeline (no advance → stays S0)
    world.activate(SM_A); tick(&mut world); // tick 1 — no input buffer applied
    world.activate(SM_A); tick(&mut world); // tick 2
    assert_eq!(world.instances[&SM_A].active_state, S0, "no advance yet");

    // Rewind to 0, replay with input buffer → should advance to S1 at tick 1
    rewind_and_resimulate(&mut world, &snap_t0, &buf, 0, 2);

    assert_eq!(world.instances[&SM_A].active_state, S1,
        "after rewind+replay with input, SM advanced to S1");
}
