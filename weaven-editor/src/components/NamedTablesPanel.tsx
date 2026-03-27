import { useState } from "react";
import { useEditorStore } from "../stores/editorStore";

export default function NamedTablesPanel() {
  const schema = useEditorStore((s) => s.schema);
  const addNamedTable = useEditorStore((s) => s.addNamedTable);
  const removeNamedTable = useEditorStore((s) => s.removeNamedTable);
  const updateNamedTable = useEditorStore((s) => s.updateNamedTable);

  const [newTableName, setNewTableName] = useState("");
  const [selectedTable, setSelectedTable] = useState<string | null>(null);
  const [editJson, setEditJson] = useState("");
  const [jsonError, setJsonError] = useState<string | null>(null);

  const selected = selectedTable != null
    ? schema.named_tables.find((t) => t.name === selectedTable)
    : null;

  return (
    <div className="p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-100">Named Tables</h3>
      </div>

      <div className="flex items-center gap-1">
        <input
          type="text"
          placeholder="Table name"
          value={newTableName}
          onChange={(e) => setNewTableName(e.target.value)}
          className="flex-1 px-1 py-0.5 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200"
          aria-label="new table name"
        />
        <button
          onClick={() => {
            const name = newTableName.trim();
            if (name && !schema.named_tables.some((t) => t.name === name)) {
              addNamedTable(name);
              setNewTableName("");
            }
          }}
          className="px-2 py-0.5 text-xs rounded bg-indigo-600 hover:bg-indigo-500 text-white"
        >
          Add Table
        </button>
      </div>

      {schema.named_tables.length === 0 ? (
        <p className="text-xs text-gray-600">No named tables</p>
      ) : (
        <ul className="space-y-1">
          {schema.named_tables.map((table) => (
            <li
              key={table.name}
              onClick={() => {
                setSelectedTable(table.name);
                setEditJson(JSON.stringify(table.entries, null, 2));
                setJsonError(null);
              }}
              className={`text-xs px-2 py-1 rounded cursor-pointer flex items-center justify-between ${
                table.name === selectedTable
                  ? "bg-indigo-800 text-white"
                  : "bg-gray-800 text-gray-300 hover:bg-gray-700"
              }`}
            >
              <span>{table.name}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  removeNamedTable(table.name);
                  if (selectedTable === table.name) setSelectedTable(null);
                }}
                className="text-red-400 hover:text-red-300"
                aria-label={`remove table ${table.name}`}
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}

      {selected && (
        <div className="border-t border-gray-700 pt-3 flex flex-col gap-2">
          <h4 className="text-xs font-medium text-gray-100">Table: {selected.name}</h4>
          <textarea
            value={editJson}
            onChange={(e) => {
              setEditJson(e.target.value);
              setJsonError(null);
            }}
            className="w-full h-32 px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 font-mono"
            aria-label="table entries json"
          />
          {jsonError && <p className="text-xs text-red-400">{jsonError}</p>}
          <button
            onClick={() => {
              try {
                const parsed = JSON.parse(editJson);
                updateNamedTable(selected.name, parsed);
                setJsonError(null);
              } catch (e) {
                setJsonError(`Invalid JSON: ${e}`);
              }
            }}
            className="px-2 py-0.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 self-start"
          >
            Apply
          </button>
        </div>
      )}
    </div>
  );
}
