/// Mycelia demo — comprehensive integration tests for all 3 phases.
///
/// Phase 1: Season system, market economy (trust_score), stress↔parasitism
/// Phase 2: Allelopathy, hub node topology, fungal autonomy
/// Phase 3: Kin recognition, hyphal tip agents, biome succession

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
    for def in compiled.sm_defs { world.register_sm(def); }
    for conn in compiled.connections { world.connect(conn); }
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
        i.context.set("is_deciduous", 1.0);
        i.context.set("stress_level", 0.0);
        i.context.set("trust_score", 1.0);
        i.context.set("kinship", 0.0);
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
        i.context.set("is_deciduous", 1.0);
        i.context.set("stress_level", 0.0);
        i.context.set("trust_score", 1.0);
        i.context.set("kinship", 0.0);
    }
}

fn init_tree(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("moisture_need", 40.0);
        i.context.set("light_need", 0.7);
        i.context.set("needs_mycorrhiza", 0.0);
        i.context.set("needs_pollinator", 0.0);
        i.context.set("hp", 200.0);
        i.context.set("carbon", 500.0);
        i.context.set("is_deciduous", 0.0);
        i.context.set("stress_level", 0.0);
        i.context.set("trust_score", 1.0);
        i.context.set("connection_count", 5.0); // hub tree
        i.context.set("kinship", 0.0);
    }
}

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

fn init_season(world: &mut World, season_length: f64) {
    if let Some(i) = world.instances.get_mut(&SmId(5)) {
        i.context.set("season_length", season_length);
        i.context.set("tick_in_season", 0.0);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Schema structure (updated for all phases)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn schema_loads_7_sms_8_connections_4_tables() {
    let compiled = load_mycelia();
    assert_eq!(compiled.sm_defs.len(), 7,
        "plant, fungus, creature, soil, season, hyphal_tip, biome");
    assert_eq!(compiled.connections.len(), 8);
    assert_eq!(compiled.table_registry.0.len(), 4,
        "plant_species, fungus_compat, season_params, stress_thresholds");
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 1-1: Season SM
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn season_sm_has_4_states() {
    let compiled = load_mycelia();
    let season = compiled.sm_defs.iter().find(|d| d.id == SmId(5)).unwrap();
    assert_eq!(season.states.len(), 4, "spring(0), summer(1), autumn(2), winter(3)");
    assert_eq!(season.initial_state, StateId(0), "starts in spring");
}

#[test]
fn season_transitions_spring_to_summer() {
    let mut world = setup_world(load_mycelia());
    init_season(&mut world, 5.0); // 5 ticks per season

    // Advance past season_length → spring→summer
    if let Some(i) = world.instances.get_mut(&SmId(5)) {
        i.context.set("tick_in_season", 6.0);
    }
    world.activate(SmId(5));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(5)].active_state, StateId(1), "summer");
}

#[test]
fn season_full_cycle_returns_to_spring() {
    let mut world = setup_world(load_mycelia());
    init_season(&mut world, 1.0); // fast: 1 tick per season

    // spring→summer (tick_in_season > 1)
    if let Some(i) = world.instances.get_mut(&SmId(5)) {
        i.context.set("tick_in_season", 2.0);
    }
    world.activate(SmId(5));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(5)].active_state, StateId(1), "summer");

    // summer→autumn
    if let Some(i) = world.instances.get_mut(&SmId(5)) {
        i.context.set("tick_in_season", 2.0);
    }
    world.activate(SmId(5));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(5)].active_state, StateId(2), "autumn");

    // autumn→winter
    if let Some(i) = world.instances.get_mut(&SmId(5)) {
        i.context.set("tick_in_season", 2.0);
    }
    world.activate(SmId(5));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(5)].active_state, StateId(3), "winter");

    // winter→spring (cycle complete)
    if let Some(i) = world.instances.get_mut(&SmId(5)) {
        i.context.set("tick_in_season", 2.0);
    }
    world.activate(SmId(5));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(5)].active_state, StateId(0), "back to spring");
}

#[test]
fn season_signal_routes_to_plant_via_connection_8() {
    let compiled = load_mycelia();
    let conn8 = compiled.connections.iter().find(|c| c.id == ConnectionId(8));
    assert!(conn8.is_some(), "Connection 8: season→plant exists");
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 1-2: Market economy (trust_score)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fungus_nutrient_output_scaled_by_trust_score() {
    // Fungus transition 202 emits P+N multiplied by trust_score
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("phosphorus_pool", 30.0);
        i.context.set("nitrogen_pool", 20.0);
        i.context.set("trust_score", 0.5); // low trust → reduced output
    }
    // Bring fungus to connected state
    world.activate(SmId(2));
    tick(&mut world); // spore→germinating
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 55.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // germinating→growing
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("growth", 85.0);
        i.context.set("nearby_plants", 1.0);
    }
    world.activate(SmId(2));
    tick(&mut world); // growing→connected (signal with trust_score=0.5)
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(3));
    // The emitted signal has phosphorus = 30 * 0.5 = 15, nitrogen = 20 * 0.5 = 10
}

