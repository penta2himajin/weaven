/// Weaven Unity Adapter — C ABI FFI bridge (Phase 5, §12.5).
///
/// Exposes weaven-core through a C-compatible FFI layer for consumption by
/// Unity as a native plugin. All data exchange across the FFI boundary uses
/// JSON strings (UTF-8, null-terminated) or primitive scalars.
///
/// Ownership model:
///   - `weaven_create()` allocates a boxed World, returns an opaque handle.
///   - All `weaven_*` functions take the handle as the first argument.
///   - `weaven_destroy()` frees the handle. Using it after destroy is UB.
///   - String results are owned by Rust; call `weaven_free_string()` to free.
///
/// Thread safety: NOT thread-safe. Call all functions from one thread (Unity main thread).

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use weaven_core::{
    World, SmId, PortId, Signal, SignalTypeId,
    SystemCommand, snapshot, restore, WorldSnapshot,
};

// ---------------------------------------------------------------------------
// Opaque handle
// ---------------------------------------------------------------------------

/// Opaque handle to a Weaven World instance.
pub struct WeavenHandle {
    world: World,
    /// Cached JSON from the last tick (kept alive until next tick or free).
    last_tick_json: Option<CString>,
    /// Cached snapshot JSON.
    last_snapshot_json: Option<CString>,
}

impl WeavenHandle {
    fn new() -> Self {
        Self {
            world: World::new(),
            last_tick_json: None,
            last_snapshot_json: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: C string conversion
// ---------------------------------------------------------------------------

unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() { return None; }
    CStr::from_ptr(ptr).to_str().ok()
}

fn string_to_c(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Create a new Weaven World. Returns an opaque handle.
/// Caller must eventually call `weaven_destroy()`.
#[no_mangle]
pub extern "C" fn weaven_create() -> *mut WeavenHandle {
    Box::into_raw(Box::new(WeavenHandle::new()))
}

/// Destroy a Weaven World and free all associated memory.
///
/// # Safety
/// `handle` must be a valid pointer returned by `weaven_create()`.
/// Using the handle after this call is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn weaven_destroy(handle: *mut WeavenHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Free a string previously returned by a `weaven_*` function.
///
/// # Safety
/// `ptr` must be a pointer returned by a `weaven_*` function, or null.
#[no_mangle]
pub unsafe extern "C" fn weaven_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

// ---------------------------------------------------------------------------
// Schema loading
// ---------------------------------------------------------------------------

/// Load a Weaven schema from a JSON string.
/// Returns 0 on success, -1 on error.
/// On error, call `weaven_last_error()` for the error message.
///
/// # Safety
/// `handle` must be valid. `json` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn weaven_load_schema(
    handle: *mut WeavenHandle,
    json: *const c_char,
) -> i32 {
    let h = &mut *handle;
    let json_str = match cstr_to_str(json) {
        Some(s) => s,
        None => return -1,
    };
    match load_schema_inner(&mut h.world, json_str) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn load_schema_inner(world: &mut World, json: &str) -> Result<(), Box<dyn std::error::Error>> {
    use weaven_core::schema::{load_schema, compile_schema};
    let schema = load_schema(json)?;
    let compiled = compile_schema(&schema);
    for def in compiled.sm_defs {
        world.register_sm(def);
    }
    for conn in compiled.connections {
        world.connect(conn);
    }
    world.tables = compiled.table_registry;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

/// Advance the simulation by one tick.
/// Returns a JSON string describing state changes. Caller must free with
/// `weaven_free_string()`.
///
/// JSON format: `{"state_changes":{"<sm_id>":[<prev>,<next>],...},"tick":<n>}`
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_tick(handle: *mut WeavenHandle) -> *const c_char {
    let h = &mut *handle;
    let output = weaven_core::tick(&mut h.world);

    let mut changes = BTreeMap::new();
    for (sm_id, (prev, next)) in &output.state_changes {
        changes.insert(sm_id.0.to_string(), vec![prev.0, next.0]);
    }

    let mut cmds: Vec<serde_json::Value> = Vec::new();
    for cmd in &output.system_commands {
        match cmd {
            SystemCommand::HitStop { frames } => {
                cmds.push(serde_json::json!({"HitStop": {"frames": frames}}));
            }
            SystemCommand::SlowMotion { factor, duration_ticks } => {
                cmds.push(serde_json::json!({"SlowMotion": {"factor": factor, "duration_ticks": duration_ticks}}));
            }
            SystemCommand::TimeScale(s) => {
                cmds.push(serde_json::json!({"TimeScale": s}));
            }
        }
    }

    let json = serde_json::json!({
        "state_changes": changes,
        "system_commands": cmds,
        "tick": h.world.tick,
    });

    let json_str = json.to_string();
    let cs = CString::new(json_str).unwrap();
    let ptr = cs.as_ptr();
    h.last_tick_json = Some(cs);
    ptr
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Push a continuous input value into an SM's context field.
///
/// # Safety
/// `handle` must be valid. `field` must be a valid null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn weaven_push_input(
    handle: *mut WeavenHandle,
    sm_id: u32,
    field: *const c_char,
    value: f64,
) {
    let h = &mut *handle;
    let field_str = match cstr_to_str(field) {
        Some(s) => s,
        None => return,
    };
    if let Some(inst) = h.world.instances.get_mut(&SmId(sm_id)) {
        inst.context.set(field_str, value);
    }
}

/// Inject a discrete signal into an SM's input port.
/// `payload_json` is a JSON object `{"key": value, ...}`.
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `handle` must be valid. `payload_json` must be valid null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn weaven_inject_signal(
    handle: *mut WeavenHandle,
    sm_id: u32,
    port_id: u32,
    payload_json: *const c_char,
) -> i32 {
    let h = &mut *handle;
    let json_str = match cstr_to_str(payload_json) {
        Some(s) => s,
        None => return -1,
    };
    let payload: BTreeMap<String, f64> = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(_) => return -1,
    };
    h.world.inject_signal(
        SmId(sm_id),
        PortId(port_id),
        Signal { signal_type: SignalTypeId(0), payload },
    );
    0
}

// ---------------------------------------------------------------------------
// Output reading
// ---------------------------------------------------------------------------

/// Read a context field value from an SM.
/// Returns the field value, or 0.0 if the SM or field doesn't exist.
///
/// # Safety
/// `handle` must be valid. `field` must be valid null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn weaven_read_output(
    handle: *const WeavenHandle,
    sm_id: u32,
    field: *const c_char,
) -> f64 {
    let h = &*handle;
    let field_str = match cstr_to_str(field) {
        Some(s) => s,
        None => return 0.0,
    };
    h.world.instances
        .get(&SmId(sm_id))
        .map(|i| i.context.get(field_str))
        .unwrap_or(0.0)
}

/// Read the active state of an SM. Returns the state ID, or -1 if SM not found.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_active_state(
    handle: *const WeavenHandle,
    sm_id: u32,
) -> i32 {
    let h = &*handle;
    h.world.instances
        .get(&SmId(sm_id))
        .map(|i| i.active_state.0 as i32)
        .unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// Activation
// ---------------------------------------------------------------------------

/// Mark an SM for evaluation in the next tick.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_activate(handle: *mut WeavenHandle, sm_id: u32) {
    let h = &mut *handle;
    h.world.activate(SmId(sm_id));
}

// ---------------------------------------------------------------------------
// Spatial
// ---------------------------------------------------------------------------

/// Enable spatial indexing with the given cell size.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_enable_spatial(handle: *mut WeavenHandle, cell_size: f64) {
    let h = &mut *handle;
    h.world.enable_spatial(cell_size);
}

