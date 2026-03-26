import { useCallback } from "react";
import type { ExprSchema, BinOp } from "../generated/schema";

interface Props {
  expr: ExprSchema;
  onChange: (expr: ExprSchema) => void;
  depth?: number;
}

const BIN_OPS: BinOp[] = [
  "Add", "Sub", "Mul", "Div", "Mod",
  "Eq", "Neq", "Lt", "Gt", "Lte", "Gte",
  "And", "Or",
];

function exprKind(expr: ExprSchema): string {
  return Object.keys(expr)[0];
}

function opLabel(op: BinOp): string {
  const labels: Record<BinOp, string> = {
    Add: "+", Sub: "-", Mul: "*", Div: "/", Mod: "%",
    Eq: "==", Neq: "!=", Lt: "<", Gt: ">", Lte: "<=", Gte: ">=",
    And: "&&", Or: "||",
  };
  return labels[op];
}

function defaultExpr(): ExprSchema {
  return { Num: 0 };
}

export default function ExpressionBuilder({ expr, onChange, depth = 0 }: Props) {
  const kind = exprKind(expr);
  const maxDepth = 8;

  const handleKindChange = useCallback(
    (newKind: string) => {
      switch (newKind) {
        case "Num":
          onChange({ Num: 0 });
          break;
        case "Bool":
          onChange({ Bool: true });
          break;
        case "Str":
          onChange({ Str: "" });
          break;
        case "CtxField":
          onChange({ CtxField: "" });
          break;
        case "SigField":
          onChange({ SigField: "" });
          break;
        case "Not":
          onChange({ Not: defaultExpr() });
          break;
        case "BinOp":
          onChange({ BinOp: { op: "Add", left: defaultExpr(), right: defaultExpr() } });
          break;
        case "If":
          onChange({ If: { cond: { Bool: true }, then_: defaultExpr(), else_: defaultExpr() } });
          break;
        case "CollectionAny":
          onChange({ CollectionAny: { array_field: "", predicate: { Bool: true } } });
          break;
        case "CollectionCount":
          onChange({ CollectionCount: { array_field: "", predicate: { Bool: true } } });
          break;
        case "CollectionSum":
          onChange({ CollectionSum: { array_field: "", sum_field: "" } });
          break;
        case "TableLookup":
          onChange({ TableLookup: { table: "", keys: [] } });
          break;
      }
    },
    [onChange],
  );

  return (
    <div
      className="flex flex-col gap-1 pl-2 border-l border-gray-700"
      data-testid={`expr-node-${depth}`}
    >
      <div className="flex items-center gap-1">
        <select
          value={kind}
          onChange={(e) => handleKindChange(e.target.value)}
          className="text-xs bg-gray-800 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
          aria-label="expression type"
        >
          <option value="Num">Number</option>
          <option value="Bool">Boolean</option>
          <option value="Str">String</option>
          <option value="CtxField">Context Field</option>
          <option value="SigField">Signal Field</option>
          <option value="BinOp">Binary Op</option>
          <option value="Not">Not</option>
          <option value="If">If-Then-Else</option>
          <option value="CollectionAny">Collection Any</option>
          <option value="CollectionCount">Collection Count</option>
          <option value="CollectionSum">Collection Sum</option>
          <option value="TableLookup">Table Lookup</option>
        </select>
      </div>

      {/* Leaf editors */}
      {"Num" in expr && (
        <input
          type="number"
          value={expr.Num}
          onChange={(e) => onChange({ Num: parseFloat(e.target.value) || 0 })}
          className="w-20 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="number value"
        />
      )}

      {"Bool" in expr && (
        <select
          value={String(expr.Bool)}
          onChange={(e) => onChange({ Bool: e.target.value === "true" })}
          className="text-xs bg-gray-800 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
          aria-label="boolean value"
        >
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      )}

      {"Str" in expr && (
        <input
          type="text"
          value={expr.Str}
          onChange={(e) => onChange({ Str: e.target.value })}
          className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="string value"
        />
      )}

      {"CtxField" in expr && (
        <input
          type="text"
          value={expr.CtxField}
          onChange={(e) => onChange({ CtxField: e.target.value })}
          placeholder="field name"
          className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="context field"
        />
      )}

      {"SigField" in expr && (
        <input
          type="text"
          value={expr.SigField}
          onChange={(e) => onChange({ SigField: e.target.value })}
          placeholder="field name"
          className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="signal field"
        />
      )}

      {/* Compound editors */}
      {"BinOp" in expr && depth < maxDepth && (
        <div className="flex flex-col gap-1">
          <select
            value={expr.BinOp.op}
            onChange={(e) =>
              onChange({ BinOp: { ...expr.BinOp, op: e.target.value as BinOp } })
            }
            className="text-xs bg-gray-800 border border-gray-600 rounded text-gray-300 px-1 py-0.5"
            aria-label="operator"
          >
            {BIN_OPS.map((op) => (
              <option key={op} value={op}>
                {opLabel(op)} ({op})
              </option>
            ))}
          </select>
          <div className="text-xs text-gray-500">Left:</div>
          <ExpressionBuilder
            expr={expr.BinOp.left}
            onChange={(left) => onChange({ BinOp: { ...expr.BinOp, left } })}
            depth={depth + 1}
          />
          <div className="text-xs text-gray-500">Right:</div>
          <ExpressionBuilder
            expr={expr.BinOp.right}
            onChange={(right) => onChange({ BinOp: { ...expr.BinOp, right } })}
            depth={depth + 1}
          />
        </div>
      )}

      {"Not" in expr && depth < maxDepth && (
        <ExpressionBuilder
          expr={expr.Not}
          onChange={(inner) => onChange({ Not: inner })}
          depth={depth + 1}
        />
      )}

      {"If" in expr && depth < maxDepth && (
        <div className="flex flex-col gap-1">
          <div className="text-xs text-gray-500">Condition:</div>
          <ExpressionBuilder
            expr={expr.If.cond}
            onChange={(cond) => onChange({ If: { ...expr.If, cond } })}
            depth={depth + 1}
          />
          <div className="text-xs text-gray-500">Then:</div>
          <ExpressionBuilder
            expr={expr.If.then_}
            onChange={(then_) => onChange({ If: { ...expr.If, then_ } })}
            depth={depth + 1}
          />
          <div className="text-xs text-gray-500">Else:</div>
          <ExpressionBuilder
            expr={expr.If.else_}
            onChange={(else_) => onChange({ If: { ...expr.If, else_ } })}
            depth={depth + 1}
          />
        </div>
      )}

      {"CollectionAny" in expr && (
        <div className="flex flex-col gap-1">
          <input
            type="text"
            value={expr.CollectionAny.array_field}
            onChange={(e) =>
              onChange({ CollectionAny: { ...expr.CollectionAny, array_field: e.target.value } })
            }
            placeholder="array field"
            className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="array field"
          />
          {depth < maxDepth && (
            <>
              <div className="text-xs text-gray-500">Predicate:</div>
              <ExpressionBuilder
                expr={expr.CollectionAny.predicate}
                onChange={(predicate) =>
                  onChange({ CollectionAny: { ...expr.CollectionAny, predicate } })
                }
                depth={depth + 1}
              />
            </>
          )}
        </div>
      )}

      {"CollectionCount" in expr && (
        <div className="flex flex-col gap-1">
          <input
            type="text"
            value={expr.CollectionCount.array_field}
            onChange={(e) =>
              onChange({ CollectionCount: { ...expr.CollectionCount, array_field: e.target.value } })
            }
            placeholder="array field"
            className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="array field"
          />
          {depth < maxDepth && (
            <>
              <div className="text-xs text-gray-500">Predicate:</div>
              <ExpressionBuilder
                expr={expr.CollectionCount.predicate}
                onChange={(predicate) =>
                  onChange({ CollectionCount: { ...expr.CollectionCount, predicate } })
                }
                depth={depth + 1}
              />
            </>
          )}
        </div>
      )}

      {"CollectionSum" in expr && (
        <div className="flex flex-col gap-1">
          <input
            type="text"
            value={expr.CollectionSum.array_field}
            onChange={(e) =>
              onChange({ CollectionSum: { ...expr.CollectionSum, array_field: e.target.value } })
            }
            placeholder="array field"
            className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="array field"
          />
          <input
            type="text"
            value={expr.CollectionSum.sum_field}
            onChange={(e) =>
              onChange({ CollectionSum: { ...expr.CollectionSum, sum_field: e.target.value } })
            }
            placeholder="sum field"
            className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="sum field"
          />
        </div>
      )}

      {"TableLookup" in expr && (
        <div className="flex flex-col gap-1">
          <input
            type="text"
            value={expr.TableLookup.table}
            onChange={(e) =>
              onChange({ TableLookup: { ...expr.TableLookup, table: e.target.value } })
            }
            placeholder="table name"
            className="w-32 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
            aria-label="table name"
          />
        </div>
      )}
    </div>
  );
}

export { exprKind, opLabel, defaultExpr };