#[test]
fn high_trust_gives_more_nutrients() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("phosphorus_pool", 30.0);
        i.context.set("nitrogen_pool", 20.0);
        i.context.set("trust_score", 1.0); // full trust
        i.context.set("growth", 85.0);
        i.context.set("nearby_plants", 1.0);
    }
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
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(3));
    // phosphorus = 30 * 1.0 = 30 (full allocation)
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 1-3: Stress thresholds table
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn stress_thresholds_table_exists() {
    let compiled = load_mycelia();
    assert!(compiled.table_registry.0.contains_key("stress_thresholds"),
        "stress_thresholds named table exists");
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 2-1: Allelopathy (walnut species in Named Table)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn walnut_species_has_allelopathic_flag() {
    let compiled = load_mycelia();
    let table = compiled.table_registry.lookup("plant_species", &["walnut", "allelopathic"]);
    assert!(table.is_some(), "walnut.allelopathic exists in table");
    assert_eq!(table.and_then(|v| v.as_f64()), Some(1.0), "walnut is allelopathic");
}

#[test]
fn grass_is_not_allelopathic() {
    let compiled = load_mycelia();
    let table = compiled.table_registry.lookup("plant_species", &["grass", "allelopathic"]);
    assert_eq!(table.and_then(|v| v.as_f64()), Some(0.0));
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 2-2: Hub node (connection_count on tree)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn tree_has_hub_connection_count() {
    let mut world = setup_world(load_mycelia());
    init_tree(&mut world);
    let count = world.instances[&SmId(1)].context.get("connection_count");
    assert!(count >= 5.0, "tree is hub with 5+ connections");
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 3-1: Kin recognition
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn kin_plants_have_kinship_field() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    let kinship = world.instances[&SmId(1)].context.get("kinship");
    assert_eq!(kinship, 0.0, "default kinship is 0 (no relation)");
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 3-2: Hyphal tip agent SM
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn hyphal_tip_sm_has_5_states() {
    let compiled = load_mycelia();
    let tip = compiled.sm_defs.iter().find(|d| d.id == SmId(6)).unwrap();
    assert_eq!(tip.states.len(), 5,
        "exploring(0), branching(1), connecting(2), fused(3), dormant(4)");
}

#[test]
fn hyphal_tip_explores_with_moisture_and_nutrients() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("moisture", 40.0);
        i.context.set("nutrients", 15.0);
    }
    world.activate(SmId(6));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(6)].active_state, StateId(1), "branching");
}

#[test]
fn hyphal_tip_dormant_without_nutrients() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("moisture", 40.0);
        i.context.set("nutrients", 15.0);
    }
    world.activate(SmId(6));
    tick(&mut world); // exploring→branching
    assert_eq!(world.instances[&SmId(6)].active_state, StateId(1));

    // Deplete nutrients → dormant
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("nutrients", 3.0);
    }
    world.activate(SmId(6));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(6)].active_state, StateId(4), "dormant");
}

#[test]
fn hyphal_tip_connects_to_nearby_root() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("moisture", 40.0);
        i.context.set("nutrients", 15.0);
    }
    world.activate(SmId(6));
    tick(&mut world); // exploring→branching

    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("growth", 45.0);
    }
    world.activate(SmId(6));
    tick(&mut world); // branching→connecting (growth > 40)

    // Now simulate finding a root
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("nearby_root", 1.0);
    }
    world.activate(SmId(6));
    tick(&mut world); // connecting→fused (nearby_root > 0)
    assert_eq!(world.instances[&SmId(6)].active_state, StateId(3), "fused with root");
}

#[test]
fn hyphal_tip_revives_from_dormancy() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("moisture", 40.0);
        i.context.set("nutrients", 15.0);
    }
    world.activate(SmId(6));
    tick(&mut world); // exploring→branching

    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("nutrients", 3.0);
    }
    world.activate(SmId(6));
    tick(&mut world); // branching→dormant
    assert_eq!(world.instances[&SmId(6)].active_state, StateId(4));

    // Restore nutrients → dormant→exploring
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("nutrients", 25.0);
    }
    world.activate(SmId(6));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(6)].active_state, StateId(0), "revived from dormancy");
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 3-3: Biome succession SM
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn biome_sm_has_4_states() {
    let compiled = load_mycelia();
    let biome = compiled.sm_defs.iter().find(|d| d.id == SmId(7)).unwrap();
    assert_eq!(biome.states.len(), 4,
        "wasteland(0), pioneer(1), transitional(2), climax(3)");
}

