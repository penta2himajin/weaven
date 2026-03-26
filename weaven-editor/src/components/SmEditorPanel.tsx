import { useEditorStore } from "../stores/editorStore";

export default function SmEditorPanel() {
  const selectedSmId = useEditorStore((s) => s.selectedSmId);
  const schema = useEditorStore((s) => s.schema);
  const addState = useEditorStore((s) => s.addState);
  const removeSm = useEditorStore((s) => s.removeSm);

  const sm = selectedSmId != null
    ? schema.state_machines.find((s) => s.id === selectedSmId)
    : null;

  if (!sm) {
    return (
      <div className="p-4 text-gray-500">
        Select a state machine to edit.
      </div>
    );
  }

  const nextStateId = sm.states.length > 0 ? Math.max(...sm.states) + 1 : 0;

  return (
    <div className="p-4 flex flex-col gap-3 overflow-y-auto">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-100">SM({sm.id})</h3>
        <button
          onClick={() => removeSm(sm.id)}
          className="px-2 py-1 text-xs rounded bg-red-800 hover:bg-red-700 text-gray-200"
        >
          Delete SM
        </button>
      </div>

      {/* States */}
      <section>
        <div className="flex items-center justify-between mb-1">
          <h4 className="text-xs font-medium text-gray-400 uppercase">States</h4>
          <button
            onClick={() => addState(sm.id, nextStateId)}
            className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
          >
            Add State
          </button>
        </div>
        <ul className="space-y-1">
          {sm.states.map((stateId) => (
            <li key={stateId} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
              State {stateId}
              {stateId === sm.initial_state && (
                <span className="ml-2 text-indigo-400">(initial)</span>
              )}
            </li>
          ))}
        </ul>
      </section>

      {/* Transitions */}
      <section>
        <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Transitions</h4>
        {sm.transitions.length === 0 ? (
          <p className="text-xs text-gray-600">No transitions</p>
        ) : (
          <ul className="space-y-1">
            {sm.transitions.map((t) => (
              <li key={t.id} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
                T({t.id}): {t.source} → {t.target} (p={t.priority})
              </li>
            ))}
          </ul>
        )}
      </section>

      {/* Ports */}
      <section>
        <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Ports</h4>
        <ul className="space-y-1">
          {sm.input_ports.map((p) => (
            <li key={`in-${p.id}`} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
              Input:{p.id} (sig={p.signal_type})
            </li>
          ))}
          {sm.output_ports.map((p) => (
            <li key={`out-${p.id}`} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
              Output:{p.id} (sig={p.signal_type})
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
