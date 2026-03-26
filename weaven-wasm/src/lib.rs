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
    diff_snapshots, policy_filtered_diff, scoped_snapshot, interest_region_sms,
    rewind_and_resimulate, InputBuffer, TaggedInput, SmStateDiff,
    SmNetworkPolicy, Authority, SyncPolicy, ReconciliationPolicy,
};
use std::collections::{BTreeMap, BTreeSet};
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
    input_buffer: Option<InputBuffer>,
    last_snapshot: Option<WorldSnapshot>,
}

#[wasm_bindgen]
impl WeavenSession {
    /// Create a new empty session.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WeavenSession {
        WeavenSession {
            world: World::new(),
            input_buffer: None,
            last_snapshot: None,
        }
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

    // -- Network APIs (§8) ------------------------------------------------

    /// Compute the diff between two snapshot JSON strings.
    /// Returns a JSON array of `SmStateDiff`.
    pub fn diff_snapshots_json(&self, before_json: &str, after_json: &str) -> Result<String, JsValue> {
        let before = WorldSnapshot::from_json(before_json.as_bytes()).map_err(js_err)?;
        let after  = WorldSnapshot::from_json(after_json.as_bytes()).map_err(js_err)?;
        let diffs = diff_snapshots(&before, &after);
        serde_json::to_string(&diffs).map_err(js_err)
    }

    /// Register a network policy for an SM.
    /// `policy_json`: `{"sm_id":1,"authority":"Server","sync_policy":"StateSync","reconciliation":"Snap"}`
    pub fn set_network_policy(&mut self, policy_json: &str) -> Result<(), JsValue> {
        let raw: NetworkPolicySer = serde_json::from_str(policy_json).map_err(js_err)?;
        let policy = raw.to_runtime().map_err(js_err)?;
        self.world.network_policies.insert(policy.sm_id, policy);
        Ok(())
    }

    /// Filter a diff JSON array by registered network policies.
    /// Returns the filtered diff JSON array.
    pub fn policy_filtered_diff_json(&self, diffs_json: &str) -> Result<String, JsValue> {
        let diffs: Vec<SmStateDiff> = serde_json::from_str(diffs_json).map_err(js_err)?;
        let filtered = policy_filtered_diff(&diffs, &self.world.network_policies);
        serde_json::to_string(&filtered).map_err(js_err)
    }

    /// Take a scoped snapshot — only the listed SM IDs.
    /// `sm_ids_json`: a JSON array `[1,2,3]`.
    pub fn scoped_snapshot_json(&self, sm_ids_json: &str) -> Result<String, JsValue> {
        let ids: Vec<u32> = serde_json::from_str(sm_ids_json).map_err(js_err)?;
        let set: BTreeSet<SmId> = ids.into_iter().map(SmId).collect();
        let snap = scoped_snapshot(&self.world, &set);
        let json = String::from_utf8(snap.to_json()).map_err(js_err)?;
        Ok(json)
    }

