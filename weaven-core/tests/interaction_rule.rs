/// Integration tests for InteractionRule (§2.7).
///
/// Validates:
///   1. Parry (Appendix B) — rewritten to use register_rule instead of manual injection
///   2. Elemental reaction — fire + water proximity → extinguish
///   3. Phase 2 invariant: IR evaluates PRE-transition states

use weaven_core::*;

// ── shared ────────────────────────────────────────────────────────────────

fn combat_signal_with(key: &str, val: f64) -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert(key.to_string(), val);
    Signal { signal_type: SignalTypeId(0), payload: p }
}

fn fire_signal(intensity: f64) -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert("intensity".to_string(), intensity);
    Signal { signal_type: SignalTypeId(1), payload: p }
}

// ── Appendix B: Parry via InteractionRule ────────────────────────────────

const ENEMY_WINDUP:      StateId = StateId(0);
const ENEMY_ACTIVEFRAME: StateId = StateId(1);
const ENEMY_STAGGERED:   StateId = StateId(2);
const PC_IDLE:   StateId = StateId(10);
const PC_PARRY:  StateId = StateId(11);
const PC_RIPOSTE: StateId = StateId(12);

const PORT_STAGGER_IN:  PortId = PortId(0);
const PORT_PARRY_OK_IN: PortId = PortId(1);

fn make_enemy(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [ENEMY_WINDUP, ENEMY_ACTIVEFRAME, ENEMY_STAGGERED].into_iter().collect(),
        initial_state: ENEMY_WINDUP,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 100),
                source: ENEMY_WINDUP, target: ENEMY_ACTIVEFRAME,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("timer_expired") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            Transition {
                id: TransitionId(id.0 * 100 + 1),
                source: ENEMY_ACTIVEFRAME, target: ENEMY_STAGGERED,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("stagger") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![Port::new(PORT_STAGGER_IN, PortKind::Input, SignalTypeId(0))],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_pc(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [PC_IDLE, PC_PARRY, PC_RIPOSTE].into_iter().collect(),
        initial_state: PC_IDLE,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 100),
                source: PC_IDLE, target: PC_PARRY,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry_input") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            Transition {
                id: TransitionId(id.0 * 100 + 1),
                source: PC_PARRY, target: PC_RIPOSTE,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry_success") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![Port::new(PORT_PARRY_OK_IN, PortKind::Input, SignalTypeId(0))],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Build the Parry interaction rule.
/// Match: enemy=ActiveFrame AND pc=Parry → stagger enemy, parry_success to pc.
fn parry_rule(enemy_id: SmId, pc_id: SmId) -> InteractionRuleDef {
    InteractionRuleDef {
        id: 1,
        group: "combat",
        watch: IrWatch::All,
        spatial_condition: None,
        match_fn: Box::new(move |instances| {
            let enemy_state = instances.get(&enemy_id).map(|i| i.active_state);
            let pc_state    = instances.get(&pc_id).map(|i| i.active_state);
            if enemy_state == Some(ENEMY_ACTIVEFRAME) && pc_state == Some(PC_PARRY) {
                vec![
                    IrSignal {
                        source_sm: None,
                        target_sm:   enemy_id,
                        target_port: PORT_STAGGER_IN,
                        signal:      combat_signal_with("stagger", 1.0),
                    },
                    IrSignal {
                        source_sm: None,
                        target_sm:   pc_id,
                        target_port: PORT_PARRY_OK_IN,
                        signal:      combat_signal_with("parry_success", 1.0),
                    },
                ]
            } else {
                vec![]
            }
        }),
    }
}

/// Core Appendix B: 1-tick delay is structural.
/// Tick N: both SM transitions fire simultaneously.
///         IR sees PRE-transition states (WindUp + Idle) → no match.
/// Tick N+1: IR sees ActiveFrame + Parry → match → stagger + riposte.
#[test]
fn test_parry_via_interaction_rule() {
    let enemy_id = SmId(1);
    let pc_id    = SmId(2);

    let mut world = World::new();
    world.register_sm(make_enemy(enemy_id));
    world.register_sm(make_pc(pc_id));
    world.register_rule(parry_rule(enemy_id, pc_id));

    // Tick N — both trigger simultaneously
    if let Some(i) = world.instances.get_mut(&enemy_id) { i.context.set("timer_expired", 1.0); }
    if let Some(i) = world.instances.get_mut(&pc_id)    { i.context.set("parry_input",   1.0); }
    world.activate(enemy_id);
    world.activate(pc_id);

    let out_n = tick(&mut world);

    // After Tick N: transitioned, but NOT yet staggered/riposte
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME, "Tick N: enemy→ActiveFrame");
    assert_eq!(world.instances[&pc_id].active_state,    PC_PARRY,          "Tick N: pc→Parry");
    assert!(out_n.state_changes.contains_key(&enemy_id), "enemy change reported");
    assert!(out_n.state_changes.contains_key(&pc_id),    "pc change reported");

    // Tick N+1 — IR fires during Phase 2, signals delivered Phase 3/4
    let out_n1 = tick(&mut world);

    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_STAGGERED, "Tick N+1: enemy→Staggered");
    assert_eq!(world.instances[&pc_id].active_state,    PC_RIPOSTE,      "Tick N+1: pc→Riposte");
    assert!(out_n1.state_changes.contains_key(&enemy_id));
    assert!(out_n1.state_changes.contains_key(&pc_id));
}