/// Update an SM's spatial position.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_set_position(
    handle: *mut WeavenHandle,
    sm_id: u32,
    x: f64,
    y: f64,
) {
    let h = &mut *handle;
    h.world.set_position(SmId(sm_id), x, y);
}

/// Query SM IDs within a radius. Returns a JSON array `[1,2,3]`.
/// Caller must free the returned string with `weaven_free_string()`.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_query_radius(
    handle: *const WeavenHandle,
    x: f64,
    y: f64,
    radius: f64,
) -> *mut c_char {
    let h = &*handle;
    let ids: Vec<u32> = h.world.query_radius(x, y, radius)
        .into_iter()
        .map(|id| id.0)
        .collect();
    string_to_c(serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string()))
}

// ---------------------------------------------------------------------------
// Snapshot / Restore
// ---------------------------------------------------------------------------

/// Take a snapshot of the current world state. Returns a JSON string.
/// Caller must free with `weaven_free_string()`.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_snapshot(handle: *mut WeavenHandle) -> *const c_char {
    let h = &mut *handle;
    let snap = snapshot(&h.world);
    let json_str = String::from_utf8(snap.to_json()).unwrap_or_default();
    let cs = CString::new(json_str).unwrap();
    let ptr = cs.as_ptr();
    h.last_snapshot_json = Some(cs);
    ptr
}

/// Restore world state from a snapshot JSON string.
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `handle` must be valid. `json` must be valid null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn weaven_restore(
    handle: *mut WeavenHandle,
    json: *const c_char,
) -> i32 {
    let h = &mut *handle;
    let json_str = match cstr_to_str(json) {
        Some(s) => s,
        None => return -1,
    };
    match WorldSnapshot::from_json(json_str.as_bytes()) {
        Ok(snap) => {
            restore(&mut h.world, &snap);
            0
        }
        Err(_) => -1,
    }
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Get the current tick number.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_current_tick(handle: *const WeavenHandle) -> u64 {
    let h = &*handle;
    h.world.tick
}

/// Get all registered SM IDs as a JSON array. Caller must free the result.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn weaven_sm_ids(handle: *const WeavenHandle) -> *mut c_char {
    let h = &*handle;
    let ids: Vec<u32> = h.world.defs.keys().map(|id| id.0).collect();
    string_to_c(serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string()))
}

