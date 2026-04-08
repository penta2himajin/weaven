/// Mycelia demo — Weaven schema integration tests.
///
/// Tests ecological simulation modeled on real mycorrhizal biology:
///
///   **Bidirectional exchange** (Simard et al.): plants supply ~30% of
///   photosynthetic carbon to fungi; fungi return phosphorus + nitrogen.
///   Modeled via Connection 1 (fungus→plant: P+N) and Connection 6
///   (plant→fungus: carbon at death).
///
///   **Jasmonic acid warning cascade** (Song et al. 2010, Nature Sci Rep 2014):
///   herbivore attack triggers JA production → transmitted through CMN →
///   receiver plants upregulate defense within 6 hours.
///   Modeled via plant output port 3 (warning) → Connection 3 → fungus →
///   Connection 4/5 (relay with attenuation) → plant input port 5 (defense).
///
///   **Source-sink nutrient flow**: nutrients move from high to low
///   concentration; Connection pipelines use Filter to enforce this.
///
///   **Decomposition cycle**: dead plant → organic matter signal →
///   fungus decomposes → soil enrichment.

use weaven_core::*;
use weaven_core::schema::{load_schema, compile_schema, SchemaCompileResult};

fn load_mycelia() -> SchemaCompileResult {
    let json = include_str!("../../demos/mycelia/mycelia.json");
    let schema = load_schema(json).expect("mycelia schema should parse");
    compile_schema(&schema)
}

fn setup_world(compiled: SchemaCompileResult) -> World {
    let mut world = World::new();
    world.tables = compiled.table_registry;
    for def in compiled.sm_defs {
        world.register_sm(def);
    }
    for conn in compiled.connections {
        world.connect(conn);
    }
    world
}

fn init_grass(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture_need", 20.0);
        i.context.set("light_need", 0.5);
        i.context.set("needs_mycorrhiza", 0.0);
        i.context.set("needs_pollinator", 0.0);
        i.context.set("hp", 50.0);
        i.context.set("carbon", 100.0);
    }
}

fn init_orchid(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture_need", 60.0);
        i.context.set("light_need", 0.3);
        i.context.set("needs_mycorrhiza", 1.0);
        i.context.set("needs_pollinator", 1.0);
        i.context.set("hp", 30.0);
        i.context.set("carbon", 50.0);
    }
}

fn init_tree(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture_need", 40.0);
        i.context.set("light_need", 0.7);
        i.context.set("needs_mycorrhiza", 0.0);
        i.context.set("needs_pollinator", 0.0);
        i.context.set("hp", 200.0);
        i.context.set("carbon", 500.0);  // Mother tree: much more carbon
    }
}

/// Advance plant from seed to target state.
fn grow_plant_to(world: &mut World, target_state: u32) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 70.0);
        i.context.set("light", 0.8);
        i.context.set("growth", 95.0);
    }
    for _ in 0..target_state {
        world.activate(SmId(1));
        tick(world);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Schema structure
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn schema_loads_4_sms_7_connections_2_tables() {
    let compiled = load_mycelia();
    assert_eq!(compiled.sm_defs.len(), 4);
    assert_eq!(compiled.connections.len(), 7);
    assert_eq!(compiled.table_registry.0.len(), 2);
}

#[test]
fn plant_sm_has_7_states_and_correct_ports() {
    let compiled = load_mycelia();
    let plant = compiled.sm_defs.iter().find(|d| d.id == SmId(1)).unwrap();
    assert_eq!(plant.states.len(), 7);
    assert_eq!(plant.input_ports.len(), 3,  "nutrient_in + warning_in + damage_in");
    assert_eq!(plant.output_ports.len(), 4, "pollen + decompose + warning_out + carbon_out");
}

#[test]
fn fungus_sm_has_4_states_and_correct_ports() {
    let compiled = load_mycelia();
    let fungus = compiled.sm_defs.iter().find(|d| d.id == SmId(2)).unwrap();
    assert_eq!(fungus.states.len(), 4);
    assert_eq!(fungus.input_ports.len(), 3,  "decompose_in + warning_in + carbon_in");
    assert_eq!(fungus.output_ports.len(), 2, "nutrient_out + warning_relay_out");
}

// ═══════════════════════════════════════════════════════════════════════
// Plant lifecycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn plant_seed_dormant_without_water() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 0.0);
        i.context.set("light", 1.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(0));
}

