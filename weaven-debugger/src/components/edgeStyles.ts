/**
 * Pure function: compute edge styles for React Flow based on highlighted edges.
 *
 * Decoupled from React Flow for testability.
 */

import type { HighlightedEdge } from "../stores/debugStore";

export interface EdgeStyle {
  stroke: string;
  strokeWidth: number;
  strokeDasharray?: string;
  animated?: boolean;
}

export type EdgeStyleMap = Record<string, EdgeStyle>;

interface MinimalEdge {
  id: string;
  source: string; // "sm-{id}"
  target: string; // "sm-{id}"
}

function extractSmId(nodeId: string): number {
  const match = nodeId.match(/^sm-(\d+)$/);
  return match ? parseInt(match[1], 10) : -1;
}

export function computeEdgeStyles(
  edges: MinimalEdge[],
  highlights: HighlightedEdge[],
): EdgeStyleMap {
  if (highlights.length === 0) return {};

  const map: EdgeStyleMap = {};

  for (const edge of edges) {
    const src = extractSmId(edge.source);
    const tgt = extractSmId(edge.target);

    for (const hl of highlights) {
      if (hl.kind === "signal" && hl.source === src && hl.target === tgt) {
        map[edge.id] = {
          stroke: "#22d3ee",
          strokeWidth: 4,
          animated: true,
        };
      } else if (hl.kind === "filtered") {
        // Filtered edges: highlight any edge pointing TO the blocked SM.
        if (hl.source === hl.target && tgt === hl.target) {
          map[edge.id] = {
            stroke: "#ef4444",
            strokeWidth: 3,
            strokeDasharray: "6 3",
          };
        }
      }
    }
  }

  return map;
}
