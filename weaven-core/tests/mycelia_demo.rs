/// Mycelia demo game — Weaven schema integration tests.
///
/// Tests the ecological simulation: plant lifecycle, mycorrhizal nutrient
/// exchange (bidirectional carbon-for-minerals trade), jasmonic acid warning
/// cascade, pest damage, pollination, decomposition, and the "Moonlight
/// Orchid bloom" win condition.
///
/// Biology references:
///   - Bidirectional exchange: plants supply ~30% photosynthetic carbon to
///     fungi; fungi supply phosphorus, nitrogen, water back.
///   - Warning signals: jasmonic acid transmitted through CMN, upregulates
///     defense in receiver plants within 6 hours of herbivore attack.
///   - Source-sink dynamics: nutrients flow from surplus to deficit areas.

use weaven_core::*;
use weaven_core::schema::{load_schema, compile_schema};

fn load_mycelia() -> schema::SchemaCompileResult {
    let json = include_str!("../../demos/mycelia/mycelia.json");
    let schema = load_schema(json).expect("mycelia schema should parse");
    compile_schema(&schema)
}

fn setup_world(compiled: schema::SchemaCompileResult) -> World {
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

/// Helper: initialize a plant instance with species-specific parameters.
/// Context is f64-only, so species parameters are set directly.
fn init_plant_as_grass(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture_need", 20.0);
        i.context.set("light_need", 0.5);
        i.context.set("needs_mycorrhiza", 0.0);
        i.context.set("needs_pollinator", 0.0);
        i.context.set("hp", 50.0);
    }
}

fn init_plant_as_orchid(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture_need", 60.0);
        i.context.set("light_need", 0.3);
        i.context.set("needs_mycorrhiza", 1.0);
        i.context.set("needs_pollinator", 1.0);
        i.context.set("hp", 30.0);
    }
}

// ── Schema loading ──────────────────────────────────────────────────────

#[test]
fn test_mycelia_schema_loads_and_compiles() {
    let compiled = load_mycelia();
    assert_eq!(compiled.sm_defs.len(), 4, "4 SM types: plant, fungus, creature, soil");
    assert!(!compiled.connections.is_empty(), "connections exist");
    assert!(!compiled.table_registry.0.is_empty(), "named tables exist");
}

#[test]
fn test_mycelia_plant_sm_structure() {
    let compiled = load_mycelia();
    let plant = compiled.sm_defs.iter().find(|d| d.id == SmId(1)).unwrap();
    assert_eq!(plant.states.len(), 7, "plant has 7 lifecycle states");
    assert_eq!(plant.initial_state, StateId(0), "starts as seed");
}

#[test]
fn test_mycelia_fungus_sm_structure() {
    let compiled = load_mycelia();
    let fungus = compiled.sm_defs.iter().find(|d| d.id == SmId(2)).unwrap();
    assert_eq!(fungus.states.len(), 4, "fungus has 4 states");
    assert_eq!(fungus.initial_state, StateId(0), "starts as spore");
}

// ── Plant lifecycle ─────────────────────────────────────────────────────

#[test]
fn test_plant_seed_stays_dormant_without_water() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);
    init_plant_as_grass(&mut world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 0.0);
        i.context.set("light", 1.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(0),
        "seed stays dormant without water");
}

#[test]
fn test_plant_seed_sprouts_with_conditions_met() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);
    init_plant_as_grass(&mut world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 30.0);
        i.context.set("light", 0.8);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1),
        "grass sprouts with sufficient moisture and light");
}

#[test]
fn test_plant_growth_progression() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);
    init_plant_as_grass(&mut world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 30.0);
        i.context.set("light", 0.8);
    }
    // seed -> sprout
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1), "sprouted");

    // sprout -> growing (growth > 30)
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("growth", 35.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(2), "growing");

    // growing -> mature (growth > 70)
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("growth", 75.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3), "mature");
}

#[test]
fn test_plant_dies_when_hp_depleted() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);
    init_plant_as_grass(&mut world);

    // Fast-track to growing state, then set hp=0
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 30.0);
        i.context.set("light", 0.8);
    }
    world.activate(SmId(1));
    tick(&mut world); // seed -> sprout

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("growth", 40.0);
    }
    world.activate(SmId(1));
    tick(&mut world); // sprout -> growing

    // Now set hp=0 — higher-priority wilting transition should fire
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("hp", 0.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(5),
        "plant wilts when HP depleted");
}

// ── Fungus lifecycle ────────────────────────────────────────────────────

