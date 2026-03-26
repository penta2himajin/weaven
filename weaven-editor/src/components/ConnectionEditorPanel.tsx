import { useState } from "react";
import { useEditorStore } from "../stores/editorStore";
import type { PipelineStepSchema } from "../generated/schema";

type StepKind = "Transform" | "Filter" | "Redirect";

function defaultStep(kind: StepKind): PipelineStepSchema {
  switch (kind) {
    case "Transform":
      return { Transform: {} };
    case "Filter":
      return { Filter: { Bool: true } };
    case "Redirect":
      return { Redirect: 0 };
  }
}

function stepKind(step: PipelineStepSchema): string {
  return Object.keys(step)[0];
}

export default function ConnectionEditorPanel() {
  const selectedConnectionId = useEditorStore((s) => s.selectedConnectionId);
  const schema = useEditorStore((s) => s.schema);
  const removeConnection = useEditorStore((s) => s.removeConnection);
  const addPipelineStep = useEditorStore((s) => s.addPipelineStep);
  const removePipelineStep = useEditorStore((s) => s.removePipelineStep);
  const updateConnectionDelay = useEditorStore((s) => s.updateConnectionDelay);
  const [newStepKind, setNewStepKind] = useState<StepKind>("Transform");

  const conn = selectedConnectionId != null
    ? schema.connections.find((c) => c.id === selectedConnectionId)
    : null;

  if (!conn) {
    return (
      <div className="p-4 text-gray-500">
        Select a connection to edit.
      </div>
    );
  }

  return (
    <div className="p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-100">Connection({conn.id})</h3>
        <button
          onClick={() => removeConnection(conn.id)}
          className="px-2 py-1 text-xs rounded bg-red-800 hover:bg-red-700 text-gray-200"
        >
          Delete Connection
        </button>
      </div>

      <div className="text-xs text-gray-300 space-y-1">
        <p>SM({conn.source_sm}) → SM({conn.target_sm})</p>
        <p>Port: {conn.source_port} → {conn.target_port}</p>
      </div>

      <div className="flex items-center gap-2">
        <label className="text-xs text-gray-400">Delay:</label>
        <input
          type="number"
          min={0}
          value={conn.delay_ticks}
          onChange={(e) => updateConnectionDelay(conn.id, parseInt(e.target.value, 10) || 0)}
          className="w-16 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="delay ticks"
        />
        <span className="text-xs text-gray-500">ticks</span>
      </div>

      <section>
        <div className="flex items-center justify-between mb-1">
          <h4 className="text-xs font-medium text-gray-400 uppercase">Pipeline</h4>
          <div className="flex items-center gap-1">
            <select
              value={newStepKind}
              onChange={(e) => setNewStepKind(e.target.value as StepKind)}
              className="text-xs bg-gray-800 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
              aria-label="step type"
            >
              <option value="Transform">Transform</option>
              <option value="Filter">Filter</option>
              <option value="Redirect">Redirect</option>
            </select>
            <button
              onClick={() => addPipelineStep(conn.id, defaultStep(newStepKind))}
              className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
            >
              Add Step
            </button>
          </div>
        </div>
        {conn.pipeline.length === 0 ? (
          <p className="text-xs text-gray-600">No pipeline steps</p>
        ) : (
          <ul className="space-y-1">
            {conn.pipeline.map((step, i) => (
              <li key={i} className="flex items-center justify-between text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
                <span>{stepKind(step)}</span>
                <button
                  onClick={() => removePipelineStep(conn.id, i)}
                  className="text-red-400 hover:text-red-300 text-xs"
                  aria-label={`remove step ${i}`}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
