/// Phase 5 Lifecycle tests: Appendix C scenario.
///
/// Necromancer summons a Skeleton.
/// Skeleton has an OnDespawn Transition that emits an explosion signal.
/// Skeleton HP reaches 0 → despawn → explosion → nearby enemy takes damage.

use weaven_core::*;

// ── IDs ─────────────────────────────────────────────────────────────────────

const NECRO_SM:  SmId = SmId(1);
const SKEL_SM:   SmId = SmId(2);
const ENEMY_SM:  SmId = SmId(3);

// Necromancer states
const NECRO_IDLE:    StateId = StateId(0);
const NECRO_SUMMONED: StateId = StateId(1);

// Skeleton states
const SKEL_ALIVE: StateId = StateId(10);
const SKEL_DEAD:  StateId = StateId(11);

// Enemy HP states
const ENEMY_ALIVE:     StateId = StateId(20);
const ENEMY_DESTROYED: StateId = StateId(21);

// Ports
const PORT_CMD_IN:        PortId = PortId(0); // Skeleton: receives commands
const PORT_CMD_OUT:       PortId = PortId(1); // Necromancer: emits commands
const PORT_EXPLODE_IN:    PortId = PortId(2); // Enemy: receives explosion
const PORT_EXPLODE_OUT:   PortId = PortId(3); // Skeleton: emits explosion on death

const SIGTYPE_CMD:     SignalTypeId = SignalTypeId(0);
const SIGTYPE_EXPLODE: SignalTypeId = SignalTypeId(1);

fn sig(key: &str, val: f64, stype: SignalTypeId) -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert(key.to_string(), val);
    Signal { signal_type: stype, payload: p }
}

// ── SM definitions ───────────────────────────────────────────────────────────

