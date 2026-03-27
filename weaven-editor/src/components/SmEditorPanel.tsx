import { useState } from "react";
import { useEditorStore } from "../stores/editorStore";
import type { TransitionSchema, EffectSchema, ExprSchema, PortKind } from "../generated/schema";
import ExpressionBuilder from "./ExpressionBuilder";
import EffectEditor from "./EffectEditor";
import { defaultEffect } from "./EffectEditor";

export default function SmEditorPanel() {
  const selectedSmId = useEditorStore((s) => s.selectedSmId);
  const schema = useEditorStore((s) => s.schema);
  const addState = useEditorStore((s) => s.addState);
  const removeSm = useEditorStore((s) => s.removeSm);
  const addTransition = useEditorStore((s) => s.addTransition);
  const removeTransition = useEditorStore((s) => s.removeTransition);
  const updateTransition = useEditorStore((s) => s.updateTransition);
  const addPort = useEditorStore((s) => s.addPort);

  const [selectedTransitionId, setSelectedTransitionId] = useState<number | null>(null);
  const [newSource, setNewSource] = useState("");
  const [newTarget, setNewTarget] = useState("");
  const [newPriority, setNewPriority] = useState("10");
  const [newPortKind, setNewPortKind] = useState<PortKind>("Input");
  const [newSignalType, setNewSignalType] = useState("0");

  const sm = selectedSmId != null
    ? schema.state_machines.find((s) => s.id === selectedSmId)
    : null;

  const selectedTransition = sm && selectedTransitionId != null
    ? sm.transitions.find((t) => t.id === selectedTransitionId)
    : null;

  if (!sm) {
    return (
      <div className="p-4 text-gray-500">
        Select a state machine to edit.
      </div>
    );
  }

  const nextStateId = sm.states.length > 0 ? Math.max(...sm.states) + 1 : 0;
  const nextTransitionId = sm.transitions.length > 0
    ? Math.max(...sm.transitions.map((t) => t.id)) + 1
    : 1;
  const nextPortId = [...sm.input_ports, ...sm.output_ports].length > 0
    ? Math.max(...[...sm.input_ports, ...sm.output_ports].map((p) => p.id)) + 1
    : 0;

  const handleAddTransition = () => {
    const source = parseInt(newSource, 10);
    const target = parseInt(newTarget, 10);
    const priority = parseInt(newPriority, 10) || 10;
    if (isNaN(source) || isNaN(target)) return;
    addTransition(sm.id, {
      id: nextTransitionId,
      source,
      target,
      priority,
      effects: [],
    });
    setNewSource("");
    setNewTarget("");
  };

  const handleAddPort = () => {
    const sigType = parseInt(newSignalType, 10) || 0;
    const direction = newPortKind === "Input" || newPortKind === "ContinuousInput" ? "input" : "output";
    addPort(sm.id, direction, { id: nextPortId, kind: newPortKind, signal_type: sigType });
  };

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
        <div className="flex items-center justify-between mb-1">
          <h4 className="text-xs font-medium text-gray-400 uppercase">Transitions</h4>
        </div>
        {sm.transitions.length === 0 ? (
          <p className="text-xs text-gray-600">No transitions</p>
        ) : (
          <ul className="space-y-1">
            {sm.transitions.map((t) => (
              <li
                key={t.id}
                onClick={() => setSelectedTransitionId(t.id)}
                className={`text-xs px-2 py-1 rounded cursor-pointer flex items-center justify-between ${
                  t.id === selectedTransitionId
                    ? "bg-indigo-800 text-white"
                    : "bg-gray-800 text-gray-300 hover:bg-gray-700"
                }`}
              >
                <span>
                  T({t.id}): {t.source} → {t.target} (p={t.priority})
                  {t.guard && <span className="ml-1 text-yellow-400">[G]</span>}
                  {t.effects.length > 0 && (
                    <span className="ml-1 text-emerald-400">[{t.effects.length}E]</span>
                  )}
                </span>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    removeTransition(sm.id, t.id);
                    if (selectedTransitionId === t.id) setSelectedTransitionId(null);
                  }}
                  className="text-red-400 hover:text-red-300"
                  aria-label={`remove transition ${t.id}`}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
        <div className="flex items-center gap-1 mt-1">
          <input
            type="number"
            placeholder="Source"
            value={newSource}
            onChange={(e) => setNewSource(e.target.value)}
            className="w-14 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="transition source"
          />
          <input
            type="number"
            placeholder="Target"
            value={newTarget}
            onChange={(e) => setNewTarget(e.target.value)}
            className="w-14 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="transition target"
          />
          <input
            type="number"
            placeholder="Priority"
            value={newPriority}
            onChange={(e) => setNewPriority(e.target.value)}
            className="w-14 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="transition priority"
          />
          <button
            onClick={handleAddTransition}
            className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
          >
            Add Transition
          </button>
        </div>
      </section>

      {/* Transition detail editor */}
      {selectedTransition && (
        <section className="border-t border-gray-700 pt-3">
          <h4 className="text-xs font-medium text-gray-100 mb-2">
            Transition T({selectedTransition.id}) Detail
          </h4>

          {/* Priority */}
          <div className="flex items-center gap-2 mb-2">
            <label className="text-xs text-gray-400">Priority:</label>
            <input
              type="number"
              value={selectedTransition.priority}
              onChange={(e) =>
                updateTransition(sm.id, selectedTransition.id, {
                  priority: parseInt(e.target.value, 10) || 0,
                })
              }
              className="w-16 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
              aria-label="edit priority"
            />
          </div>

          {/* Guard */}
          <div className="mb-2">
            <div className="flex items-center justify-between mb-1">
              <span className="text-xs text-gray-400">Guard:</span>
              {selectedTransition.guard ? (
                <button
                  onClick={() => updateTransition(sm.id, selectedTransition.id, { guard: null })}
                  className="text-xs text-red-400 hover:text-red-300"
                  aria-label="remove guard"
                >
                  Remove Guard
                </button>
              ) : (
                <button
                  onClick={() =>
                    updateTransition(sm.id, selectedTransition.id, { guard: { Bool: true } })
                  }
                  className="text-xs text-indigo-400 hover:text-indigo-300"
                >
                  Add Guard
                </button>
              )}
            </div>
            {selectedTransition.guard && (
              <ExpressionBuilder
                expr={selectedTransition.guard}
                onChange={(guard) =>
                  updateTransition(sm.id, selectedTransition.id, { guard })
                }
              />
            )}
          </div>

          {/* Effects */}
          <div>
            <div className="flex items-center justify-between mb-1">
              <span className="text-xs text-gray-400">Effects:</span>
              <button
                onClick={() =>
                  updateTransition(sm.id, selectedTransition.id, {
                    effects: [...selectedTransition.effects, defaultEffect("Signal")],
                  })
                }
                className="text-xs text-indigo-400 hover:text-indigo-300"
              >
                Add Effect
              </button>
            </div>
            {selectedTransition.effects.length === 0 ? (
              <p className="text-xs text-gray-600">No effects</p>
            ) : (
              <div className="space-y-2">
                {selectedTransition.effects.map((eff, i) => (
                  <EffectEditor
                    key={i}
                    effect={eff}
                    onChange={(newEff) => {
                      const next = [...selectedTransition.effects];
                      next[i] = newEff;
                      updateTransition(sm.id, selectedTransition.id, { effects: next });
                    }}
                    onRemove={() => {
                      const next = selectedTransition.effects.filter((_, j) => j !== i);
                      updateTransition(sm.id, selectedTransition.id, { effects: next });
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        </section>
      )}

      {/* Ports */}
      <section>
        <div className="flex items-center justify-between mb-1">
          <h4 className="text-xs font-medium text-gray-400 uppercase">Ports</h4>
        </div>
        <ul className="space-y-1">
          {sm.input_ports.map((p) => (
            <li key={`in-${p.id}`} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
              {p.kind}:{p.id} (sig={p.signal_type})
            </li>
          ))}
          {sm.output_ports.map((p) => (
            <li key={`out-${p.id}`} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
              {p.kind}:{p.id} (sig={p.signal_type})
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-1 mt-1">
          <select
            value={newPortKind}
            onChange={(e) => setNewPortKind(e.target.value as PortKind)}
            className="text-xs bg-gray-800 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
            aria-label="port kind"
          >
            <option value="Input">Input</option>
            <option value="Output">Output</option>
            <option value="ContinuousInput">ContinuousInput</option>
            <option value="ContinuousOutput">ContinuousOutput</option>
          </select>
          <input
            type="number"
            placeholder="SigType"
            value={newSignalType}
            onChange={(e) => setNewSignalType(e.target.value)}
            className="w-14 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="signal type"
          />
          <button
            onClick={handleAddPort}
            className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
          >
            Add Port
          </button>
        </div>
      </section>
    </div>
  );
}