    /// Return SM IDs within a spatial radius as a JSON array.
    pub fn interest_region_json(&self, cx: f32, cy: f32, radius: f32) -> String {
        let ids = interest_region_sms(&self.world, cx, cy, radius);
        let v: Vec<u32> = ids.into_iter().map(|id| id.0).collect();
        serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string())
    }

    /// Initialise the input buffer for rollback networking.
    pub fn init_input_buffer(&mut self, history_depth: u32) {
        self.input_buffer = Some(InputBuffer::new(history_depth));
    }

    /// Push a tagged input into the buffer.
    /// `input_json`: `{"tick":0,"target_sm":1,"target_port":0,"payload":{"key":1.0}}`
    pub fn push_tagged_input(&mut self, input_json: &str) -> Result<(), JsValue> {
        let buf = self.input_buffer.as_mut()
            .ok_or_else(|| JsValue::from_str("input buffer not initialised"))?;
        let raw: TaggedInputSer = serde_json::from_str(input_json).map_err(js_err)?;
        buf.push(TaggedInput {
            tick: raw.tick,
            target_sm: SmId(raw.target_sm),
            target_port: PortId(raw.target_port),
            signal: Signal {
                signal_type: SignalTypeId(0),
                payload: raw.payload,
            },
        });
        Ok(())
    }

    /// Apply buffered inputs for the current tick.
    pub fn apply_buffered_inputs(&mut self) -> Result<(), JsValue> {
        let buf = self.input_buffer.as_ref()
            .ok_or_else(|| JsValue::from_str("input buffer not initialised"))?;
        buf.apply_tick_inputs(&mut self.world);
        Ok(())
    }

    /// Store the current snapshot as the rewind base point.
    pub fn save_rewind_base(&mut self) {
        self.last_snapshot = Some(snapshot(&self.world));
    }

    /// Rewind to the saved base snapshot and re-simulate to `current_tick`,
    /// replaying all buffered inputs.
    pub fn rewind_to(&mut self, target_tick: u64, current_tick: u64) -> Result<(), JsValue> {
        let base = self.last_snapshot.as_ref()
            .ok_or_else(|| JsValue::from_str("no rewind base snapshot saved"))?
            .clone();
        let buf = self.input_buffer.as_ref()
            .ok_or_else(|| JsValue::from_str("input buffer not initialised"))?;
        rewind_and_resimulate(&mut self.world, &base, buf, target_tick, current_tick);
        Ok(())
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

#[derive(Deserialize)]
struct TaggedInputSer {
    tick:        u64,
    target_sm:   u32,
    target_port: u32,
    payload:     BTreeMap<String, f64>,
}

#[derive(Deserialize)]
struct NetworkPolicySer {
    sm_id:          u32,
    authority:      String,
    sync_policy:    serde_json::Value,
    reconciliation: serde_json::Value,
}

impl NetworkPolicySer {
    fn to_runtime(&self) -> Result<SmNetworkPolicy, String> {
        let authority = match self.authority.as_str() {
            "Server" => Authority::Server,
            "Owner"  => Authority::Owner,
            "Local"  => Authority::Local,
            other    => return Err(format!("unknown authority: {other}")),
        };
        let sync_policy = match &self.sync_policy {
            serde_json::Value::String(s) => match s.as_str() {
                "InputSync"  => SyncPolicy::InputSync,
                "StateSync"  => SyncPolicy::StateSync,
                "None"       => SyncPolicy::None,
                other        => return Err(format!("unknown sync_policy: {other}")),
            },
            serde_json::Value::Object(obj) => {
                if let Some(fields_val) = obj.get("ContextSync") {
                    let fields: Vec<String> = serde_json::from_value(
                        fields_val.get("fields").cloned().unwrap_or(serde_json::Value::Array(vec![]))
                    ).map_err(|e| e.to_string())?;
                    SyncPolicy::ContextSync { fields }
                } else {
                    return Err("unknown sync_policy object".to_string());
                }
            },
            _ => return Err("invalid sync_policy format".to_string()),
        };
        let reconciliation = match &self.reconciliation {
            serde_json::Value::String(s) => match s.as_str() {
                "Snap"   => ReconciliationPolicy::Snap,
                "Rewind" => ReconciliationPolicy::Rewind,
                other    => return Err(format!("unknown reconciliation: {other}")),
            },
            serde_json::Value::Object(obj) => {
                if let Some(val) = obj.get("Interpolate") {
                    let blend_ticks = val.get("blend_ticks")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as u32;
                    ReconciliationPolicy::Interpolate { blend_ticks }
                } else {
                    return Err("unknown reconciliation object".to_string());
                }
            },
            _ => return Err("invalid reconciliation format".to_string()),
        };
        Ok(SmNetworkPolicy {
            sm_id: SmId(self.sm_id),
            authority,
            sync_policy,
            reconciliation,
        })
    }
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
    fn test_session_diff_snapshots() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();
        let before = session.snapshot_json();
        session.push_input(1, "trigger", 1.0);
        session.activate(1);
        session.tick();
        let after = session.snapshot_json();

        let diffs_json = session.diff_snapshots_json(&before, &after).unwrap();
        let diffs: Vec<serde_json::Value> = serde_json::from_str(&diffs_json).unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0]["sm_id"], 1);
        assert_eq!(diffs[0]["prev_state"], 0);
        assert_eq!(diffs[0]["new_state"], 1);
    }

    #[test]
    fn test_session_network_policy_and_filter() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();

        session.set_network_policy(r#"{
            "sm_id": 1,
            "authority": "Server",
            "sync_policy": "StateSync",
            "reconciliation": "Snap"
        }"#).unwrap();

        let before = session.snapshot_json();
        session.push_input(1, "trigger", 1.0);
        session.activate(1);
        session.tick();
        let after = session.snapshot_json();

        let diffs_json = session.diff_snapshots_json(&before, &after).unwrap();
        let filtered_json = session.policy_filtered_diff_json(&diffs_json).unwrap();
        let filtered: Vec<serde_json::Value> = serde_json::from_str(&filtered_json).unwrap();
        assert_eq!(filtered.len(), 1);
        // StateSync strips context changes
        assert!(filtered[0]["context_changes"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_session_scoped_snapshot() {
        let mut session = WeavenSession::new();
        // Load a schema with 2 SMs
        let schema = r#"{
            "state_machines": [
                {"id":1,"states":[0],"initial_state":0,"transitions":[],"input_ports":[],"output_ports":[],"elapse_capability":"NonElapsable"},
                {"id":2,"states":[0],"initial_state":0,"transitions":[],"input_ports":[],"output_ports":[],"elapse_capability":"NonElapsable"}
            ],
            "connections": [],
            "named_tables": []
        }"#;
        session.load_schema(schema).unwrap();

        let snap_json = session.scoped_snapshot_json("[1]").unwrap();
        let snap: serde_json::Value = serde_json::from_str(&snap_json).unwrap();
        assert_eq!(snap["instances"].as_array().unwrap().len(), 1);
        assert_eq!(snap["instances"][0]["sm_id"], 1);
    }

    #[test]
    fn test_session_input_buffer_and_rewind() {
        let mut session = WeavenSession::new();
        session.load_schema(simple_schema()).unwrap();
        session.activate(1);
        session.init_input_buffer(10);
        session.save_rewind_base();

        // Push a tagged input that sets trigger
        session.push_tagged_input(r#"{
            "tick": 0,
            "target_sm": 1,
            "target_port": 0,
            "payload": {"trigger": 1.0}
        }"#).unwrap();

        // Apply and tick
        session.apply_buffered_inputs().unwrap();
        session.tick();
        assert_eq!(session.active_state(1), 1, "should transition");

        // Rewind and replay
        session.rewind_to(0, 1).unwrap();
        assert_eq!(session.active_state(1), 1,
            "should still be S1 after rewind+replay");
    }

    #[test]
    fn test_session_interest_region() {
        let mut session = WeavenSession::new();
        let schema = r#"{
            "state_machines": [
                {"id":1,"states":[0],"initial_state":0,"transitions":[],"input_ports":[],"output_ports":[],"elapse_capability":"NonElapsable"},
                {"id":2,"states":[0],"initial_state":0,"transitions":[],"input_ports":[],"output_ports":[],"elapse_capability":"NonElapsable"}
            ],
            "connections": [],
            "named_tables": []
        }"#;
        session.load_schema(schema).unwrap();
        session.enable_spatial(10.0);
        session.set_position(1, 0.0, 0.0);
        session.set_position(2, 100.0, 100.0);

        let json = session.interest_region_json(0.0, 0.0, 5.0);
        let ids: Vec<u32> = serde_json::from_str(&json).unwrap();
        assert!(ids.contains(&1));
        assert!(!ids.contains(&2));
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
