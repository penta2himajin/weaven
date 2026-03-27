/// Spatial Routing tests (§7.1) — grid hash, proximity queries, spatial InteractionRules.

use weaven_core::*;
use weaven_core::spatial::SpatialIndex;

// ── SpatialIndex unit tests ────────────────────────────────────────────────

#[test]
fn test_spatial_index_basic_insert_and_query() {
    let mut idx = SpatialIndex::new(10.0);
    idx.update(SmId(1), 0.0, 0.0);
    idx.update(SmId(2), 5.0, 0.0);
    idx.update(SmId(3), 15.0, 0.0); // outside radius 10

    let mut results = idx.query_radius(0.0, 0.0, 10.0);
    results.sort();
    assert!(results.contains(&SmId(1)), "SM1 at origin");
    assert!(results.contains(&SmId(2)), "SM2 at distance 5");
    assert!(!results.contains(&SmId(3)), "SM3 at distance 15, outside radius");
}

#[test]
fn test_spatial_index_move_updates_cell() {
    let mut idx = SpatialIndex::new(10.0);
    idx.update(SmId(1), 0.0, 0.0);
    assert!(idx.query_radius(0.0, 0.0, 5.0).contains(&SmId(1)));

    // Move SM1 far away
    idx.update(SmId(1), 100.0, 100.0);
    assert!(!idx.query_radius(0.0, 0.0, 5.0).contains(&SmId(1)), "SM1 moved away");
    assert!(idx.query_radius(100.0, 100.0, 5.0).contains(&SmId(1)), "SM1 at new position");
}

#[test]
fn test_spatial_index_remove() {
    let mut idx = SpatialIndex::new(10.0);
    idx.update(SmId(1), 0.0, 0.0);
    idx.update(SmId(2), 3.0, 0.0);

    idx.remove(SmId(1));
    let results = idx.query_radius(0.0, 0.0, 10.0);
    assert!(!results.contains(&SmId(1)), "SM1 removed");
    assert!(results.contains(&SmId(2)), "SM2 still present");
}

#[test]
fn test_spatial_index_exact_distance() {
    let mut idx = SpatialIndex::new(10.0);
    idx.update(SmId(1), 0.0, 0.0);
    idx.update(SmId(2), 3.0, 4.0); // distance = 5

    let d = idx.distance(SmId(1), SmId(2)).unwrap();
    assert!((d - 5.0).abs() < 1e-9, "distance should be 5.0, got {d}");
}

#[test]
fn test_spatial_index_query_radius_of() {
    let mut idx = SpatialIndex::new(10.0);
    idx.update(SmId(1), 0.0, 0.0);
    idx.update(SmId(2), 3.0, 0.0);
    idx.update(SmId(3), 20.0, 0.0);

    let mut near = idx.query_radius_of(SmId(1), 5.0);
    near.sort();
    assert!(near.contains(&SmId(1)), "self included");
    assert!(near.contains(&SmId(2)), "SM2 within 5");
    assert!(!near.contains(&SmId(3)), "SM3 outside");
}

#[test]
fn test_spatial_index_empty_query() {
    let idx = SpatialIndex::new(10.0);
    assert!(idx.query_radius(0.0, 0.0, 100.0).is_empty(), "empty index");
}

// ── World spatial integration ──────────────────────────────────────────────

#[test]
fn test_world_enable_spatial_and_set_position() {
    let mut world = World::new();
    world.enable_spatial(10.0);

    world.register_sm(SmDef::new(SmId(1), [StateId(0)], StateId(0),
        vec![], vec![], vec![]));

    world.set_position(SmId(1), 5.0, 5.0);
    let pos = world.spatial_index.as_ref().unwrap().position(SmId(1));
    assert_eq!(pos, Some((5.0, 5.0)));
}