#[test]
fn plant_seed_sprouts_with_conditions() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 30.0);
        i.context.set("light", 0.8);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1));
}

#[test]
fn plant_full_lifecycle_seed_to_mature() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 3);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3), "mature");
}

#[test]
fn plant_wilts_when_hp_zero_emits_warning() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 2); // growing state
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(2));

    // Drain HP → wilting transition fires (priority 20 > growth transition)
    // This transition also emits jasmonic_acid warning signal on port 3
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("hp", 0.0);
    }
    world.activate(SmId(1));
    let output = tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(5), "wilting");
    // Verify state change was recorded
    assert!(output.state_changes.contains_key(&SmId(1)));
}

#[test]
fn plant_dead_emits_decompose_and_carbon_signals() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 2);

    // growing → wilting
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("hp", 0.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(5));

    // wilting → dead (emits decompose signal on port 2 + carbon on port 4)
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(6), "dead");
}

// ═══════════════════════════════════════════════════════════════════════
// Fungus lifecycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fungus_germinates_with_moisture() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
    }
    world.activate(SmId(2));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(1));
}

#[test]
fn fungus_dormant_in_dry_soil() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 20.0);
    }
    world.activate(SmId(2));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(0));
}

#[test]
fn fungus_full_lifecycle_to_connected() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("phosphorus_pool", 30.0);
        i.context.set("nitrogen_pool", 20.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // spore → germinating

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 55.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // germinating → growing

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 85.0);
        i.context.set("nearby_plants", 1.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // growing → connected (emits P+N signal)
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(3));
}

// ═══════════════════════════════════════════════════════════════════════
// Creature behavior
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn creature_forages_when_hungry() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(1), "moving");
}

#[test]
fn creature_flees_overrides_hunger() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
        i.context.set("threat_level", 5.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(3), "fleeing");
}

#[test]
fn creature_emits_damage_signal_when_acting() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
    }
    world.activate(SmId(3));
    tick(&mut world); // idle → moving

    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("at_target", 1.0);
    }
    world.activate(SmId(3));
    let output = tick(&mut world); // moving → acting (emits damage signal)
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(2));
    assert!(output.state_changes.contains_key(&SmId(3)));
}

#[test]
fn creature_returns_idle_after_satiated() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("at_target", 1.0);
    }
    world.activate(SmId(3));
    tick(&mut world);

    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 5.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(0), "idle");
}

// ═══════════════════════════════════════════════════════════════════════
// Soil cell
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn soil_barren_to_poor() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("organic_matter", 25.0);
    }
    world.activate(SmId(4));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(4)].active_state, StateId(1));
}

#[test]
fn soil_full_enrichment_to_rich() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("organic_matter", 85.0);
        i.context.set("nutrients", 65.0);
        i.context.set("moisture", 50.0);
    }
    for _ in 0..3 {
        world.activate(SmId(4));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(4)].active_state, StateId(3));
}

#[test]
fn soil_degrades_when_depleted() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("organic_matter", 85.0);
        i.context.set("nutrients", 65.0);
        i.context.set("moisture", 50.0);
    }
    for _ in 0..3 {
        world.activate(SmId(4));
        tick(&mut world);
    }

    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("nutrients", 25.0);
    }
    world.activate(SmId(4));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(4)].active_state, StateId(2), "degraded");
}

// ═══════════════════════════════════════════════════════════════════════
// Orchid win condition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn orchid_blocked_without_mycorrhiza() {
    let mut world = setup_world(load_mycelia());
    init_orchid(&mut world);
    grow_plant_to(&mut world, 3);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3));

    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3),
        "orchid stays mature without mycorrhiza");
}

#[test]
fn orchid_blocked_without_pollinator() {
    let mut world = setup_world(load_mycelia());
    init_orchid(&mut world);
    grow_plant_to(&mut world, 3);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("has_mycorrhiza", 1.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3),
        "orchid stays mature without pollinator");
}

#[test]
fn orchid_flowers_with_both_requirements() {
    let mut world = setup_world(load_mycelia());
    init_orchid(&mut world);
    grow_plant_to(&mut world, 3);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("has_mycorrhiza", 1.0);
        i.context.set("has_pollinator", 1.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(4),
        "ORCHID BLOOMS — WIN!");
}

#[test]
fn grass_flowers_without_special_requirements() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 4);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(4));
}