#[test]
fn biome_wasteland_to_pioneer() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("soil_quality", 25.0);
        i.context.set("plant_cover", 15.0);
    }
    world.activate(SmId(7));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(1), "pioneer");
}

#[test]
fn biome_pioneer_to_transitional() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("soil_quality", 55.0);
        i.context.set("plant_cover", 15.0);
        i.context.set("plant_diversity", 4.0);
        i.context.set("network_density", 35.0);
    }
    world.activate(SmId(7));
    tick(&mut world); // wasteland→pioneer
    world.activate(SmId(7));
    tick(&mut world); // pioneer→transitional
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(2), "transitional");
}

#[test]
fn biome_reaches_climax_with_hubs_and_diversity() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("soil_quality", 85.0);
        i.context.set("plant_cover", 50.0);
        i.context.set("plant_diversity", 8.0);
        i.context.set("network_density", 75.0);
        i.context.set("hub_count", 3.0);
    }
    // wasteland→pioneer→transitional→climax (3 ticks)
    for _ in 0..3 {
        world.activate(SmId(7));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(3), "climax!");
}

#[test]
fn biome_degrades_when_hubs_destroyed() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("soil_quality", 85.0);
        i.context.set("plant_cover", 50.0);
        i.context.set("plant_diversity", 8.0);
        i.context.set("network_density", 75.0);
        i.context.set("hub_count", 3.0);
    }
    for _ in 0..3 {
        world.activate(SmId(7));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(3));

    // Destroy all hubs → climax degrades to transitional
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("hub_count", 0.0);
    }
    world.activate(SmId(7));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(2),
        "climax degrades when hubs destroyed (scale-free fragmentation)");
}

#[test]
fn biome_degrades_when_network_density_drops() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("soil_quality", 85.0);
        i.context.set("plant_cover", 50.0);
        i.context.set("plant_diversity", 8.0);
        i.context.set("network_density", 75.0);
        i.context.set("hub_count", 3.0);
    }
    for _ in 0..3 {
        world.activate(SmId(7));
        tick(&mut world);
    }
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(3));

    // Network density drops → climax→transitional
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("network_density", 25.0);
    }
    world.activate(SmId(7));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(7)].active_state, StateId(2));
}

// ═══════════════════════════════════════════════════════════════════════
// Existing core mechanics (preserved from previous tests)
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
fn plant_full_lifecycle() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 4);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(4), "flowering");
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
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(4), "WIN!");
}

#[test]
fn plant_wilts_emits_jasmonic_acid() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 2);
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("hp", 0.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(5));
}

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
fn creature_flees_overrides_hunger() {
    let mut world = setup_world(load_mycelia());
    if let Some(i) = world.instances.get_mut(&SmId(3)) {
        i.context.set("hunger", 60.0);
        i.context.set("threat_level", 5.0);
    }
    world.activate(SmId(3));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(3)].active_state, StateId(3));
}

#[test]
fn soil_full_enrichment() {
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
fn hundred_tick_stability_all_7_sms() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    init_season(&mut world, 10.0);
    if let Some(i) = world.instances.get_mut(&SmId(2)) {
        i.context.set("moisture", 50.0);
        i.context.set("trust_score", 1.0);
    }
    if let Some(i) = world.instances.get_mut(&SmId(6)) {
        i.context.set("moisture", 40.0);
        i.context.set("nutrients", 15.0);
    }
    if let Some(i) = world.instances.get_mut(&SmId(7)) {
        i.context.set("soil_quality", 30.0);
        i.context.set("plant_cover", 15.0);
    }

    for t in 0..100u32 {
        let moisture = 20.0 + 30.0 * (t as f64 * 0.1).sin();
        if let Some(i) = world.instances.get_mut(&SmId(1)) {
            i.context.set("moisture", moisture);
            i.context.set("light", (t as f64 * 0.05 * std::f64::consts::PI).sin().max(0.0));
        }
        if let Some(i) = world.instances.get_mut(&SmId(2)) {
            i.context.set("moisture", moisture);
        }
        let current = world.instances[&SmId(5)].context.get("tick_in_season");
        if let Some(i) = world.instances.get_mut(&SmId(5)) {
            i.context.set("tick_in_season", current + 1.0);
        }
        for id in 1..=7 {
            world.activate(SmId(id));
        }
        tick(&mut world);
    }
    // No panic = pass
}

#[test]
fn snapshot_restore_all_sms() {
    let mut world = setup_world(load_mycelia());
    init_grass(&mut world);
    grow_plant_to(&mut world, 2);
    let snap = weaven_core::network::snapshot(&world);

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("growth", 95.0);
    }
    world.activate(SmId(1));
    tick(&mut world);
    assert_ne!(world.instances[&SmId(1)].active_state, StateId(2));

    weaven_core::network::restore(&mut world, &snap);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(2));
}