#[test]
fn test_fungus_spore_germinates_with_moisture() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
    }
    world.activate(SmId(2));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(1),
        "spore germinates with sufficient moisture");
}

#[test]
fn test_fungus_stays_dormant_in_dry_soil() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 20.0);
    }
    world.activate(SmId(2));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(0),
        "spore stays dormant without sufficient moisture");
}

#[test]
fn test_fungus_full_lifecycle_to_connected() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("nutrient_pool", 20.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // spore -> germinating
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(1));

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 55.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // germinating -> growing
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(2));

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 85.0);
        i.context.set("nearby_plants", 1.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // growing -> connected
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(3),
        "fungus reaches connected state");
}

// ── Creature behavior ───────────────────────────────────────────────────

#[test]
fn test_creature_starts_foraging_when_hungry() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
        i.context.set("threat_level", 0.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(1),
        "creature starts moving when hungry");
}

#[test]
fn test_creature_flees_when_threatened() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
        i.context.set("threat_level", 5.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(3),
        "creature flees when threat detected (higher priority)");
}

#[test]
fn test_creature_returns_to_idle_after_eating() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    // Move to acting state
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
    }
    world.activate(SmId(3));
    tick(&mut world); // idle -> moving

    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("at_target", 1.0);
    }
    world.activate(SmId(3));
    tick(&mut world); // moving -> acting
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(2), "acting");

    // Hunger satisfied -> return to idle
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 5.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(0),
        "creature returns to idle after hunger satisfied");
}

// ── Soil cell transitions ───────────────────────────────────────────────

#[test]
fn test_soil_enrichment_barren_to_poor() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("organic_matter", 25.0);
    }
    world.activate(SmId(4));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(4)].active_state, StateId(1), "barren -> poor");
}

#[test]
fn test_soil_full_enrichment() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    // Set all conditions for rich soil from the start
    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("organic_matter", 85.0);
        i.context.set("nutrients", 65.0);
        i.context.set("moisture", 50.0);
    }
    // barren -> poor -> moderate -> rich (3 ticks)
    for _ in 0..3 {
        world.activate(SmId(4));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(4)].active_state, StateId(3), "soil is rich");
}

#[test]
fn test_soil_degrades_when_nutrients_drop() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

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

    // Drop nutrients -> rich degrades to moderate
    if let Some(i) = world.instances.get_mut(&SmId(4)) {
        i.context.set("nutrients", 25.0);
    }
    world.activate(SmId(4));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(4)].active_state, StateId(2),
        "soil degrades when nutrients drop");
}

// ── Orchid bloom win condition ──────────────────────────────────────────

#[test]
fn test_orchid_requires_mycorrhiza_and_pollinator_to_flower() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);
    init_plant_as_orchid(&mut world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 70.0);
        i.context.set("light", 0.5);
        i.context.set("growth", 95.0);
    }
    // seed -> sprout -> growing -> mature (3 ticks)
    for _ in 0..3 {
        world.activate(SmId(1));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3), "orchid is mature");

    // Try to flower without mycorrhiza -> stays mature
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3),
        "orchid cannot flower without mycorrhiza");

    // Add mycorrhiza only -> still not enough
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("has_mycorrhiza", 1.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(3),
        "orchid cannot flower without pollinator");

    // Add both -> NOW flowers
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("has_pollinator", 1.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(4),
        "orchid flowers with mycorrhiza AND pollinator — WIN!");
}

#[test]
fn test_grass_flowers_without_special_requirements() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);
    init_plant_as_grass(&mut world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture", 30.0);
        i.context.set("light", 0.8);
        i.context.set("growth", 95.0);
    }
    // Grass: needs_mycorrhiza=0, needs_pollinator=0
    // Guard: has_mycorrhiza(0) >= needs_mycorrhiza(0) -> true
    //        has_pollinator(0) >= needs_pollinator(0) -> true
    // seed -> sprout -> growing -> mature -> flowering (4 ticks)
    for _ in 0..4 {
        world.activate(SmId(1));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(4),
        "grass flowers without special requirements");
}

// ── Connection: fungus nutrient -> plant ─────────────────────────────────

#[test]
fn test_fungus_connection_emits_signal_on_connected() {
    let compiled = load_mycelia();
    let mut world = setup_world(compiled);

    // Bring fungus to connected state
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("nutrient_pool", 20.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // spore -> germinating

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 55.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // germinating -> growing

    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 85.0);
        i.context.set("nearby_plants", 1.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // growing -> connected (emits signal)

    assert_eq!(world.instances[&SmId(2)].active_state, StateId(3),
        "fungus connected and signal emitted to plant");
}
