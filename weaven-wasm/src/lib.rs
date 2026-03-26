/// Weaven WASM Adapter — Browser integration (§12.5, Phase 4).
///
/// Provides JavaScript/TypeScript bindings for Weaven Core via wasm-bindgen.
///
/// TypeScript usage pattern:
/// ```typescript
/// import init, { WeavenSession } from "@weaven/wasm";
///
/// await init();
/// const session = new WeavenSession();
/// session.load_schema(jsonString);
///
/// function gameLoop() {
///     session.tick();
///     const state = session.read_state(smId);
///     render(state);
///     requestAnimationFrame(gameLoop);
/// }
/// requestAnimationFrame(gameLoop);
/// ```

use wasm_bindgen::prelude::*;
use weaven_core::{
    World, SmId, StateId, PortId, Signal, SignalTypeId,
    schema::{load_schema, compile_schema},
    snapshot, restore, WorldSnapshot,
};
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

// ---------------------------------------------------------------------------
// JS error helper
// ---------------------------------------------------------------------------

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

// ---------------------------------------------------------------------------
// WeavenSession — main entry point exposed to JavaScript
// ---------------------------------------------------------------------------

/// A Weaven simulation session. Create one per game/canvas context.
#[wasm_bindgen]
pub struct WeavenSession {
    world: World,
}