fn make_necro_sm() -> SmDef {
    SmDef {
        id: NECRO_SM,
        states: [NECRO_IDLE, NECRO_SUMMONED].into_iter().collect(),
        initial_state: NECRO_IDLE,
        transitions: vec![
            // Idle → Summoned: spawn skeleton
            Transition {
                id: TransitionId(10),
                source: NECRO_IDLE, target: NECRO_SUMMONED, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("summon") > 0.0)),
                guard_expr: None,
                effects: vec![],  // spawn issued externally in test
            },
        ],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_CMD_OUT, PortKind::Output, SIGTYPE_CMD)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_skeleton_sm() -> SmDef {
    SmDef {
        id: SKEL_SM,
        states: [SKEL_ALIVE, SKEL_DEAD].into_iter().collect(),
        initial_state: SKEL_ALIVE,
        transitions: vec![
            // Alive → Dead when HP reaches 0
            Transition {
                id: TransitionId(20),
                source: SKEL_ALIVE, target: SKEL_DEAD, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("hp") <= 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_CMD_IN, PortKind::Input, SIGTYPE_CMD),
            Port::new(PORT_EXPLODE_IN, PortKind::Input, SIGTYPE_EXPLODE),
        ],
        output_ports: vec![
            Port::new(PORT_EXPLODE_OUT, PortKind::Output, SIGTYPE_EXPLODE),
        ],
        // OnDespawn: emit explosion {damage: 50}
        on_despawn_transitions: vec![
            Transition {
                id: TransitionId(99),
                source: SKEL_DEAD, target: SKEL_DEAD, priority: 1,
                guard: None,
                guard_expr: None,
                effects: vec![Box::new(|_ctx| {
                    let mut p = std::collections::BTreeMap::new();
                    p.insert("damage".to_string(), 50.0);
                    vec![EffectOutput::Signal(PORT_EXPLODE_OUT, Signal { signal_type: SIGTYPE_EXPLODE, payload: p })]
                })],
            },
        ],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_enemy_sm() -> SmDef {
    SmDef {
        id: ENEMY_SM,
        states: [ENEMY_ALIVE, ENEMY_DESTROYED].into_iter().collect(),
        initial_state: ENEMY_ALIVE,
        transitions: vec![
            // Alive → Destroyed when explosion damage >= 1
            Transition {
                id: TransitionId(30),
                source: ENEMY_ALIVE, target: ENEMY_DESTROYED, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("damage") >= 1.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_EXPLODE_IN, PortKind::Input, SIGTYPE_EXPLODE),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Tick N: Necromancer summons Skeleton. Skeleton enters Active Set on Tick N+1.
#[test]
fn test_spawn_entity_enters_active_set_next_tick() {
    let mut world = World::new();
    world.register_sm(make_necro_sm());
    world.register_sm(make_skeleton_sm());  // pre-registered, not yet active

    // Tick N: Necromancer transitions, requests spawn
    if let Some(i) = world.instances.get_mut(&NECRO_SM) { i.context.set("summon", 1.0); }
    world.activate(NECRO_SM);
    world.request_spawn(vec![SKEL_SM], vec![]);

    tick(&mut world);

    // After Tick N: Skeleton NOT yet in Active Set (§4.5: "next tick")
    assert!(!world.active_set.contains(&SKEL_SM),
        "Skeleton should not be active on the spawn tick");

    // Tick N+1: Skeleton is eligible (manually activate to simulate first-tick entry)
    world.activate(SKEL_SM);
    // Set HP > 0 so the death guard doesn't fire immediately
    if let Some(i) = world.instances.get_mut(&SKEL_SM) { i.context.set("hp", 100.0); }
    tick(&mut world);

    assert_eq!(world.instances[&SKEL_SM].active_state, SKEL_ALIVE,
        "Skeleton starts at initial state");
}

/// Connection Template: Necromancer.CommandOut → Skeleton.CommandIn established at spawn.
#[test]
fn test_spawn_connection_template_established() {
    let mut world = World::new();
    world.register_sm(make_necro_sm());
    world.register_sm(make_skeleton_sm());

    // Spawn with Connection Template: Necro.CMD_OUT → Skel.CMD_IN
    world.request_spawn(vec![SKEL_SM], vec![
        Connection {
            id: ConnectionId(1),
            source_sm: NECRO_SM, source_port: PORT_CMD_OUT,
            target_sm: SKEL_SM,  target_port: PORT_CMD_IN,
            delay_ticks: 0,
            pipeline: vec![],
        },
    ]);
    tick(&mut world); // Phase 5 establishes the connection

    // Verify connection exists
    let conn_exists = world.connections.iter().any(|c|
        c.source_sm == NECRO_SM && c.target_sm == SKEL_SM
    );
    assert!(conn_exists, "Connection Template should be established at spawn");
}

/// Core Appendix C: Skeleton dies → despawn → explosion cascade → enemy destroyed.
#[test]
fn test_skeleton_death_explosion_destroys_enemy() {
    let mut world = World::new();
    world.register_sm(make_necro_sm());
    world.register_sm(make_skeleton_sm());
    world.register_sm(make_enemy_sm());

    // Establish Connection Template: Skeleton.EXPLODE_OUT → Enemy.EXPLODE_IN
    world.connections.push(Connection {
        id: ConnectionId(2),
        source_sm: SKEL_SM,    source_port: PORT_EXPLODE_OUT,
        target_sm: ENEMY_SM,   target_port: PORT_EXPLODE_IN,
        delay_ticks: 0,
        pipeline: vec![],
    });

    // Set skeleton HP to 0 (already dead)
    world.activate(SKEL_SM);
    world.activate(ENEMY_SM);
    if let Some(i) = world.instances.get_mut(&SKEL_SM) { i.context.set("hp", 0.0); }

    // Tick M: Skeleton Alive → Dead (HP = 0)
    tick(&mut world);
    assert_eq!(world.instances[&SKEL_SM].active_state, SKEL_DEAD, "Skeleton→Dead");

    // Despawn the skeleton (issued externally, e.g. by game logic)
    world.request_despawn(vec![SKEL_SM]);

    // Tick M+1: Phase 5 processes despawn.
    //   - OnDespawn fires → emits explosion {damage:50} via EXPLODE_OUT
    //   - Connection routes signal to Enemy.EXPLODE_IN
    //   - Phase 5d cascade: Enemy guard fires → Alive→Destroyed
    tick(&mut world);

    assert_eq!(world.instances[&ENEMY_SM].active_state, ENEMY_DESTROYED,
        "Enemy destroyed by explosion cascade");

    // Skeleton is severed from Active Set and connections
    assert!(!world.active_set.contains(&SKEL_SM), "Skeleton removed from Active Set");
    let skel_conns = world.connections.iter().any(|c|
        c.source_sm == SKEL_SM || c.target_sm == SKEL_SM
    );
    assert!(!skel_conns, "Skeleton connections severed");
}

/// Multiple entities despawning simultaneously: batch delivery is order-independent.
#[test]
fn test_multiple_simultaneous_despawns_batch_delivered() {
    const SKEL_SM_2: SmId = SmId(4);
    const ENEMY_SM_2: SmId = SmId(5);

    let skel2 = SmDef {
        id: SKEL_SM_2,
        states: [SKEL_ALIVE, SKEL_DEAD].into_iter().collect(),
        initial_state: SKEL_ALIVE,
        transitions: vec![Transition {
            id: TransitionId(40),
            source: SKEL_ALIVE, target: SKEL_DEAD, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hp") <= 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_EXPLODE_OUT, PortKind::Output, SIGTYPE_EXPLODE)],
        on_despawn_transitions: vec![Transition {
            id: TransitionId(98),
            source: SKEL_DEAD, target: SKEL_DEAD, priority: 1,
            guard: None,
            guard_expr: None,
            effects: vec![Box::new(|_ctx| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("damage".to_string(), 50.0);
                vec![EffectOutput::Signal(PORT_EXPLODE_OUT, Signal { signal_type: SIGTYPE_EXPLODE, payload: p })]
            })],
        }],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    };

    let enemy2 = SmDef {
        id: ENEMY_SM_2,
        states: [ENEMY_ALIVE, ENEMY_DESTROYED].into_iter().collect(),
        initial_state: ENEMY_ALIVE,
        transitions: vec![Transition {
            id: TransitionId(50),
            source: ENEMY_ALIVE, target: ENEMY_DESTROYED, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("damage") >= 1.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports: vec![Port::new(PORT_EXPLODE_IN, PortKind::Input, SIGTYPE_EXPLODE)],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    };

    let mut world = World::new();
    world.register_sm(make_skeleton_sm());
    world.register_sm(skel2);
    world.register_sm(make_enemy_sm());
    world.register_sm(enemy2);

    world.connections.push(Connection {
        id: ConnectionId(10),
        source_sm: SKEL_SM,  source_port: PORT_EXPLODE_OUT,
        target_sm: ENEMY_SM, target_port: PORT_EXPLODE_IN,
        delay_ticks: 0, pipeline: vec![],
    });
    world.connections.push(Connection {
        id: ConnectionId(11),
        source_sm: SKEL_SM_2,  source_port: PORT_EXPLODE_OUT,
        target_sm: ENEMY_SM_2, target_port: PORT_EXPLODE_IN,
        delay_ticks: 0, pipeline: vec![],
    });

    // Both skeletons die
    for id in [SKEL_SM, SKEL_SM_2, ENEMY_SM, ENEMY_SM_2] { world.activate(id); }
    for id in [SKEL_SM, SKEL_SM_2] {
        if let Some(i) = world.instances.get_mut(&id) { i.context.set("hp", 0.0); }
    }
    tick(&mut world); // both → Dead

    // Despawn both simultaneously
    world.request_despawn(vec![SKEL_SM]);
    world.request_despawn(vec![SKEL_SM_2]);
    tick(&mut world);

    assert_eq!(world.instances[&ENEMY_SM].active_state,   ENEMY_DESTROYED, "enemy1 destroyed");
    assert_eq!(world.instances[&ENEMY_SM_2].active_state, ENEMY_DESTROYED, "enemy2 destroyed");
}

/// Despawned entity's SM is no longer reachable via Active Set.
#[test]
fn test_despawned_sm_removed_from_active_set() {
    let mut world = World::new();
    world.register_sm(make_skeleton_sm());
    world.activate(SKEL_SM);
    if let Some(i) = world.instances.get_mut(&SKEL_SM) { i.context.set("hp", 0.0); }

    tick(&mut world); // → Dead

    world.request_despawn(vec![SKEL_SM]);
    tick(&mut world); // Phase 5 processes despawn

    assert!(!world.active_set.contains(&SKEL_SM), "despawned SM not in Active Set");
}
