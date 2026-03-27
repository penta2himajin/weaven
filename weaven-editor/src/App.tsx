import { useCallback, useRef, useState, useEffect } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import TopologyCanvas from "./components/TopologyCanvas";
import SmEditorPanel from "./components/SmEditorPanel";
import ConnectionEditorPanel from "./components/ConnectionEditorPanel";
import IREditorPanel from "./components/IREditorPanel";
import NamedTablesPanel from "./components/NamedTablesPanel";
import LivePreview from "./components/LivePreview";
import type { WeavenAdapterLike } from "./components/LivePreview";
import { createWasmAdapter } from "./components/WasmAdapterBridge";
import { useEditorStore } from "./stores/editorStore";
import { parseSchema, validateSchema, serializeSchema } from "./schemaIo";

export default function App() {
  const addSm = useEditorStore((s) => s.addSm);
  const schema = useEditorStore((s) => s.schema);
  const loadSchema = useEditorStore((s) => s.loadSchema);
  const dirty = useEditorStore((s) => s.dirty);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [wasmAdapter, setWasmAdapter] = useState<WeavenAdapterLike | null>(null);

  useEffect(() => {
    createWasmAdapter().then(setWasmAdapter);
  }, []);

  const errors = validateSchema(schema);

  const handleExport = useCallback(() => {
    const json = serializeSchema(schema);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "weaven-schema.json";
    a.click();
    URL.revokeObjectURL(url);
  }, [schema]);

  const handleImport = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        const result = parseSchema(reader.result as string);
        if (result.ok) {
          loadSchema(result.value);
        }
      };
      reader.readAsText(file);
      e.target.value = "";
    },
    [loadSchema],
  );

  return (
    <div className="h-screen w-screen flex flex-col bg-gray-950 text-gray-100">
      {/* Toolbar */}
      <header className="flex items-center gap-3 px-4 py-2 bg-gray-900 border-b border-gray-800 shrink-0">
        <h1 className="text-sm font-bold mr-4">Weaven Editor</h1>
        <button
          onClick={addSm}
          className="px-3 py-1 text-xs rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium"
        >
          Add SM
        </button>
        <button
          onClick={handleExport}
          className="px-3 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
        >
          Export
        </button>
        <button
          onClick={handleImport}
          className="px-3 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300"
        >
          Import
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept=".json"
          className="hidden"
          onChange={handleFileChange}
        />
        {dirty && <span className="text-xs text-yellow-400 ml-auto">Unsaved changes</span>}
      </header>

      {/* Main layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Canvas */}
        <div className="flex-1">
          <ReactFlowProvider>
            <TopologyCanvas />
          </ReactFlowProvider>
        </div>

        {/* Right sidebar */}
        <aside className="w-80 border-l border-gray-800 overflow-y-auto flex flex-col">
          <SmEditorPanel />
          <div className="border-t border-gray-800">
            <ConnectionEditorPanel />
          </div>
          <div className="border-t border-gray-800">
            <IREditorPanel />
          </div>
          <div className="border-t border-gray-800">
            <NamedTablesPanel />
          </div>
          <div className="border-t border-gray-800">
            <LivePreview adapter={wasmAdapter} />
          </div>
          <div className="border-t border-gray-800 p-4">
            <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Validation</h4>
            {errors.length === 0 ? (
              <p className="text-xs text-green-400">No validation errors</p>
            ) : (
              <ul className="space-y-1">
                {errors.map((err, i) => (
                  <li key={i} className="text-xs text-red-400">{err}</li>
                ))}
              </ul>
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}
