//! Tauri commands — the IPC boundary between Rust backend and frontend.
//!
//! Design doc §6.1: command list.

use std::sync::Mutex;
use tauri::State;

use weaven_debugger_core::debug_session::{DebugSession, TickResult, WorldState};
use weaven_debugger_core::topology::{TopologyGraph, build_topology, add_ir_edges_from_trace};

/// Shared state managed by Tauri.
pub struct AppState {
    pub session: Mutex<Option<DebugSession>>,
}

/// Load a weaven-schema JSON file and initialize the debug session.
#[tauri::command]
pub fn load_schema(path: String, state: State<'_, AppState>) -> Result<TopologyGraph, String> {
    let json_str = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let schema: weaven_core::schema::WeavenSchema = serde_json::from_str(&json_str)
        .map_err(|e| format!("Invalid schema JSON: {}", e))?;

    let compiled = weaven_core::schema::compile_schema(&schema);

    let mut world = weaven_core::World::new();

    // Register SMs.
    for sm_def in compiled.sm_defs {
        world.register_sm(sm_def);
    }

    // Register connections.
    for conn in compiled.connections {
        world.connections.push(conn);
    }

    let topology = build_topology(&world);
    let session = DebugSession::new(world);

    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    *guard = Some(session);

    Ok(topology)
}

/// Advance one tick.
#[tauri::command]
pub fn tick(state: State<'_, AppState>) -> Result<TickResult, String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("No session loaded")?;
    Ok(session.tick())
}

/// Advance N ticks.
#[tauri::command]
pub fn tick_n(n: u32, state: State<'_, AppState>) -> Result<TickResult, String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("No session loaded")?;
    Ok(session.tick_n(n))
}

/// Seek to a specific tick (snapshot restore + re-simulate).
#[tauri::command]
pub fn seek_tick(tick: u64, state: State<'_, AppState>) -> Result<WorldState, String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("No session loaded")?;
    Ok(session.seek_tick(tick))
}

/// Get the current topology graph.
#[tauri::command]
pub fn get_topology(state: State<'_, AppState>) -> Result<TopologyGraph, String> {
    let guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_ref().ok_or("No session loaded")?;
    let mut graph = build_topology(&session.world);

    // Augment with IR edges from trace of current tick.
    let trace = session.trace_for_tick(session.current_tick());
    add_ir_edges_from_trace(&mut graph, &trace);

    Ok(graph)
}

/// Get cascade steps for a specific tick.
#[tauri::command]
pub fn get_cascade_steps(tick: u64, state: State<'_, AppState>) -> Result<Vec<weaven_core::trace::TraceEvent>, String> {
    let guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_ref().ok_or("No session loaded")?;
    let events = session.trace_for_tick(tick);
    let cascade: Vec<_> = events.into_iter()
        .filter(|e| matches!(e, weaven_core::trace::TraceEvent::CascadeStep { .. }))
        .collect();
    Ok(cascade)
}

/// Inject a debug signal into a port (for testing).
#[tauri::command]
pub fn inject_signal(
    sm_id: u32,
    port_id: u32,
    payload: std::collections::BTreeMap<String, f64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = state.session.lock().map_err(|e| e.to_string())?;
    let session = guard.as_mut().ok_or("No session loaded")?;

    let signal = weaven_core::Signal {
        signal_type: weaven_core::SignalTypeId(0),
        payload,
    };
    session.world.signal_queue.push_back(weaven_core::QueuedSignal {
        target_sm: weaven_core::SmId(sm_id),
        target_port: weaven_core::PortId(port_id),
        signal,
        delay: 0,
        source_conn: None,
    });
    session.world.active_set.insert(weaven_core::SmId(sm_id));

    Ok(())
}
