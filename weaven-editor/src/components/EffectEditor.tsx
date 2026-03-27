import type { EffectSchema, ExprSchema } from "../generated/schema";
import ExpressionBuilder from "./ExpressionBuilder";

interface Props {
  effect: EffectSchema;
  onChange: (effect: EffectSchema) => void;
  onRemove: () => void;
}

function effectKind(effect: EffectSchema): string {
  return Object.keys(effect)[0];
}

const EFFECT_KINDS = ["Signal", "HitStop", "SlowMotion", "TimeScale", "SetContext"] as const;

function defaultEffect(kind: string): EffectSchema {
  switch (kind) {
    case "Signal":
      return { Signal: { port: 0, payload: {} } };
    case "HitStop":
      return { HitStop: { frames: 3 } };
    case "SlowMotion":
      return { SlowMotion: { factor: 0.5, duration_ticks: 60 } };
    case "TimeScale":
      return { TimeScale: 1.0 };
    case "SetContext":
      return { SetContext: { field: "", expr: { Num: 0 } } };
    default:
      return { Signal: { port: 0, payload: {} } };
  }
}

export default function EffectEditor({ effect, onChange, onRemove }: Props) {
  const kind = effectKind(effect);

  return (
    <div className="flex flex-col gap-1 p-2 bg-gray-800 rounded border border-gray-700">
      <div className="flex items-center justify-between">
        <select
          value={kind}
          onChange={(e) => onChange(defaultEffect(e.target.value))}
          className="text-xs bg-gray-900 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
          aria-label="effect type"
        >
          {EFFECT_KINDS.map((k) => (
            <option key={k} value={k}>{k}</option>
          ))}
        </select>
        <button
          onClick={onRemove}
          className="text-red-400 hover:text-red-300 text-xs"
          aria-label="remove effect"
        >
          Remove
        </button>
      </div>

      {"Signal" in effect && (
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-1">
            <label className="text-xs text-gray-400">Port:</label>
            <input
              type="number"
              value={effect.Signal.port}
              onChange={(e) =>
                onChange({ Signal: { ...effect.Signal, port: parseInt(e.target.value, 10) || 0 } })
              }
              className="w-16 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
              aria-label="signal port"
            />
          </div>
          <div className="text-xs text-gray-400">Payload fields:</div>
          {Object.entries(effect.Signal.payload).map(([field, expr]) => (
            <div key={field} className="flex flex-col gap-1 pl-2">
              <div className="flex items-center gap-1">
                <span className="text-xs text-gray-300">{field}:</span>
                <button
                  onClick={() => {
                    const next = { ...effect.Signal.payload };
                    delete next[field];
                    onChange({ Signal: { ...effect.Signal, payload: next } });
                  }}
                  className="text-red-400 hover:text-red-300 text-xs"
                  aria-label={`remove payload field ${field}`}
                >
                  x
                </button>
              </div>
              <ExpressionBuilder
                expr={expr}
                onChange={(newExpr) =>
                  onChange({
                    Signal: {
                      ...effect.Signal,
                      payload: { ...effect.Signal.payload, [field]: newExpr },
                    },
                  })
                }
              />
            </div>
          ))}
          <div className="flex items-center gap-1">
            <input
              type="text"
              placeholder="field name"
              className="w-24 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
              aria-label="new payload field"
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  const name = (e.target as HTMLInputElement).value.trim();
                  if (name && !(name in effect.Signal.payload)) {
                    onChange({
                      Signal: {
                        ...effect.Signal,
                        payload: { ...effect.Signal.payload, [name]: { Num: 0 } as ExprSchema },
                      },
                    });
                    (e.target as HTMLInputElement).value = "";
                  }
                }
              }}
            />
            <span className="text-xs text-gray-500">Enter to add</span>
          </div>
        </div>
      )}

      {"HitStop" in effect && (
        <div className="flex items-center gap-1">
          <label className="text-xs text-gray-400">Frames:</label>
          <input
            type="number"
            min={1}
            value={effect.HitStop.frames}
            onChange={(e) =>
              onChange({ HitStop: { frames: parseInt(e.target.value, 10) || 1 } })
            }
            className="w-16 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
            aria-label="hitstop frames"
          />
        </div>
      )}

      {"SlowMotion" in effect && (
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            <label className="text-xs text-gray-400">Factor:</label>
            <input
              type="number"
              step={0.1}
              value={effect.SlowMotion.factor}
              onChange={(e) =>
                onChange({
                  SlowMotion: { ...effect.SlowMotion, factor: parseFloat(e.target.value) || 0.5 },
                })
              }
              className="w-16 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
              aria-label="slowmotion factor"
            />
          </div>
          <div className="flex items-center gap-1">
            <label className="text-xs text-gray-400">Duration:</label>
            <input
              type="number"
              min={1}
              value={effect.SlowMotion.duration_ticks}
              onChange={(e) =>
                onChange({
                  SlowMotion: {
                    ...effect.SlowMotion,
                    duration_ticks: parseInt(e.target.value, 10) || 1,
                  },
                })
              }
              className="w-16 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
              aria-label="slowmotion duration"
            />
          </div>
        </div>
      )}

      {"TimeScale" in effect && (
        <div className="flex items-center gap-1">
          <label className="text-xs text-gray-400">Scale:</label>
          <input
            type="number"
            step={0.1}
            value={effect.TimeScale}
            onChange={(e) =>
              onChange({ TimeScale: parseFloat(e.target.value) || 1.0 })
            }
            className="w-16 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
            aria-label="timescale value"
          />
        </div>
      )}

      {"SetContext" in effect && (
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-1">
            <label className="text-xs text-gray-400">Field:</label>
            <input
              type="text"
              value={effect.SetContext.field}
              onChange={(e) =>
                onChange({ SetContext: { ...effect.SetContext, field: e.target.value } })
              }
              placeholder="context field"
              className="w-24 px-1 py-0.5 text-xs bg-gray-900 border border-gray-600 rounded text-gray-200"
              aria-label="setcontext field"
            />
          </div>
          <div className="text-xs text-gray-400">Value:</div>
          <ExpressionBuilder
            expr={effect.SetContext.expr}
            onChange={(expr) =>
              onChange({ SetContext: { ...effect.SetContext, expr } })
            }
          />
        </div>
      )}
    </div>
  );
}

export { effectKind, defaultEffect, EFFECT_KINDS };
