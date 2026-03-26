import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ConnectionEditorPanel from "../components/ConnectionEditorPanel";
import { useEditorStore } from "../stores/editorStore";
import type { WeavenSchema } from "../generated/schema";

const schema: WeavenSchema = {
  state_machines: [
    { id: 1, states: [0], initial_state: 0, transitions: [], input_ports: [], output_ports: [{ id: 1, kind: "Output", signal_type: 0 }] },
    { id: 2, states: [0], initial_state: 0, transitions: [], input_ports: [{ id: 0, kind: "Input", signal_type: 0 }], output_ports: [] },
  ],
  connections: [
    { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 2, pipeline: [] },
  ],
  named_tables: [],
};

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    dirty: false,
  });
}

describe("ConnectionEditorPanel", () => {
  beforeEach(resetStore);

  it("shows prompt when no connection selected", () => {
    render(<ConnectionEditorPanel />);
    expect(screen.getByText(/select a connection/i)).toBeInTheDocument();
  });

  it("shows connection details when selected", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    expect(screen.getByText("Connection(1)")).toBeInTheDocument();
    expect(screen.getByText(/SM\(1\) → SM\(2\)/)).toBeInTheDocument();
  });

  it("shows delay_ticks", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    expect(screen.getByText(/Delay: 2/)).toBeInTheDocument();
  });

  it("delete button removes the connection", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    await user.click(screen.getByRole("button", { name: /delete connection/i }));
    expect(useEditorStore.getState().schema.connections).toHaveLength(0);
  });
});
