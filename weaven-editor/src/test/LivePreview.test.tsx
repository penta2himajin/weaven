import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import LivePreview from "../components/LivePreview";
import { useEditorStore } from "../stores/editorStore";
import type { WeavenSchema } from "../generated/schema";

// Mock WeavenAdapter from weaven-browser
const mockTick = vi.fn().mockReturnValue([]);
const mockLoadSchema = vi.fn();
const mockActiveState = vi.fn().mockReturnValue(0);
const mockDispose = vi.fn();
const mockSmIds = [1];

const mockAdapter = {
  init: vi.fn().mockResolvedValue(undefined),
  loadSchema: mockLoadSchema,
  tick: mockTick,
  activeState: mockActiveState,
  get smIds() { return mockSmIds; },
  get currentTick() { return 0; },
  dispose: mockDispose,
};

const schema: WeavenSchema = {
  state_machines: [
    {
      id: 1, states: [0, 1], initial_state: 0,
      transitions: [{ id: 10, source: 0, target: 1, priority: 10, guard: { BinOp: { op: "Gt", left: { CtxField: "trigger" }, right: { Num: 0.0 } } }, effects: [] }],
      input_ports: [],
      output_ports: [],
    },
  ],
  connections: [],
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

describe("LivePreview", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  it("shows loading state before adapter is ready", () => {
    render(<LivePreview adapter={null} />);
    expect(screen.getByText(/load a schema/i)).toBeInTheDocument();
  });

  it("shows tick button when adapter is provided", () => {
    useEditorStore.getState().loadSchema(schema);
    render(<LivePreview adapter={mockAdapter as any} />);
    expect(screen.getByRole("button", { name: /tick/i })).toBeInTheDocument();
  });

  it("calls adapter.tick when tick button is clicked", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    render(<LivePreview adapter={mockAdapter as any} />);

    await user.click(screen.getByRole("button", { name: /tick/i }));
    expect(mockTick).toHaveBeenCalledOnce();
  });

  it("displays SM active states", () => {
    useEditorStore.getState().loadSchema(schema);
    render(<LivePreview adapter={mockAdapter as any} />);
    expect(screen.getByText(/SM\(1\)/)).toBeInTheDocument();
    expect(screen.getByText(/State: 0/)).toBeInTheDocument();
  });

  it("displays current tick number", () => {
    useEditorStore.getState().loadSchema(schema);
    render(<LivePreview adapter={mockAdapter as any} />);
    expect(screen.getByText(/Tick: 0/)).toBeInTheDocument();
  });
});
