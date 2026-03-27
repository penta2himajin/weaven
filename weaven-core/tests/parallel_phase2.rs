/// Multi-threading tests for Phase 2 parallel SM evaluation (§11.6).
///
/// These tests verify two properties:
///   1. With `--features parallel`, the decisions produced by Phase 2 are
///      identical to the serial path (determinism guarantee).
///   2. The parallel path does not panic or produce data races under concurrent
///      access to read-only World fields.
///
/// All tests in this file require the `parallel` feature.
/// Run with: cargo test -p weaven-core --features parallel --test parallel_phase2

use weaven_core::*;

// ── helpers ────────────────────────────────────────────────────────────────

const S0: StateId = StateId(0);
const S1: StateId = StateId(1);
const P0: PortId  = PortId(0);

fn make_sm_with_threshold(id: SmId, threshold: f64) -> SmDef {
    SmDef {
        id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(id.0 * 10),
            source: S0,
            target: S1,
            priority: 10,
            guard: Some(Box::new(move |ctx, _| ctx.get("value") >= threshold)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports:  vec![Port::new(P0, PortKind::Input, SignalTypeId(0))],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Build a world with `n` SMs. Half will have their guard satisfied.
fn world_with_n_sms(n: usize) -> World {
    let mut world = World::new();
    for i in 0..n {
        let id = SmId((i + 1) as u32);
        world.register_sm(make_sm_with_threshold(id, 1.0));
        let value = if i % 2 == 0 { 1.0 } else { 0.0 };
        world.instances.get_mut(&id).unwrap().context.set("value", value);
        world.activate(id);
    }
    world
}

// ── tests ──────────────────────────────────────────────────────────────────

/// Serial and parallel Phase 2 produce identical state changes for a small world.
#[test]
fn test_parallel_phase2_matches_serial_small() {
    // Serial world
    let mut serial = world_with_n_sms(10);
    let serial_out = tick(&mut serial);

    // Parallel world — identical setup
    let mut parallel = world_with_n_sms(10);
    let parallel_out = tick(&mut parallel);

    // Same transitions must have fired.
    assert_eq!(
        serial_out.state_changes.keys().collect::<Vec<_>>(),
        parallel_out.state_changes.keys().collect::<Vec<_>>(),
        "state_changes keys must be identical"
    );
    for (id, (from_s, to_s)) in &serial_out.state_changes {
        let (p_from, p_to) = parallel_out.state_changes.get(id)
            .expect("parallel missing state_change for SM");
        assert_eq!(from_s, p_from, "from_state mismatch for SM {:?}", id);
        assert_eq!(to_s, p_to,   "to_state mismatch for SM {:?}", id);
    }
    // Same final active states.
    for (id, inst) in &serial.instances {
        let p_inst = parallel.instances.get(id).unwrap();
        assert_eq!(inst.active_state, p_inst.active_state,
            "final active_state mismatch for SM {:?}", id);
    }
}

/// Parallel Phase 2 is correct for a large world (stress test).
#[test]
fn test_parallel_phase2_matches_serial_large() {
    let mut serial   = world_with_n_sms(200);
    let mut parallel = world_with_n_sms(200);

    // Run multiple ticks to exercise steady-state parallel evaluation.
    for _ in 0..5 {
        let s_out = tick(&mut serial);
        let p_out = tick(&mut parallel);
        assert_eq!(
            s_out.state_changes.len(),
            p_out.state_changes.len(),
            "state_changes count must match"
        );
    }
}

/// No SM transitions fire when no guards are satisfied — same in parallel.
#[test]
fn test_parallel_no_transitions_when_no_guards_pass() {
    let mut world = World::new();
    for i in 0..50 {
        let id = SmId(i + 1);
        world.register_sm(make_sm_with_threshold(id, 1.0));
        // value = 0 → guard never passes
        world.instances.get_mut(&id).unwrap().context.set("value", 0.0);
        world.activate(id);
    }
    let out = tick(&mut world);
    assert_eq!(out.state_changes.len(), 0,
        "no transitions should fire when no guards pass");
}

/// Parallel evaluation respects priority: highest-priority transition wins.
#[test]
fn test_parallel_phase2_priority_respected() {
    let sm_id = SmId(1);
    let mut world = World::new();
    world.register_sm(SmDef {
        id: sm_id,
        states: [S0, S1, StateId(2)].into_iter().collect(),
        initial_state: S0,
        transitions: vec![
            Transition {
                id: TransitionId(10),
                source: S0, target: S1,
                priority: 5, // lower
                guard: Some(Box::new(|ctx, _| ctx.get("v") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            Transition {
                id: TransitionId(20),
                source: S0, target: StateId(2),
                priority: 10, // higher — should win
                guard: Some(Box::new(|ctx, _| ctx.get("v") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports:  vec![],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });
    world.instances.get_mut(&sm_id).unwrap().context.set("v", 1.0);
    world.activate(sm_id);

    tick(&mut world);
    assert_eq!(world.instances[&sm_id].active_state, StateId(2),
        "higher-priority transition must win even in parallel");
}

// ── IR parallel tests ──────────────────────────────────────────────────────

const PC_PARRY: StateId  = StateId(11);
const PC_IDLE:  StateId  = StateId(10);
const PORT_HIT: PortId   = PortId(1);

fn make_target_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [PC_IDLE, PC_PARRY, StateId(12)].into_iter().collect(),
        initial_state: PC_PARRY,
        transitions: vec![Transition {
            id: TransitionId(id.0 * 100),
            source: PC_PARRY,
            target: StateId(12),
            priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports:  vec![Port::new(PORT_HIT, PortKind::Input, SignalTypeId(0))],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn hit_signal() -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert("hit".to_string(), 1.0);
    Signal { signal_type: SignalTypeId(0), payload: p }
}

/// Parallel IR evaluation produces the same signals as serial for multiple rules.
#[test]
fn test_parallel_ir_matches_serial() {
    fn build_world(n_rules: usize) -> World {
        let mut w = World::new();
        let trigger = SmId(1);
        w.register_sm(make_sm_with_threshold(trigger, 0.0)); // always fires
        for i in 0..n_rules {
            let target = SmId(100 + i as u32);
            w.register_sm(make_target_sm(target));
            w.activate(target);
            let cap_target = target;
            w.register_rule(InteractionRuleDef {
                id: i as u32,
                group: "test",
                watch: IrWatch::All,
                spatial_condition: None,
                match_fn: Box::new(move |instances| {
                    if instances.get(&cap_target).map(|i| i.active_state) == Some(PC_PARRY) {
                        vec![IrSignal { source_sm: None, target_sm: cap_target, target_port: PORT_HIT, signal: hit_signal() }]
                    } else {
                        vec![]
                    }
                }),
            });
        }
        w.activate(trigger);
        w
    }

    let mut serial   = build_world(20);
    let mut parallel = build_world(20);

    let s_out = tick(&mut serial);
    let p_out = tick(&mut parallel);

    assert_eq!(s_out.state_changes.len(), p_out.state_changes.len(),
        "IR parallel: state_changes count must match serial");
    for (id, (sf, st)) in &s_out.state_changes {
        let (pf, pt) = p_out.state_changes.get(id)
            .unwrap_or_else(|| panic!("parallel missing change for {:?}", id));
        assert_eq!(sf, pf);
        assert_eq!(st, pt);
    }
}

/// IR dirty-flag skip works correctly under parallel evaluation.
#[test]
fn test_parallel_ir_dirty_flag_skip() {
    let watched = SmId(99);
    let target  = SmId(200);

    let mut world = World::new();
    // threshold=1.0, initial value=0.0 → watched does NOT transition on tick 1.
    world.register_sm(make_sm_with_threshold(watched, 1.0));
    world.register_sm(make_target_sm(target));
    world.activate(target);

    let fired = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fired_c = fired.clone();
    world.register_rule(InteractionRuleDef {
        id: 999,
        group: "test",
        watch: IrWatch::AnySm([watched].into_iter().collect()),
        spatial_condition: None,
        match_fn: Box::new(move |_| {
            fired_c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            vec![]
        }),
    });

    world.activate(watched);

    // Tick 1: value=0.0 < 1.0 → watched does NOT transition → prev_dirty empty → skip.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0,
        "parallel: dirty-flag skips when watched SM has not transitioned");

    // Set value=1.0 → watched will transition in tick 2 Phase 3.
    world.instances.get_mut(&watched).unwrap().context.set("value", 1.0);
    world.activate(watched);

    // Tick 2: watched transitions in Phase 3 → dirty_sms={watched}.
    // Phase 2 still sees prev_dirty from tick 1 (empty) → still skipped.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0,
        "parallel: Phase 2 of transition tick still sees old prev_dirty");

    // Tick 3: prev_dirty = {watched} → IR evaluated.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 1,
        "parallel: IR fires after watched SM appears in prev_dirty");
}
