import { useState } from "react";
import { useEditorStore } from "../stores/editorStore";
import type { IRConditionSchema } from "../generated/schema";

export default function IREditorPanel() {
  const schema = useEditorStore((s) => s.schema);
  const selectedIrId = useEditorStore((s) => s.selectedInteractionRuleId);
  const addInteractionRule = useEditorStore((s) => s.addInteractionRule);
  const removeInteractionRule = useEditorStore((s) => s.removeInteractionRule);
  const updateInteractionRule = useEditorStore((s) => s.updateInteractionRule);
  const selectInteractionRule = useEditorStore((s) => s.selectInteractionRule);

  const [newSmId, setNewSmId] = useState("");
  const [newRequiredState, setNewRequiredState] = useState("");
  const [newCondKind, setNewCondKind] = useState<"Spatial" | "Guard">("Spatial");
  const [newRadius, setNewRadius] = useState("10");

  const selectedRule = selectedIrId != null
    ? schema.interaction_rules.find((r) => r.id === selectedIrId)
    : null;

  return (
    <div className="p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-100">Interaction Rules</h3>
        <button
          onClick={addInteractionRule}
          className="px-2 py-0.5 text-xs rounded bg-indigo-600 hover:bg-indigo-500 text-white"
        >
          Add IR
        </button>
      </div>

      {schema.interaction_rules.length === 0 ? (
        <p className="text-xs text-gray-600">No interaction rules</p>
      ) : (
        <ul className="space-y-1">
          {schema.interaction_rules.map((rule) => (
            <li
              key={rule.id}
              onClick={() => selectInteractionRule(rule.id)}
              className={`text-xs px-2 py-1 rounded cursor-pointer ${
                rule.id === selectedIrId
                  ? "bg-indigo-800 text-white"
                  : "bg-gray-800 text-gray-300 hover:bg-gray-700"
              }`}
            >
              IR({rule.id}) — {rule.participants.length} participants, {rule.conditions.length} conditions
            </li>
          ))}
        </ul>
      )}

      {selectedRule && (
        <div className="border-t border-gray-700 pt-3 flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <h4 className="text-xs font-medium text-gray-100">IR({selectedRule.id})</h4>
            <button
              onClick={() => removeInteractionRule(selectedRule.id)}
              className="px-2 py-0.5 text-xs rounded bg-red-800 hover:bg-red-700 text-gray-200"
            >
              Delete IR
            </button>
          </div>

          {/* Participants */}
          <section>
            <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Participants</h4>
            {selectedRule.participants.length === 0 ? (
              <p className="text-xs text-gray-600">No participants</p>
            ) : (
              <ul className="space-y-1">
                {selectedRule.participants.map((p, i) => (
                  <li key={i} className="flex items-center justify-between text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
                    <span>
                      SM({p.sm_id})
                      {p.required_state != null && ` @ State ${p.required_state}`}
                    </span>
                    <button
                      onClick={() => {
                        const next = selectedRule.participants.filter((_, j) => j !== i);
                        updateInteractionRule(selectedRule.id, { participants: next });
                      }}
                      className="text-red-400 hover:text-red-300"
                      aria-label={`remove participant ${i}`}
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
                placeholder="SM ID"
                value={newSmId}
                onChange={(e) => setNewSmId(e.target.value)}
                className="w-16 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
                aria-label="participant SM ID"
              />
              <input
                type="number"
                placeholder="State (opt)"
                value={newRequiredState}
                onChange={(e) => setNewRequiredState(e.target.value)}
                className="w-20 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
                aria-label="required state"
              />
              <button
                onClick={() => {
                  const smId = parseInt(newSmId, 10);
                  if (isNaN(smId)) return;
                  const reqState = newRequiredState ? parseInt(newRequiredState, 10) : null;
                  updateInteractionRule(selectedRule.id, {
                    participants: [
                      ...selectedRule.participants,
                      { sm_id: smId, required_state: isNaN(reqState as number) ? null : reqState },
                    ],
                  });
                  setNewSmId("");
                  setNewRequiredState("");
                }}
                className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
              >
                Add Participant
              </button>
            </div>
          </section>

          {/* Conditions */}
          <section>
            <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Conditions</h4>
            {selectedRule.conditions.length === 0 ? (
              <p className="text-xs text-gray-600">No conditions</p>
            ) : (
              <ul className="space-y-1">
                {selectedRule.conditions.map((c, i) => (
                  <li key={i} className="flex items-center justify-between text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
                    <span>
                      {c.kind === "Spatial" ? `Spatial (radius: ${c.radius})` : `Guard`}
                    </span>
                    <button
                      onClick={() => {
                        const next = selectedRule.conditions.filter((_, j) => j !== i);
                        updateInteractionRule(selectedRule.id, { conditions: next });
                      }}
                      className="text-red-400 hover:text-red-300"
                      aria-label={`remove condition ${i}`}
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <div className="flex items-center gap-1 mt-1">
              <select
                value={newCondKind}
                onChange={(e) => setNewCondKind(e.target.value as "Spatial" | "Guard")}
                className="text-xs bg-gray-800 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
                aria-label="condition type"
              >
                <option value="Spatial">Spatial</option>
                <option value="Guard">Guard</option>
              </select>
              {newCondKind === "Spatial" && (
                <input
                  type="number"
                  placeholder="Radius"
                  value={newRadius}
                  onChange={(e) => setNewRadius(e.target.value)}
                  className="w-16 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
                  aria-label="spatial radius"
                />
              )}
              <button
                onClick={() => {
                  let cond: IRConditionSchema;
                  if (newCondKind === "Spatial") {
                    cond = { kind: "Spatial", radius: parseInt(newRadius, 10) || 10 };
                  } else {
                    cond = { kind: "Guard", expr: { Bool: true } };
                  }
                  updateInteractionRule(selectedRule.id, {
                    conditions: [...selectedRule.conditions, cond],
                  });
                }}
                className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
              >
                Add Condition
              </button>
            </div>
          </section>

          {/* Effects summary */}
          <section>
            <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Effects</h4>
            <p className="text-xs text-gray-600">
              {selectedRule.effects.length} effect(s) configured
            </p>
          </section>
        </div>
      )}
    </div>
  );
}