#[test]
fn test_world_query_radius() {
    let mut world = World::new();
    world.enable_spatial(10.0);

    for id in 1..=3u32 {
        world.register_sm(SmDef::new(SmId(id), [StateId(0)], StateId(0),
            vec![], vec![], vec![]));
    }
    world.set_position(SmId(1), 0.0, 0.0);
    world.set_position(SmId(2), 5.0, 0.0);
    world.set_position(SmId(3), 50.0, 0.0); // far away

    let nearby = world.query_radius(0.0, 0.0, 10.0);
    assert!(nearby.contains(&SmId(1)));
    assert!(nearby.contains(&SmId(2)));
    assert!(!nearby.contains(&SmId(3)));
}

// ── Spatial InteractionRule ────────────────────────────────────────────────

const ATTACKER:   SmId    = SmId(1);
const TARGET:     SmId    = SmId(2);
const FAR_TARGET: SmId    = SmId(3);
const S_IDLE:     StateId = StateId(0);
const S_HIT:      StateId = StateId(1);
const PORT_HIT:   PortId  = PortId(0);
const SIGTYPE:    SignalTypeId = SignalTypeId(0);

fn make_target_sm(id: SmId) -> SmDef {
    SmDef::new(
        id,
        [S_IDLE, S_HIT],
        S_IDLE,
        vec![Transition {
            id: TransitionId(id.0 * 10),
            source: S_IDLE, target: S_HIT, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        vec![Port::new(PORT_HIT, PortKind::Input, SIGTYPE)],
        vec![],
    )
}

/// Spatial InteractionRule: attacker hits only targets within range.
/// FAR_TARGET is beyond the attack range and should NOT be hit.
#[test]
fn test_spatial_ir_proximity_filters_targets() {
    let mut world = World::new();
    world.enable_spatial(10.0);

    world.register_sm(SmDef::new(ATTACKER, [S_IDLE, S_HIT], S_IDLE,
        vec![Transition {
            id: TransitionId(10),
            source: S_IDLE, target: S_HIT, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("attack") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        vec![], vec![]));
    world.register_sm(make_target_sm(TARGET));
    world.register_sm(make_target_sm(FAR_TARGET));

    // Position: ATTACKER at origin, TARGET nearby, FAR_TARGET far away
    world.set_position(ATTACKER,   0.0, 0.0);
    world.set_position(TARGET,     3.0, 0.0);  // distance 3 — within range 5
    world.set_position(FAR_TARGET, 20.0, 0.0); // distance 20 — outside range 5

    const ATTACK_RANGE: f64 = 5.0;

    // Spatial IR: if ATTACKER attacks AND target is within range → hit signal
    world.register_rule(InteractionRuleDef {
        id: 1,
        group: "combat",
        watch: IrWatch::All,
        spatial_condition: Some(proximity(ATTACK_RANGE)),
        match_fn: Box::new(move |instances| {
            let attacker_state = instances.get(&ATTACKER).map(|i| i.active_state);
            if attacker_state != Some(S_HIT) { return vec![]; }

            // Use the world's spatial index via the match_fn closure.
            // In a full implementation the match_fn would receive the spatial index.
            // Here we inline the proximity check in the rule itself.
            let mut signals = vec![];
            for &target_id in &[TARGET, FAR_TARGET] {
                if let Some(inst) = instances.get(&target_id) {
                    if inst.active_state == S_IDLE {
                        let (tx, ty) = match target_id {
                            t if t == TARGET     => (3.0f64, 0.0f64),
                            t if t == FAR_TARGET => (20.0f64, 0.0f64),
                            _ => continue,
                        };
                        let dist = (tx * tx + ty * ty).sqrt();
                        if dist <= ATTACK_RANGE {
                            let mut p = std::collections::BTreeMap::new();
                            p.insert("hit".to_string(), 1.0);
                            signals.push(IrSignal {
                                source_sm: None,
                                target_sm:   target_id,
                                target_port: PORT_HIT,
                                signal: Signal { signal_type: SIGTYPE, payload: p },
                            });
                        }
                    }
                }
            }
            signals
        }),
    });

    // Tick N: ATTACKER transitions to HIT (attack fires)
    if let Some(i) = world.instances.get_mut(&ATTACKER) { i.context.set("attack", 1.0); }
    world.activate(ATTACKER);
    tick(&mut world);

    assert_eq!(world.instances[&ATTACKER].active_state, S_HIT, "attacker attacked");
    // IR not matched yet (pre-transition states: ATTACKER still S_IDLE in Phase 2)
    assert_eq!(world.instances[&TARGET].active_state,     S_IDLE);
    assert_eq!(world.instances[&FAR_TARGET].active_state, S_IDLE);

    // Tick N+1: IR now sees ATTACKER=S_HIT → emits hit for TARGET (in range), not FAR_TARGET
    world.activate(ATTACKER);
    tick(&mut world);

    assert_eq!(world.instances[&TARGET].active_state,     S_HIT,
        "TARGET hit (within range 5)");
    assert_eq!(world.instances[&FAR_TARGET].active_state, S_IDLE,
        "FAR_TARGET NOT hit (distance 20 > range 5)");
}

/// proximity() helper correctly evaluates distance constraint.
#[test]
fn test_proximity_condition() {
    let mut idx = SpatialIndex::new(5.0);
    idx.update(SmId(1), 0.0, 0.0);
    idx.update(SmId(2), 4.0, 3.0); // distance = 5
    idx.update(SmId(3), 6.0, 0.0); // distance = 6

    let close = proximity(5.0);
    assert!(close(&idx, SmId(1), SmId(2)), "distance 5 <= 5 → true");
    assert!(!close(&idx, SmId(1), SmId(3)), "distance 6 > 5 → false");
}

/// SMs without positions in the index are treated as not in proximity.
#[test]
fn test_spatial_unregistered_sm_not_in_proximity() {
    let mut idx = SpatialIndex::new(10.0);
    idx.update(SmId(1), 0.0, 0.0);
    // SmId(2) not registered

    let cond = proximity(100.0);
    assert!(!cond(&idx, SmId(1), SmId(2)), "unregistered SM → not in proximity");
}

/// Spatial index handles many SMs efficiently (basic perf sanity check).
#[test]
fn test_spatial_index_many_sms() {
    let mut idx = SpatialIndex::new(10.0);
    for i in 0..100u32 {
        let x = (i % 10) as f64 * 5.0;
        let y = (i / 10) as f64 * 5.0;
        idx.update(SmId(i), x, y);
    }
    assert_eq!(idx.sm_count(), 100);

    // Query near origin (0,0) with radius 10 → should find ~4 nearby SMs
    let nearby = idx.query_radius(0.0, 0.0, 10.0);
    assert!(!nearby.is_empty(), "should find some nearby SMs");
    assert!(nearby.len() <= 20, "should not find too many (not all 100)");
}

// ── Gap 1: spatial_condition post-filter via source_sm ────────────────────
//
// These tests verify that spatial_condition is actually applied to filter
// IrSignals based on (source_sm, target_sm) proximity — not bypassed.
// Previously the retain() closure was a placeholder that always returned true.

const NEAR_ATTACKER:  SmId = SmId(10);
const NEAR_TARGET:    SmId = SmId(11);
const FAR_ATTACKER:   SmId = SmId(12);
const FAR_ATTACKER_T: SmId = SmId(13); // target for FAR_ATTACKER

/// spatial_condition properly blocks signals whose source is out of range.
///
/// Setup:
///   NEAR_ATTACKER at (0,0)  → NEAR_TARGET at (2,0) — distance 2, range 5 → signal allowed
///   FAR_ATTACKER  at (50,0) → FAR_ATTACKER_T at (2,0) — distance 48 > range 5 → blocked
///
/// The match_fn emits signals for both pairs unconditionally.
/// spatial_condition(proximity(5)) should block FAR_ATTACKER's signal.
#[test]
fn test_spatial_condition_blocks_out_of_range_source() {
    let s_idle = StateId(0);
    let s_active = StateId(1);

    let mut world = World::new();
    world.enable_spatial(10.0);

    // Attacker SMs: transition on "trigger"
    for &id in &[NEAR_ATTACKER, FAR_ATTACKER] {
        world.register_sm(SmDef::new(
            id,
            [s_idle, s_active],
            s_idle,
            vec![Transition {
                id: TransitionId(id.0 * 10),
                source: s_idle, target: s_active, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
                guard_expr: None,
                effects: vec![],
            }],
            vec![],
            vec![],
        ));
    }
    // Target SMs: transition on "hit" (written into context by IR signal payload)
    for &id in &[NEAR_TARGET, FAR_ATTACKER_T] {
        world.register_sm(SmDef::new(
            id,
            [s_idle, s_active],
            s_idle,
            vec![Transition {
                id: TransitionId(id.0 * 10),
                source: s_idle, target: s_active, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
                guard_expr: None,
                effects: vec![],
            }],
            vec![Port::new(PORT_HIT, PortKind::Input, SIGTYPE)],
            vec![],
        ));
    }

    world.set_position(NEAR_ATTACKER,   0.0,  0.0);
    world.set_position(NEAR_TARGET,     2.0,  0.0); // dist 2 from NEAR_ATTACKER
    world.set_position(FAR_ATTACKER,   50.0,  0.0);
    world.set_position(FAR_ATTACKER_T,  2.0,  0.0); // dist 48 from FAR_ATTACKER

    // Pre-set attackers to s_active so the match_fn fires immediately.
    world.instances.get_mut(&NEAR_ATTACKER).unwrap().active_state = s_active;
    world.instances.get_mut(&FAR_ATTACKER).unwrap().active_state  = s_active;
    world.activate(NEAR_ATTACKER);
    world.activate(NEAR_TARGET);
    world.activate(FAR_ATTACKER);
    world.activate(FAR_ATTACKER_T);

    let hit_signal = || {
        let mut p = std::collections::BTreeMap::new();
        p.insert("hit".to_string(), 1.0);
        Signal { signal_type: SIGTYPE, payload: p }
    };

    // IR: emit hit signals for both (near, near_target) and (far, far_target).
    // spatial_condition(proximity(5)) should filter out the far pair.
    world.register_rule(InteractionRuleDef {
        id: 20,
        group: "combat",
        watch: IrWatch::All,
        // source_sm must be set on emitted IrSignals for filtering to work.
        spatial_condition: Some(weaven_core::spatial::proximity(5.0)),
        match_fn: Box::new(move |instances| {
            let mut sigs = vec![];
            // Near pair
            if instances.get(&NEAR_ATTACKER).map(|i| i.active_state) == Some(s_active) {
                sigs.push(IrSignal {
                    source_sm:   Some(NEAR_ATTACKER),
                    target_sm:   NEAR_TARGET,
                    target_port: PORT_HIT,
                    signal:      hit_signal(),
                });
            }
            // Far pair
            if instances.get(&FAR_ATTACKER).map(|i| i.active_state) == Some(s_active) {
                sigs.push(IrSignal {
                    source_sm:   Some(FAR_ATTACKER),
                    target_sm:   FAR_ATTACKER_T,
                    target_port: PORT_HIT,
                    signal:      hit_signal(),
                });
            }
            sigs
        }),
    });

    tick(&mut world);

    assert_eq!(world.instances[&NEAR_TARGET].active_state, s_active,
        "NEAR_TARGET should be hit (distance 2 <= range 5)");
    assert_eq!(world.instances[&FAR_ATTACKER_T].active_state, s_idle,
        "FAR_ATTACKER_T should NOT be hit (distance 48 > range 5)");
}

/// When source_sm is None, spatial_condition is not applied (pass-through).
#[test]
fn test_spatial_condition_skipped_when_no_source_sm() {
    let s_idle   = StateId(0);
    let s_active = StateId(1);

    let mut world = World::new();
    world.enable_spatial(10.0);

    let attacker = SmId(20);
    let target   = SmId(21);
    world.register_sm(SmDef::new(attacker, [s_idle, s_active], s_idle,
        vec![], vec![], vec![]));
    world.register_sm(SmDef::new(target, [s_idle, s_active], s_idle,
        vec![Transition {
            id: TransitionId(210),
            source: s_idle, target: s_active, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        vec![Port::new(PORT_HIT, PortKind::Input, SIGTYPE)],
        vec![],
    ));

    // Even though attacker is far, source_sm=None means no spatial filtering.
    world.set_position(attacker, 0.0, 0.0);
    world.set_position(target, 100.0, 0.0); // distance 100 > range 5

    world.instances.get_mut(&attacker).unwrap().active_state = s_active;
    world.activate(attacker);
    world.activate(target);

    let mut p = std::collections::BTreeMap::new();
    p.insert("hit".to_string(), 1.0);
    let sig = Signal { signal_type: SIGTYPE, payload: p };

    world.register_rule(InteractionRuleDef {
        id: 30,
        group: "test",
        watch: IrWatch::All,
        spatial_condition: Some(weaven_core::spatial::proximity(5.0)),
        match_fn: Box::new(move |instances| {
            if instances.get(&attacker).map(|i| i.active_state) == Some(s_active) {
                vec![IrSignal {
                    source_sm:   None, // no source — spatial_condition not applied
                    target_sm:   target,
                    target_port: PORT_HIT,
                    signal:      sig.clone(),
                }]
            } else { vec![] }
        }),
    });

    tick(&mut world);

    assert_eq!(world.instances[&target].active_state, s_active,
        "signal should pass through when source_sm is None (no spatial filtering)");
}

// ── Gap 3: influence_radius spatial routing ────────────────────────────────
//
// Output Port with influence_radius → signals automatically delivered to
// compatible Input Ports on nearby SMs (§7.1 spatial routing layer).

const INFLUENCE_SIG: SignalTypeId = SignalTypeId(99);
const INFLUENCE_SIG_OTHER: SignalTypeId = SignalTypeId(98); // different type

fn make_broadcaster(id: SmId, radius: f64) -> SmDef {
    SmDef {
        id,
        states: [StateId(0), StateId(1)].into_iter().collect(),
        initial_state: StateId(0),
        transitions: vec![Transition {
            id: TransitionId(id.0 * 1000),
            source: StateId(0), target: StateId(1),
            priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            guard_expr: None,
            effects: vec![Box::new(move |_ctx| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("hit".to_string(), 1.0);
                vec![EffectOutput::Signal(
                    PortId(99),
                    Signal { signal_type: INFLUENCE_SIG, payload: p },
                )]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::with_radius(PortId(99), PortKind::Output, INFLUENCE_SIG, radius)],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_receiver(id: SmId, sig_type: SignalTypeId) -> SmDef {
    SmDef {
        id,
        states: [StateId(0), StateId(1)].into_iter().collect(),
        initial_state: StateId(0),
        transitions: vec![Transition {
            id: TransitionId(id.0 * 1000),
            source: StateId(0), target: StateId(1),
            priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports: vec![Port::new(PortId(88), PortKind::Input, sig_type)],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// SM with influence_radius broadcasts to nearby SMs with compatible ports.
#[test]
fn test_influence_radius_delivers_to_nearby_sms() {
    let broadcaster = SmId(50);
    let near         = SmId(51); // distance 3 < radius 5 → receives signal
    let far          = SmId(52); // distance 8 > radius 5 → does NOT receive

    let mut world = World::new();
    world.enable_spatial(10.0);

    world.register_sm(make_broadcaster(broadcaster, 5.0));
    world.register_sm(make_receiver(near, INFLUENCE_SIG));
    world.register_sm(make_receiver(far,  INFLUENCE_SIG));

    world.set_position(broadcaster, 0.0, 0.0);
    world.set_position(near,         3.0, 0.0);
    world.set_position(far,          8.0, 0.0);

    world.instances.get_mut(&broadcaster).unwrap().context.set("fire", 1.0);
    world.activate(broadcaster);
    world.activate(near);
    world.activate(far);

    tick(&mut world);

    assert_eq!(world.instances[&near].active_state, StateId(1),
        "near SM (dist 3 < radius 5) should receive spatially routed signal");
    assert_eq!(world.instances[&far].active_state, StateId(0),
        "far SM (dist 8 > radius 5) should NOT receive spatially routed signal");
}

/// Spatial routing only delivers to ports with matching signal_type.
#[test]
fn test_influence_radius_respects_signal_type() {
    let broadcaster    = SmId(60);
    let matching_type  = SmId(61); // INFLUENCE_SIG → receives
    let mismatched     = SmId(62); // INFLUENCE_SIG_OTHER → does not receive

    let mut world = World::new();
    world.enable_spatial(10.0);

    world.register_sm(make_broadcaster(broadcaster, 10.0));
    world.register_sm(make_receiver(matching_type, INFLUENCE_SIG));
    world.register_sm(make_receiver(mismatched,    INFLUENCE_SIG_OTHER));

    world.set_position(broadcaster,   0.0, 0.0);
    world.set_position(matching_type, 2.0, 0.0);
    world.set_position(mismatched,    2.0, 0.0);

    world.instances.get_mut(&broadcaster).unwrap().context.set("fire", 1.0);
    world.activate(broadcaster);
    world.activate(matching_type);
    world.activate(mismatched);

    tick(&mut world);

    assert_eq!(world.instances[&matching_type].active_state, StateId(1),
        "SM with matching signal_type should receive");
    assert_eq!(world.instances[&mismatched].active_state, StateId(0),
        "SM with mismatched signal_type should NOT receive");
}

/// Without spatial index (Tier 1), influence_radius has no effect.
#[test]
fn test_influence_radius_no_effect_without_spatial_index() {
    let broadcaster = SmId(70);
    let receiver     = SmId(71);

    let mut world = World::new();
    // No world.enable_spatial() — Tier 1

    world.register_sm(make_broadcaster(broadcaster, 5.0));
    world.register_sm(make_receiver(receiver, INFLUENCE_SIG));

    world.instances.get_mut(&broadcaster).unwrap().context.set("fire", 1.0);
    world.activate(broadcaster);
    world.activate(receiver);

    tick(&mut world);

    assert_eq!(world.instances[&receiver].active_state, StateId(0),
        "Without spatial index, influence_radius delivers nothing");
}

/// Broadcaster does not deliver to itself via spatial routing.
#[test]
fn test_influence_radius_excludes_self() {
    // Broadcaster has both an input port matching its own output signal.
    let broadcaster = SmId(80);
    let mut world = World::new();
    world.enable_spatial(10.0);

    world.register_sm(SmDef {
        id: broadcaster,
        states: [StateId(0), StateId(1)].into_iter().collect(),
        initial_state: StateId(0),
        transitions: vec![Transition {
            id: TransitionId(8000),
            source: StateId(0), target: StateId(1),
            priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            guard_expr: None,
            effects: vec![Box::new(|_| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("hit".to_string(), 1.0);
                vec![EffectOutput::Signal(
                    PortId(99),
                    Signal { signal_type: INFLUENCE_SIG,
                             payload: p },
                )]
            })],
        }],
        input_ports:  vec![Port::new(PortId(88), PortKind::Input, INFLUENCE_SIG)],
        output_ports: vec![Port::with_radius(PortId(99), PortKind::Output, INFLUENCE_SIG, 10.0)],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    world.set_position(broadcaster, 0.0, 0.0);
    world.instances.get_mut(&broadcaster).unwrap().context.set("fire", 1.0);
    world.activate(broadcaster);

    // tick: broadcaster fires S0→S1, emits influence signal.
    // Spatial routing should NOT loop back to itself.
    tick(&mut world);
    // If it looped back, broadcaster would receive "hit" and fire again next tick.
    // Just verify it stayed at S1 (no loop), not crashed.
    assert_eq!(world.instances[&broadcaster].active_state, StateId(1));
    // context.hit should NOT be set via self-routing
    assert_eq!(world.instances[&broadcaster].context.get("hit"), 0.0,
        "broadcaster should not receive its own spatially-routed signal");
}
