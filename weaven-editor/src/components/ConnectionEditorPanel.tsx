import { useEditorStore } from "../stores/editorStore";

export default function ConnectionEditorPanel() {
  const selectedConnectionId = useEditorStore((s) => s.selectedConnectionId);
  const schema = useEditorStore((s) => s.schema);
  const removeConnection = useEditorStore((s) => s.removeConnection);

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
        <p>Delay: {conn.delay_ticks} ticks</p>
      </div>

      {conn.pipeline.length > 0 && (
        <section>
          <h4 className="text-xs font-medium text-gray-400 uppercase mb-1">Pipeline</h4>
          <ul className="space-y-1">
            {conn.pipeline.map((step, i) => {
              const kind = Object.keys(step)[0];
              return (
                <li key={i} className="text-xs text-gray-300 px-2 py-1 bg-gray-800 rounded">
                  {kind}
                </li>
              );
            })}
          </ul>
        </section>
      )}
    </div>
  );
}
