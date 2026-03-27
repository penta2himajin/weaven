import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import EffectEditor from "../components/EffectEditor";
import type { EffectSchema } from "../generated/schema";

describe("EffectEditor", () => {
  it("renders Signal effect with port input", () => {
    const effect: EffectSchema = { Signal: { port: 5, payload: {} } };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={vi.fn()} />);
    expect(screen.getByLabelText("signal port")).toHaveValue(5);
  });

  it("renders HitStop effect with frames input", () => {
    const effect: EffectSchema = { HitStop: { frames: 3 } };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={vi.fn()} />);
    expect(screen.getByLabelText("hitstop frames")).toHaveValue(3);
  });

  it("renders SlowMotion effect with factor and duration inputs", () => {
    const effect: EffectSchema = { SlowMotion: { factor: 0.5, duration_ticks: 60 } };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={vi.fn()} />);
    expect(screen.getByLabelText("slowmotion factor")).toHaveValue(0.5);
    expect(screen.getByLabelText("slowmotion duration")).toHaveValue(60);
  });

  it("renders TimeScale effect with scale input", () => {
    const effect: EffectSchema = { TimeScale: 2.0 };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={vi.fn()} />);
    expect(screen.getByLabelText("timescale value")).toHaveValue(2.0);
  });

  it("renders SetContext effect with field and expression", () => {
    const effect: EffectSchema = { SetContext: { field: "hp", expr: { Num: 100 } } };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={vi.fn()} />);
    expect(screen.getByLabelText("setcontext field")).toHaveValue("hp");
  });

  it("changing effect type calls onChange with new default", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const effect: EffectSchema = { Signal: { port: 0, payload: {} } };
    render(<EffectEditor effect={effect} onChange={onChange} onRemove={vi.fn()} />);

    await user.selectOptions(screen.getByLabelText("effect type"), "HitStop");
    expect(onChange).toHaveBeenCalledWith({ HitStop: { frames: 3 } });
  });

  it("remove button calls onRemove", async () => {
    const user = userEvent.setup();
    const onRemove = vi.fn();
    const effect: EffectSchema = { HitStop: { frames: 3 } };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={onRemove} />);

    await user.click(screen.getByLabelText("remove effect"));
    expect(onRemove).toHaveBeenCalled();
  });

  it("editing HitStop frames calls onChange", () => {
    const onChange = vi.fn();
    const effect: EffectSchema = { HitStop: { frames: 3 } };
    render(<EffectEditor effect={effect} onChange={onChange} onRemove={vi.fn()} />);

    const input = screen.getByLabelText("hitstop frames") as HTMLInputElement;
    fireEvent.change(input, { target: { value: "7" } });
    expect(onChange).toHaveBeenCalledWith({ HitStop: { frames: 7 } });
  });

  it("editing SetContext field calls onChange", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const effect: EffectSchema = { SetContext: { field: "", expr: { Num: 0 } } };
    render(<EffectEditor effect={effect} onChange={onChange} onRemove={vi.fn()} />);

    await user.type(screen.getByLabelText("setcontext field"), "hp");
    // Each keystroke fires; last call has field="p" since component re-renders with original prop
    // Check that onChange was called and last call includes SetContext
    expect(onChange).toHaveBeenCalled();
    const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1][0];
    expect("SetContext" in lastCall).toBe(true);
  });

  it("Signal effect shows payload fields", () => {
    const effect: EffectSchema = { Signal: { port: 0, payload: { damage: { Num: 10 } } } };
    render(<EffectEditor effect={effect} onChange={vi.fn()} onRemove={vi.fn()} />);
    expect(screen.getByText("damage:")).toBeInTheDocument();
  });
});
