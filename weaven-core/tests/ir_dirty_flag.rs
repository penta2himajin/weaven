/// IR dirty-flag optimization tests (§11.2).
///
/// IrWatch::AnySm(set) → rule is skipped unless at least one SM in `set`
/// changed state (was in dirty_sms) during the previous tick.
/// IrWatch::All (default) → evaluated every tick regardless.

use weaven_core::*;

// ── helpers ────────────────────────────────────────────────────────────────

const S0: StateId = StateId(0);
const S1: StateId = StateId(1);
const P0: PortId  = PortId(0);

fn make_simple_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(id.0 * 10),
            source: S0,
            target: S1,
            priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
            effects: vec![],
        }],
        input_ports: vec![Port::new(P0, PortKind::Input, SignalTypeId(0))],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn probe_signal() -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert("probe".to_string(), 1.0);
    Signal { signal_type: SignalTypeId(0), payload: p }
}

// ── tests ──────────────────────────────────────────────────────────────────

/// IrWatch::All (default) — rule fires every tick regardless of dirty state.
#[test]
fn test_ir_watch_all_always_evaluates() {
    let sm1 = SmId(1);
    let sm2 = SmId(2);

    let mut world = World::new();
    world.register_sm(make_simple_sm(sm1));
    world.register_sm(make_simple_sm(sm2));

    // Pre-set sm2 to S1 so the rule condition is satisfied.
    world.instances.get_mut(&sm2).unwrap().active_state = S1;

    let fired = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fired_clone = fired.clone();

    world.register_rule(InteractionRuleDef {
        id: 1,
        group: "test",
        watch: IrWatch::All,
        spatial_condition: None,
        match_fn: Box::new(move |instances| {
            if instances.get(&sm2).map(|i| i.active_state) == Some(S1) {
                fired_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                vec![IrSignal {
                    source_sm: None,
                    target_sm: sm1,
                    target_port: P0,
                    signal: probe_signal(),
                }]
            } else {
                vec![]
            }
        }),
    });

    world.activate(sm1);
    world.activate(sm2);

    // Tick 1: sm2 was pre-set to S1, no dirty transition happened, but IrWatch::All
    // should evaluate regardless.
    tick(&mut world);
    assert!(fired.load(std::sync::atomic::Ordering::SeqCst) >= 1,
        "IrWatch::All: rule should fire on tick 1 even without dirty SMs");

    // Tick 2: still fires.
    tick(&mut world);
    assert!(fired.load(std::sync::atomic::Ordering::SeqCst) >= 2,
        "IrWatch::All: rule should fire on tick 2");
}

/// IrWatch::AnySm — rule is skipped when no watched SM is dirty.
#[test]
fn test_ir_watch_any_sm_skips_when_no_dirty() {
    let sm1 = SmId(1);
    let sm2 = SmId(2);

    let mut world = World::new();
    world.register_sm(make_simple_sm(sm1));
    world.register_sm(make_simple_sm(sm2));

    // Pre-set sm2 to S1 so condition is satisfied if evaluated.
    world.instances.get_mut(&sm2).unwrap().active_state = S1;

    let fired = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fired_clone = fired.clone();

    world.register_rule(InteractionRuleDef {
        id: 2,
        group: "test",
        // Only evaluate when sm1 transitions (is dirty).
        watch: IrWatch::AnySm([sm1].into_iter().collect()),
        spatial_condition: None,
        match_fn: Box::new(move |instances| {
            fired_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if instances.get(&sm2).map(|i| i.active_state) == Some(S1) {
                vec![IrSignal {
                    source_sm: None,
                    target_sm: sm1,
                    target_port: P0,
                    signal: probe_signal(),
                }]
            } else {
                vec![]
            }
        }),
    });

    world.activate(sm1);
    world.activate(sm2);

    // Tick 1: sm1 has NOT transitioned → sm1 not in dirty_sms → rule skipped.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0,
        "IrWatch::AnySm: match_fn should NOT be called when watched SM is clean");
}