/// IR does NOT fire when only one participant is in the correct state.
#[test]
fn test_parry_rule_partial_match_no_fire() {
    let enemy_id = SmId(1);
    let pc_id    = SmId(2);

    let mut world = World::new();
    world.register_sm(make_enemy(enemy_id));
    world.register_sm(make_pc(pc_id));
    world.register_rule(parry_rule(enemy_id, pc_id));

    // Only enemy advances
    if let Some(i) = world.instances.get_mut(&enemy_id) { i.context.set("timer_expired", 1.0); }
    world.activate(enemy_id);
    tick(&mut world);

    // Tick 2: IR sees ActiveFrame + Idle → no match
    tick(&mut world);
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME, "enemy stays ActiveFrame");
    assert_eq!(world.instances[&pc_id].active_state,    PC_IDLE,           "pc stays Idle");
}

/// Multiple rules in the same group all fire when their conditions are met.
#[test]
fn test_multiple_rules_same_tick() {
    let enemy_id = SmId(1);
    let pc_id    = SmId(2);

    let mut world = World::new();
    world.register_sm(make_enemy(enemy_id));
    world.register_sm(make_pc(pc_id));

    // Rule A: parry
    world.register_rule(parry_rule(enemy_id, pc_id));

    // Rule B: same group, fires whenever enemy is ActiveFrame (e.g. "send taunt to PC")
    // We reuse PORT_PARRY_OK_IN as a secondary trigger for simplicity.
    world.register_rule(InteractionRuleDef {
        id: 2,
        group: "combat",
        watch: IrWatch::All,
        spatial_condition: None,
        match_fn: Box::new(move |instances| {
            if instances.get(&enemy_id).map(|i| i.active_state) == Some(ENEMY_ACTIVEFRAME) {
                vec![IrSignal {
                    source_sm: None,
                    target_sm:   pc_id,
                    target_port: PORT_PARRY_OK_IN,
                    // "taunt" = 2.0 to distinguish from parry_success = 1.0
                    signal:      combat_signal_with("taunt", 2.0),
                }]
            } else { vec![] }
        }),
    });

    // Set up state: enemy=ActiveFrame, pc=Parry (pre-set, skip transition tick)
    if let Some(i) = world.instances.get_mut(&enemy_id) {
        i.active_state = ENEMY_ACTIVEFRAME;
    }
    if let Some(i) = world.instances.get_mut(&pc_id) {
        i.active_state = PC_PARRY;
    }
    world.activate(enemy_id);
    world.activate(pc_id);

    tick(&mut world);

    // Both rules fired. Parry rule delivers stagger(1) and parry_success(1).
    // Rule B delivers taunt(2) to PORT_PARRY_OK_IN (same port, higher priority overwrites).
    // The parry_success guard wins (fires PC_PARRY→PC_RIPOSTE).
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_STAGGERED, "staggered by rule A");
    assert_eq!(world.instances[&pc_id].active_state,    PC_RIPOSTE,      "riposte by rule A");
}

// ── Elemental reaction: fire + water → extinguish ────────────────────────

const TILE_GRASS:    StateId = StateId(0);
const TILE_BURNING:  StateId = StateId(1);
const TILE_WET:      StateId = StateId(2);
const PC_BURNING:    StateId = StateId(10);
const PC_WET:        StateId = StateId(11);

const PORT_EXTINGUISH_IN: PortId = PortId(5);

