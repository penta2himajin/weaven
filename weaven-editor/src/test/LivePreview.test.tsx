import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import LivePreview from "../components/LivePreview";
import type { WeavenAdapterLike } from "../components/LivePreview";
import { useEditorStore } from "../stores/editorStore";
import type { WeavenSchema } from "../generated/schema";

let tickCount = 0;

function createMockAdapter(): WeavenAdapterLike {
  tickCount = 0;
  return {
    loadSchema: vi.fn(),
    tick: vi.fn(() => {
      tickCount++;
      return [];
    }),
    tickN: vi.fn((n: number) => {
      tickCount += n;
      return [];
    }),
    activeState: vi.fn().mockReturnValue(0),
    snapshot: vi.fn().mockReturnValue('{"tick":0}'),
    restore: vi.fn(),
    get smIds() { return [1]; },
    get currentTick() { return tickCount; },
  };
}

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
  interaction_rules: [],
};

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    selectedInteractionRuleId: null,
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
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByRole("button", { name: /^tick$/i })).toBeInTheDocument();
  });

  it("calls adapter.tick when tick button is clicked", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);

    await user.click(screen.getByRole("button", { name: /^tick$/i }));
    expect(adapter.tick).toHaveBeenCalledOnce();
  });

  it("displays SM active states", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByText(/SM\(1\)/)).toBeInTheDocument();
    expect(screen.getByText(/State: 0/)).toBeInTheDocument();
  });

  it("displays current tick number", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByText(/Tick: 0/)).toBeInTheDocument();
  });

  it("shows Run/Stop button", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByRole("button", { name: /run/i })).toBeInTheDocument();
  });

  it("shows Tick xN button for batch ticking", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByRole("button", { name: /tick x/i })).toBeInTheDocument();
  });

  it("calls adapter.tickN when Tick xN button is clicked", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);

    await user.click(screen.getByRole("button", { name: /tick x/i }));
    expect(adapter.tickN).toHaveBeenCalledOnce();
  });

  it("shows tick rate input", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByLabelText("tick rate")).toBeInTheDocument();
  });

  it("shows Save Snapshot button", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(screen.getByRole("button", { name: /save snapshot/i })).toBeInTheDocument();
  });

  it("shows Restore Snapshot button (disabled initially)", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    const btn = screen.getByRole("button", { name: /restore snapshot/i });
    expect(btn).toBeDisabled();
  });

  it("Save Snapshot calls adapter.snapshot and enables Restore", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);

    await user.click(screen.getByRole("button", { name: /save snapshot/i }));
    expect(adapter.snapshot).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: /restore snapshot/i })).not.toBeDisabled();
  });

  it("Restore Snapshot calls adapter.restore", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);

    await user.click(screen.getByRole("button", { name: /save snapshot/i }));
    await user.click(screen.getByRole("button", { name: /restore snapshot/i }));
    expect(adapter.restore).toHaveBeenCalledOnce();
  });

  it("displays transitions when adapter returns them", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    (adapter.tick as ReturnType<typeof vi.fn>).mockReturnValue([
      { smId: 1, prev: 0, next: 1 },
    ]);
    render(<LivePreview adapter={adapter} />);

    await user.click(screen.getByRole("button", { name: /^tick$/i }));
    expect(screen.getByText(/0 → 1/)).toBeInTheDocument();
  });

  it("calls loadSchema on adapter when schema changes", () => {
    useEditorStore.getState().loadSchema(schema);
    const adapter = createMockAdapter();
    render(<LivePreview adapter={adapter} />);
    expect(adapter.loadSchema).toHaveBeenCalled();
  });
});
