import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import InjectSignalForm from "../components/InjectSignalForm";
import { useDebugStore } from "../stores/debugStore";
import { CommandsProvider } from "../components/CommandsContext";
import { createCommands, type TauriInvoke } from "../commands";

beforeEach(() => {
  useDebugStore.setState({
    loaded: true,
    currentTick: 1,
    maxTick: 1,
    topology: null,
    traceEvents: [],
    selectedSmId: { inner: 1 },
    cascadeIndex: 0,
    selectedTraceIndex: null,
  });
});

function renderWithCommands(mockInvoke: TauriInvoke) {
  const cmds = createCommands(mockInvoke);
  return render(
    <CommandsProvider commands={cmds}>
      <InjectSignalForm />
    </CommandsProvider>,
  );
}

describe("InjectSignalForm", () => {
  it("pre-fills SM ID from selected SM", () => {
    const invoke = vi.fn();
    renderWithCommands(invoke);

    const smInput = screen.getByLabelText(/SM/i) as HTMLInputElement;
    expect(smInput.value).toBe("1");
  });

  it("calls inject_signal with entered values", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    renderWithCommands(invoke);

    const portInput = screen.getByLabelText(/Port/i) as HTMLInputElement;
    fireEvent.change(portInput, { target: { value: "5" } });

    const payloadInput = screen.getByLabelText(/Payload/i) as HTMLInputElement;
    fireEvent.change(payloadInput, { target: { value: '{"intensity": 3.0}' } });

    const injectBtn = screen.getByRole("button", { name: /inject/i });
    await fireEvent.click(injectBtn);

    expect(invoke).toHaveBeenCalledWith("inject_signal", {
      smId: 1,
      portId: 5,
      payload: { intensity: 3.0 },
    });
  });

  it("shows nothing when no SM selected", () => {
    useDebugStore.setState({ selectedSmId: null });
    const invoke = vi.fn();
    renderWithCommands(invoke);

    expect(screen.queryByRole("button", { name: /inject/i })).toBeNull();
  });
});
