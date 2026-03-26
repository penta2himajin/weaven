import { describe, it, expect } from 'bun:test';
import type * as M from './models';
import * as helpers from './helpers';
import * as fix from './fixtures';

describe('property tests', () => {
  it('invariant GuardEvalPhase', () => {
    const guardEvaluateds: M.GuardEvaluated[] = [];
    expect([...guardEvaluateds].every(e => new Set([...PhaseEvaluate, ...PhasePropagate]).includes(e.phase))).toBe(true);
  });

  it('invariant IrMatchPhase', () => {
    const irMatcheds: M.IrMatched[] = [];
    expect([...irMatcheds].every(e => e.phase === PhaseEvaluate)).toBe(true);
  });

  it('invariant TransitionFiredPhase', () => {
    const transitionFireds: M.TransitionFired[] = [];
    expect([...transitionFireds].every(e => new Set([...PhaseExecute, ...PhasePropagate]).includes(e.phase))).toBe(true);
  });

  it('invariant SignalEmittedPhase', () => {
    const signalEmitteds: M.SignalEmitted[] = [];
    expect([...signalEmitteds].every(e => new Set([...PhaseExecute, ...PhasePropagate]).includes(e.phase))).toBe(true);
  });

  it('invariant CascadeStepPhase', () => {
    const cascadeSteps: M.CascadeStep[] = [];
    expect([...cascadeSteps].every(e => e.phase === PhasePropagate)).toBe(true);
  });

  it('invariant PipelineFilteredPhase', () => {
    const pipelineFiltereds: M.PipelineFiltered[] = [];
    expect([...pipelineFiltereds].every(e => e.phase === PhasePropagate)).toBe(true);
  });

  it('invariant CascadeDepthNonNeg', () => {
    const cascadeSteps: M.CascadeStep[] = [];
    expect([...cascadeSteps].every(e => e.depth >= 0)).toBe(true);
  });

  it('invariant CascadeQueueNonNeg', () => {
    const cascadeSteps: M.CascadeStep[] = [];
    expect([...cascadeSteps].every(e => e.queueSize >= 0)).toBe(true);
  });

  it('invariant NoSelfLoop', () => {
    const graphEdges: M.GraphEdge[] = [fix.defaultGraphEdge()];
    expect(![...graphEdges].some(e => JSON.stringify(e.edgeSource) === JSON.stringify(e.edgeTarget))).toBe(true);
  });

  it('invariant EdgesReferenceGraphNodes', () => {
    const topologyGraphs: M.TopologyGraph[] = [fix.defaultTopologyGraph()];
    expect([...topologyGraphs].every(g => [...g.edges].every(e => g.nodes.has(e.edgeSource) && g.nodes.has(e.edgeTarget)))).toBe(true);
  });

  it('invariant UniqueSmPerNode', () => {
    const topologyGraphs: M.TopologyGraph[] = [fix.defaultTopologyGraph()];
    expect([...topologyGraphs].every(g => [...g.nodes].every(n1 => [...g.nodes].every(n2 => n1 !== n2 ? n1.sm !== n2.sm : true)))).toBe(true);
  });

  it('invariant NoCyclicEvalTree', () => {
    const evalTreeNodes: M.EvalTreeNode[] = [fix.defaultEvalTreeNode()];
    expect(![...evalTreeNodes].some(n => helpers.tcChildren(n).includes(n))).toBe(true);
  });

  it('invariant CursorRange', () => {
    const tickCursors: M.TickCursor[] = [fix.defaultTickCursor()];
    expect([...tickCursors].every(c => c.current >= 0 && c.current <= c.maxTick)).toBe(true);
  });

  it('invariant MaxTickNonNeg', () => {
    const tickCursors: M.TickCursor[] = [fix.defaultTickCursor()];
    expect([...tickCursors].every(c => c.maxTick >= 0)).toBe(true);
  });

  it('invariant SnapshotNonEmpty', () => {
    const debugSessions: M.DebugSession[] = [fix.defaultDebugSession()];
    expect([...debugSessions].every(d => d.snapshots.length > 0)).toBe(true);
  });

  it('boundary SnapshotNonEmpty', () => {
    const debugSessions: M.DebugSession[] = [fix.boundaryDebugSession()];
    expect([...debugSessions].every(d => d.snapshots.length > 0)).toBe(true);
  });

  it('invalid SnapshotNonEmpty', () => {
    const debugSessions: M.DebugSession[] = [fix.invalidDebugSession()];
    expect(!([...debugSessions].every(d => d.snapshots.length > 0))).toBe(true);
  });

});