#[wasm_bindgen]
impl WeavenSession {
    /// Create a new empty session.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WeavenSession {
        WeavenSession { world: World::new() }
    }

    /// Load SM definitions from a JSON schema string.
    ///
    /// Replaces any previously loaded definitions. Call once at startup.
    /// Throws a JS error string on parse failure.
    pub fn load_schema(&mut self, json: &str) -> Result<(), JsValue> {
        let schema  = load_schema(json).map_err(js_err)?;
        let compiled = compile_schema(&schema);
        for def in compiled.sm_defs {
            self.world.register_sm(def);
        }
        for conn in compiled.connections {
            self.world.connect(conn);
        }
        self.world.tables = compiled.table_registry;
        Ok(())
    }

    /// Enable the spatial index with the given cell size.
    pub fn enable_spatial(&mut self, cell_size: f64) {
        self.world.enable_spatial(cell_size);
    }

    /// Advance the simulation by one tick.
    ///
    /// Returns a JSON string describing state changes:
    /// `[{ "sm_id": N, "prev": M, "next": K }, ...]`
    pub fn tick(&mut self) -> String {
        let output = weaven_core::tick(&mut self.world);
        let changes: Vec<StateChangeSer> = output.state_changes
            .iter()
            .map(|(&id, &(prev, next))| StateChangeSer {
                sm_id: id.0,
                prev:  prev.0,
                next:  next.0,
            })
            .collect();
        serde_json::to_string(&changes).unwrap_or_default()
    }

    /// Push a continuous input value into an SM's context field.
    ///
    /// Corresponds to §2.4.3 Continuous Input Port.
    pub fn push_input(&mut self, sm_id: u32, field: &str, value: f64) {
        if let Some(inst) = self.world.instances.get_mut(&SmId(sm_id)) {
            inst.context.set(field, value);
        }
    }

    /// Read a context field value from an SM's Continuous Output Port (§2.4.4).
    pub fn read_output(&self, sm_id: u32, field: &str) -> f64 {
        self.world.instances
            .get(&SmId(sm_id))
            .map(|i| i.context.get(field))
            .unwrap_or(0.0)
    }

    /// Get the active state ID for an SM.
    /// Returns -1 if the SM does not exist.
    pub fn active_state(&self, sm_id: u32) -> i32 {
        self.world.instances
            .get(&SmId(sm_id))
            .map(|i| i.active_state.0 as i32)
            .unwrap_or(-1)
    }

    /// Inject a discrete signal into an SM's Input Port.
    ///
    /// `payload_json` is a JSON object string of `{ "field": value }` pairs.
    pub fn inject_signal(
        &mut self,
        sm_id: u32,
        port_id: u32,
        signal_type: u32,
        payload_json: &str,
    ) -> Result<(), JsValue> {
        let payload: BTreeMap<String, f64> =
            serde_json::from_str(payload_json).map_err(js_err)?;
        self.world.inject_signal(
            SmId(sm_id),
            PortId(port_id),
            Signal { signal_type: SignalTypeId(signal_type), payload },
        );
        Ok(())
    }

    /// Mark an SM as active (will be evaluated on the next tick).
    pub fn activate(&mut self, sm_id: u32) {
        self.world.activate(SmId(sm_id));
    }

    /// Update the spatial position of an SM (Tier 2 — requires enable_spatial).
    pub fn set_position(&mut self, sm_id: u32, x: f64, y: f64) {
        self.world.set_position(SmId(sm_id), x, y);
    }

    /// Take a JSON snapshot of the current world state (for rollback).
    pub fn snapshot_json(&self) -> String {
        let snap = snapshot(&self.world);
        String::from_utf8(snap.to_json()).unwrap_or_default()
    }

    /// Restore world state from a JSON snapshot.
    pub fn restore_json(&mut self, json: &str) -> Result<(), JsValue> {
        let snap = WorldSnapshot::from_json(json.as_bytes()).map_err(js_err)?;
        restore(&mut self.world, &snap);
        Ok(())
    }

    /// Return the current tick number.
    pub fn current_tick(&self) -> u64 {
        self.world.tick
    }

    /// Return a JSON array of all registered SM IDs.
    pub fn sm_ids_json(&self) -> String {
        let ids: Vec<u32> = self.world.defs.keys().map(|id| id.0).collect();
        serde_json::to_string(&ids).unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Serialisation helpers (not exposed to JS directly)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct StateChangeSer {
    sm_id: u32,
    prev:  u32,
    next:  u32,
}

// ---------------------------------------------------------------------------
// Non-WASM tests (run with `cargo test -p weaven-wasm`)
// ---------------------------------------------------------------------------

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn simple_schema() -> &'static str {
        r#"{
            "state_machines": [{
                "id": 1,
                "states": [0, 1],
                "initial_state": 0,
                "transitions": [{
                    "id": 10, "source": 0, "target": 1, "priority": 10,
                    "guard": { "BinOp": { "op": "Gt",
                        "left":  { "CtxField": "trigger" },
                        "right": { "Num": 0.0 }
                    }},
                    "effects": []
                }],
                "input_ports": [], "output_ports": [],
                "elapse_capability": "NonElapsable"
            }],
            "connections": [],
            "named_tables": []
        }"#
    }

    #[test]
    fn test_session_load_schema() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();
        assert_eq!(session.sm_ids_json(), "[1]");
    }

    #[test]
    fn test_session_tick_transitions() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();
        session.push_input(1, "trigger", 1.0);
        session.activate(1);
        let changes_json = session.tick();
        let changes: Vec<serde_json::Value> =
            serde_json::from_str(&changes_json).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0]["sm_id"], 1);
        assert_eq!(changes[0]["prev"], 0);
        assert_eq!(changes[0]["next"], 1);
    }

    #[test]
    fn test_session_active_state() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();
        assert_eq!(session.active_state(1), 0);
        session.push_input(1, "trigger", 1.0);
        session.activate(1);
        session.tick();
        assert_eq!(session.active_state(1), 1);
    }

    #[test]
    fn test_session_snapshot_restore() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();

        let snap = session.snapshot_json();
        session.push_input(1, "trigger", 1.0);
        session.activate(1);
        session.tick();
        assert_eq!(session.active_state(1), 1);

        session.restore_json(&snap).unwrap();
        assert_eq!(session.active_state(1), 0, "should restore to S0");
    }

    #[test]
    fn test_session_invalid_schema_errors() {
        // Test via weaven_core directly to avoid JsValue in native test env.
        let result = weaven_core::schema::load_schema("{ bad json }");
        assert!(result.is_err(), "invalid JSON should return a parse error");
    }

    #[test]
    fn test_session_inject_signal() {
        let mut session = WeavenSession::new();
        // Use fire_propagation.json as a real-world schema fixture
        let json = include_str!("../../fire_propagation.json");
        session.load_schema(json).unwrap();
        // SM 1 starts at state 0 (grass); inject fire signal
        let r = session.inject_signal(1, 0, 0, r#"{"intensity": 5.0}"#);
        assert!(r.is_ok());
        session.activate(1);
        session.tick();
        assert_eq!(session.active_state(1), 1, "SM1 should ignite");
    }

    #[test]
    fn test_session_spatial() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();
        session.enable_spatial(10.0);
        session.set_position(1, 0.0, 0.0);
        // No panic, spatial index updated
        assert_eq!(session.current_tick(), 0);
    }
}