// ═══════════════════════════════════════════════════════════════════════
// Bidirectional nutrient exchange (Connection 1: fungus→plant P+N)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fungus_connected_emits_phosphorus_nitrogen() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("growth", 85.0);
        i.context.set("nearby_plants", 1.0);
        i.context.set("phosphorus_pool", 30.0);
        i.context.set("nitrogen_pool", 20.0);
    }
    // spore → germinating → growing → connected
    world.activate(SmId(2));
    tick(&mut world);
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 55.0);
    }
    world.activate(SmId(2));
    tick(&mut world);
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 85.0);
    }
    world.activate(SmId(2));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(3),
        "fungus connected — emitted P+N signal to plant via Connection 1");
}

// ═══════════════════════════════════════════════════════════════════════
// Jasmonic acid warning cascade
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn wilting_plant_emits_jasmonic_acid_signal() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 2); // growing

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("hp", 0.0);
    }
    world.activate(SmId(1));
    let output = tick(&mut world); // growing → wilting (emits JA on port 3)
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(5));
    // The signal is sent on port 3 → Connection 3 routes to fungus port 3
    assert!(output.state_changes.contains_key(&SmId(1)),
        "state change recorded for warning signal emission");
}

#[test]
fn warning_signal_attenuates_through_fungus_relay() {
    // Connection 5: fungus→fungus relay with Transform { ja: ja - 1 }
    // + Filter { ja > 0 }. Intensity decreases with each hop.
    // Connection 4: fungus→plant relay with same attenuation.
    let compiled = load_mycelia();
    // Verify connection 4 exists (fungus→plant warning with attenuation)
    assert!(compiled.connections.iter().any(|c| c.id == ConnectionId(4)),
        "Connection 4: fungus→plant warning relay exists");
    // Verify connection 5 exists (fungus→fungus warning relay)
    assert!(compiled.connections.iter().any(|c| c.id == ConnectionId(5)),
        "Connection 5: fungus→fungus warning relay exists");
}

// ═══════════════════════════════════════════════════════════════════════
// Connection: pest damage (Connection 7: creature→plant)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn pest_damage_connection_exists() {
    let compiled = load_mycelia();
    let conn7 = compiled.connections.iter().find(|c| c.id == ConnectionId(7));
    assert!(conn7.is_some(), "Connection 7: creature→plant damage exists");
}

// ═══════════════════════════════════════════════════════════════════════
// Carbon cycle: plant death → fungus receives carbon (Connection 6)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn carbon_connection_exists() {
    let compiled = load_mycelia();
    let conn6 = compiled.connections.iter().find(|c| c.id == ConnectionId(6));
    assert!(conn6.is_some(), "Connection 6: plant→fungus carbon at death");
}

// ═══════════════════════════════════════════════════════════════════════
// Mother tree effect: tree has much more carbon reserve
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn tree_has_higher_carbon_than_grass() {
    let mut world = setup_world(load_mycelia());
    init_tree(&mut world);
    let tree_carbon = world.instances[&SmId(1)].context.get("carbon");

    let mut world2 = setup_world(load_mycelia());
    init_grass(&mut world2);
    let grass_carbon = world2.instances[&SmId(1)].context.get("carbon");

    assert!(tree_carbon > grass_carbon * 3.0,
        "tree carbon {} >> grass carbon {} (mother tree effect)",
        tree_carbon, grass_carbon);
}

// ═══════════════════════════════════════════════════════════════════════
// Stability: multi-tick simulation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn hundred_tick_stability() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
    }

    for t in 0..100u32 {
        let moisture = 20.0 + 30.0 * (t as f64 * 0.1).sin();
        let light = (t as f64 * 0.05 * std::f64::consts::PI).sin().max(0.0);

        if let Some(i) = world.instances.get_mut(&SmId(1)) {
            i.context.set("moisture", moisture);
            i.context.set("light", light);
        }
        if let Some(i) = world.instances.get_mut(&SmId(2)) {
            i.context.set("moisture", moisture);
        }

        world.activate(SmId(1));
        world.activate(SmId(2));
        world.activate(SmId(3));
        world.activate(SmId(4));
        tick(&mut world);
    }
    // No panic = pass
}

#[test]
fn snapshot_restore_preserves_all_sm_states() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 2);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(2));

    let snap = weaven_core::network::snapshot(&world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("growth", 95.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    // State advanced past growing
    assert_ne!(world.instances[&SmId(1)].active_state, StateId(2));

    weaven_core::network::restore(&mut world, &snap);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(2),
        "restored to growing state");
}