/// IrWatch::AnySm — rule fires after watched SM transitions (becomes dirty).
#[test]
fn test_ir_watch_any_sm_fires_after_dirty_transition() {
    let sm1 = SmId(1);
    let sm2 = SmId(2);

    let mut world = World::new();
    world.register_sm(make_simple_sm(sm1));
    world.register_sm(make_simple_sm(sm2));

    let fired = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fired_clone = fired.clone();

    world.register_rule(InteractionRuleDef {
        id: 3,
        group: "test",
        watch: IrWatch::AnySm([sm1].into_iter().collect()),
        spatial_condition: None,
        match_fn: Box::new(move |_instances| {
            fired_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            vec![]
        }),
    });

    world.activate(sm1);
    world.activate(sm2);

    // Tick 1: sm1 does NOT transition → rule skipped.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0,
        "tick 1: should be skipped before sm1 transitions");

    // Trigger sm1 transition S0 → S1.
    world.instances.get_mut(&sm1).unwrap().context.set("trigger", 1.0);
    world.activate(sm1);

    // Tick 2: sm1 transitions in Phase 3 → added to dirty_sms.
    // Phase 2 of tick 2 still sees prev_dirty_sms from tick 1 (empty) → skipped.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0,
        "tick 2: Phase 2 sees prev_dirty_sms from tick 1 (empty) → still skipped");

    // Tick 3: prev_dirty_sms = {{sm1}} (rotated from tick 2 dirty_sms) → rule evaluated.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 1,
        "IrWatch::AnySm: match_fn SHOULD be called when prev_dirty_sms contains watched SM");
}

/// IrWatch::AnySm with multiple watched SMs — fires when any one is dirty.
#[test]
fn test_ir_watch_any_sm_fires_when_any_watched_dirty() {
    let sm1 = SmId(1);
    let sm2 = SmId(2);
    let sm3 = SmId(3);

    let mut world = World::new();
    world.register_sm(make_simple_sm(sm1));
    world.register_sm(make_simple_sm(sm2));
    world.register_sm(make_simple_sm(sm3));

    let fired = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fired_clone = fired.clone();

    world.register_rule(InteractionRuleDef {
        id: 4,
        group: "test",
        // Watching sm1 AND sm2. Fires when either is dirty.
        watch: IrWatch::AnySm([sm1, sm2].into_iter().collect()),
        spatial_condition: None,
        match_fn: Box::new(move |_instances| {
            fired_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            vec![]
        }),
    });

    world.activate(sm1);
    world.activate(sm2);
    world.activate(sm3);

    // Tick 1: neither sm1 nor sm2 dirty → skipped.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0, "tick 1: skipped");

    // sm3 transitions in tick 2 Phase 3 — not watched.
    world.instances.get_mut(&sm3).unwrap().context.set("trigger", 1.0);
    world.activate(sm3);
    tick(&mut world); // tick 2: Phase 2 sees prev_dirty from tick 1 (empty) → skipped
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0, "tick 2 Phase 2: prev_dirty empty → skipped");

    // sm2 transitions in tick 3 Phase 3 — watched.
    world.instances.get_mut(&sm2).unwrap().context.set("trigger", 1.0);
    world.activate(sm2);
    tick(&mut world); // tick 3: Phase 2 sees prev_dirty from tick 2 = {{sm3}} → sm3 not watched → skipped
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0, "tick 3 Phase 2: prev_dirty={{sm3}} not watched → skipped");

    // Tick 4: Phase 2 sees prev_dirty from tick 3 = {{sm2}} → sm2 IS watched → fires.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 1, "tick 4: prev_dirty={{sm2}} → fires");
}

/// dirty_sms is cleared each tick — a rule with AnySm does not keep firing
/// indefinitely after a single transition.
#[test]
fn test_dirty_sms_cleared_each_tick() {
    let sm1 = SmId(1);

    let mut world = World::new();
    world.register_sm(make_simple_sm(sm1));

    let fired = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fired_clone = fired.clone();

    world.register_rule(InteractionRuleDef {
        id: 5,
        group: "test",
        watch: IrWatch::AnySm([sm1].into_iter().collect()),
        spatial_condition: None,
        match_fn: Box::new(move |_| {
            fired_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            vec![]
        }),
    });

    world.activate(sm1);

    // Trigger sm1 transition S0→S1.
    world.instances.get_mut(&sm1).unwrap().context.set("trigger", 1.0);

    // Tick 1: sm1 transitions in Phase 3 → dirty_sms={{sm1}}.
    //         Phase 2 sees prev_dirty from tick 0 (empty) → rule skipped.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 0, "tick 1: Phase 2 sees empty prev_dirty → skipped");

    // Tick 2: prev_dirty={{sm1}} → rule fires.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 1, "tick 2: prev_dirty={{sm1}} → fires");

    // Tick 3: sm1 did NOT transition in tick 2 → prev_dirty={} → rule skipped again.
    tick(&mut world);
    assert_eq!(fired.load(std::sync::atomic::Ordering::SeqCst), 1, "tick 3: prev_dirty empty → NOT fired again");
}