// ---------------------------------------------------------------------------
// Spawn / Despawn
// ---------------------------------------------------------------------------

/// Request spawn of SMs. `sm_ids_json` is a JSON array `[1,2,3]`.
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `handle` must be valid. `sm_ids_json` must be valid null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn weaven_request_spawn(
    handle: *mut WeavenHandle,
    sm_ids_json: *const c_char,
) -> i32 {
    let h = &mut *handle;
    let json_str = match cstr_to_str(sm_ids_json) {
        Some(s) => s,
        None => return -1,
    };
    let ids: Vec<u32> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    h.world.request_spawn(
        ids.into_iter().map(SmId).collect(),
        vec![],
    );
    0
}

/// Request despawn of SMs. `sm_ids_json` is a JSON array `[1,2,3]`.
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `handle` must be valid. `sm_ids_json` must be valid null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn weaven_request_despawn(
    handle: *mut WeavenHandle,
    sm_ids_json: *const c_char,
) -> i32 {
    let h = &mut *handle;
    let json_str = match cstr_to_str(sm_ids_json) {
        Some(s) => s,
        None => return -1,
    };
    let ids: Vec<u32> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return -1,
    };
    h.world.request_despawn(ids.into_iter().map(SmId).collect());
    0
}

// ---------------------------------------------------------------------------
// Tests (Rust-side FFI correctness)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use weaven_core::*;
    use std::ffi::CString;

    fn make_simple_sm(id: SmId) -> SmDef {
        SmDef::new(
            id,
            [StateId(0), StateId(1)],
            StateId(0),
            vec![Transition {
                id: TransitionId(id.0 * 10),
                source: StateId(0), target: StateId(1), priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
                effects: vec![],
            }],
            vec![], vec![],
        )
    }

    #[test]
    fn test_create_destroy() {
        unsafe {
            let h = weaven_create();
            assert!(!h.is_null());
            weaven_destroy(h);
        }
    }

    #[test]
    fn test_tick_returns_json() {
        unsafe {
            let h = weaven_create();
            (*h).world.register_sm(make_simple_sm(SmId(1)));

            let field = CString::new("trigger").unwrap();
            weaven_push_input(h, 1, field.as_ptr(), 1.0);
            weaven_activate(h, 1);
            let result = weaven_tick(h);
            assert!(!result.is_null());

            let json_str = CStr::from_ptr(result).to_str().unwrap();
            let v: serde_json::Value = serde_json::from_str(json_str).unwrap();
            assert!(v["state_changes"].get("1").is_some(),
                "SM(1) should appear in state_changes");

            weaven_destroy(h);
        }
    }

    #[test]
    fn test_read_output_and_active_state() {
        unsafe {
            let h = weaven_create();
            (*h).world.register_sm(make_simple_sm(SmId(1)));

            let field = CString::new("speed").unwrap();
            if let Some(inst) = (*h).world.instances.get_mut(&SmId(1)) {
                inst.context.set("speed", 42.0);
            }

            assert_eq!(weaven_read_output(h, 1, field.as_ptr()), 42.0);
            assert_eq!(weaven_active_state(h, 1), 0);
            assert_eq!(weaven_active_state(h, 999), -1);

            weaven_destroy(h);
        }
    }

    #[test]
    fn test_inject_signal() {
        unsafe {
            let h = weaven_create();

            let sig = SignalTypeId(0);
            (*h).world.register_sm(SmDef {
                id: SmId(1),
                states: [StateId(0), StateId(1)].into_iter().collect(),
                initial_state: StateId(0),
                transitions: vec![Transition {
                    id: TransitionId(10),
                    source: StateId(0), target: StateId(1), priority: 10,
                    guard: Some(Box::new(|_ctx, sig| {
                        sig.map_or(false, |s| s.payload.get("damage").copied().unwrap_or(0.0) > 0.0)
                    })),
                    effects: vec![],
                }],
                input_ports: vec![Port::new(PortId(0), PortKind::Input, sig)],
                output_ports: vec![],
                on_despawn_transitions: vec![],
                elapse_capability: ElapseCapabilityRt::NonElapsable,
                elapse_fn: None,
            });

            let payload = CString::new(r#"{"damage": 10.0}"#).unwrap();
            let rc = weaven_inject_signal(h, 1, 0, payload.as_ptr());
            assert_eq!(rc, 0);

            weaven_activate(h, 1);
            weaven_tick(h);

            assert_eq!(weaven_active_state(h, 1), 1,
                "SM should transition after signal injection");

            weaven_destroy(h);
        }
    }

    #[test]
    fn test_snapshot_restore() {
        unsafe {
            let h = weaven_create();
            (*h).world.register_sm(make_simple_sm(SmId(1)));

            // Take snapshot at S0
            let snap_ptr = weaven_snapshot(h);
            assert!(!snap_ptr.is_null());
            let snap_json = CStr::from_ptr(snap_ptr).to_str().unwrap().to_owned();

            // Transition to S1
            let field = CString::new("trigger").unwrap();
            weaven_push_input(h, 1, field.as_ptr(), 1.0);
            weaven_activate(h, 1);
            weaven_tick(h);
            assert_eq!(weaven_active_state(h, 1), 1);

            // Restore to S0
            let snap_cstr = CString::new(snap_json).unwrap();
            let rc = weaven_restore(h, snap_cstr.as_ptr());
            assert_eq!(rc, 0);
            assert_eq!(weaven_active_state(h, 1), 0, "should restore to S0");

            weaven_destroy(h);
        }
    }

    #[test]
    fn test_spatial() {
        unsafe {
            let h = weaven_create();
            weaven_enable_spatial(h, 10.0);

            for id in 1u32..=2 {
                (*h).world.register_sm(SmDef::new(
                    SmId(id), [StateId(0)], StateId(0), vec![], vec![], vec![],
                ));
            }

            weaven_set_position(h, 1, 0.0, 0.0);
            weaven_set_position(h, 2, 3.0, 0.0);

            let result = weaven_query_radius(h, 0.0, 0.0, 5.0);
            let json_str = CStr::from_ptr(result).to_str().unwrap();
            let ids: Vec<u32> = serde_json::from_str(json_str).unwrap();
            assert!(ids.contains(&1));
            assert!(ids.contains(&2));

            weaven_free_string(result);
            weaven_destroy(h);
        }
    }

    #[test]
    fn test_current_tick() {
        unsafe {
            let h = weaven_create();
            (*h).world.register_sm(make_simple_sm(SmId(1)));
            assert_eq!(weaven_current_tick(h), 0);

            weaven_tick(h);
            assert_eq!(weaven_current_tick(h), 1);

            weaven_tick(h);
            assert_eq!(weaven_current_tick(h), 2);

            weaven_destroy(h);
        }
    }

    #[test]
    fn test_sm_ids() {
        unsafe {
            let h = weaven_create();
            (*h).world.register_sm(make_simple_sm(SmId(1)));
            (*h).world.register_sm(make_simple_sm(SmId(2)));

            let result = weaven_sm_ids(h);
            let json_str = CStr::from_ptr(result).to_str().unwrap();
            let ids: Vec<u32> = serde_json::from_str(json_str).unwrap();
            assert!(ids.contains(&1));
            assert!(ids.contains(&2));

            weaven_free_string(result);
            weaven_destroy(h);
        }
    }

    #[test]
    fn test_free_string_null_safe() {
        unsafe {
            weaven_free_string(ptr::null_mut()); // Should not panic
        }
    }

    #[test]
    fn test_destroy_null_safe() {
        unsafe {
            weaven_destroy(ptr::null_mut()); // Should not panic
        }
    }

    #[test]
    fn test_load_schema() {
        unsafe {
            let h = weaven_create();
            let json = CString::new(r#"{
                "state_machines": [{
                    "id": 1,
                    "states": [0, 1],
                    "initial_state": 0,
                    "transitions": [{
                        "id": 10,
                        "source": 0, "target": 1, "priority": 10,
                        "guard": { "CtxField": "hp" },
                        "effects": []
                    }],
                    "input_ports": [],
                    "output_ports": [],
                    "elapse_capability": "NonElapsable"
                }],
                "connections": [],
                "named_tables": []
            }"#).unwrap();

            let rc = weaven_load_schema(h, json.as_ptr());
            assert_eq!(rc, 0);
            assert!((*h).world.defs.contains_key(&SmId(1)));

            weaven_destroy(h);
        }
    }

    #[test]
    fn test_load_schema_invalid() {
        unsafe {
            let h = weaven_create();
            let json = CString::new("not valid json").unwrap();
            let rc = weaven_load_schema(h, json.as_ptr());
            assert_eq!(rc, -1);
            weaven_destroy(h);
        }
    }

    #[test]
    fn test_spawn_despawn() {
        unsafe {
            let h = weaven_create();
            (*h).world.register_sm(make_simple_sm(SmId(1)));

            let ids = CString::new("[1]").unwrap();
            let rc = weaven_request_spawn(h, ids.as_ptr());
            assert_eq!(rc, 0);

            let rc = weaven_request_despawn(h, ids.as_ptr());
            assert_eq!(rc, 0);

            weaven_destroy(h);
        }
    }
}