fn make_tile_elemental(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [TILE_GRASS, TILE_BURNING, TILE_WET].into_iter().collect(),
        initial_state: TILE_GRASS,
        transitions: vec![
            // Grass → Burning on fire
            Transition {
                id: TransitionId(id.0 * 100),
                source: TILE_GRASS, target: TILE_BURNING, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("intensity") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            // Burning → Wet on extinguish
            Transition {
                id: TransitionId(id.0 * 100 + 1),
                source: TILE_BURNING, target: TILE_WET, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("extinguish") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_EXTINGUISH_IN, PortKind::Input, SignalTypeId(2)),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_pc_elemental(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [PC_BURNING, PC_WET].into_iter().collect(),
        initial_state: PC_BURNING,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 100),
                source: PC_BURNING, target: PC_WET, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("extinguish") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_EXTINGUISH_IN, PortKind::Input, SignalTypeId(2)),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Elemental reaction rule: if Weather=Raining AND any tile/entity is Burning → extinguish.
/// This mirrors the Appendix A Tick N+1 scenario.
#[test]
fn test_elemental_reaction_rain_extinguishes_burning() {
    let tile_id = SmId(1);
    let pc_id   = SmId(2);
    let weather_id = SmId(3);

    const WEATHER_CLEAR:   StateId = StateId(20);
    const WEATHER_RAINING: StateId = StateId(21);

    let mut world = World::new();

    // Tile starts Burning (simulating result of previous fire propagation tick)
    let mut tile = make_tile_elemental(tile_id);
    world.register_sm(tile);
    world.instances.get_mut(&tile_id).unwrap().active_state = TILE_BURNING;

    // PC starts Burning
    world.register_sm(make_pc_elemental(pc_id));
    world.instances.get_mut(&pc_id).unwrap().active_state = PC_BURNING;

    // Weather SM: Clear → Raining
    world.register_sm(SmDef {
        id: weather_id,
        states: [WEATHER_CLEAR, WEATHER_RAINING].into_iter().collect(),
        initial_state: WEATHER_CLEAR,
        transitions: vec![Transition {
            id: TransitionId(300),
            source: WEATHER_CLEAR, target: WEATHER_RAINING, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("rain_start") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports: vec![], output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    // Interaction Rule: Raining AND entity=Burning → extinguish signal
    let extinguish_signal = || {
        let mut p = std::collections::BTreeMap::new();
        p.insert("extinguish".to_string(), 1.0);
        Signal { signal_type: SignalTypeId(2), payload: p }
    };

    world.register_rule(InteractionRuleDef {
        id: 10,
        group: "elemental_reactions",
        watch: IrWatch::All,
        spatial_condition: None,
        match_fn: Box::new(move |instances| {
            let is_raining = instances.get(&weather_id)
                .map(|i| i.active_state == WEATHER_RAINING)
                .unwrap_or(false);
            if !is_raining { return vec![]; }

            let mut signals = vec![];
            if instances.get(&tile_id).map(|i| i.active_state) == Some(TILE_BURNING) {
                signals.push(IrSignal {
                    source_sm: None,
                    target_sm: tile_id, target_port: PORT_EXTINGUISH_IN,
                    signal: extinguish_signal(),
                });
            }
            if instances.get(&pc_id).map(|i| i.active_state) == Some(PC_BURNING) {
                signals.push(IrSignal {
                    source_sm: None,
                    target_sm: pc_id, target_port: PORT_EXTINGUISH_IN,
                    signal: extinguish_signal(),
                });
            }
            signals
        }),
    });

    // Tick N: weather transitions Clear→Raining.
    // IR sees Weather=Clear (pre-transition) → no match.
    if let Some(i) = world.instances.get_mut(&weather_id) { i.context.set("rain_start", 1.0); }
    world.activate(weather_id);
    world.activate(tile_id);
    world.activate(pc_id);

    tick(&mut world);

    assert_eq!(world.instances[&weather_id].active_state, WEATHER_RAINING, "Tick N: weather→Raining");
    // Tile and PC still Burning — IR didn't fire (saw Clear in Phase 2)
    assert_eq!(world.instances[&tile_id].active_state, TILE_BURNING, "Tick N: tile still Burning");
    assert_eq!(world.instances[&pc_id].active_state,   PC_BURNING,   "Tick N: pc still Burning");

    // Tick N+1: IR sees Raining + Burning → extinguish both
    tick(&mut world);

    assert_eq!(world.instances[&tile_id].active_state, TILE_WET, "Tick N+1: tile→Wet");
    assert_eq!(world.instances[&pc_id].active_state,   PC_WET,   "Tick N+1: pc→Wet");
}
