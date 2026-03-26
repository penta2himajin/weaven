import type * as M from './models';

/** Transitive closure traversal for EvalTreeNode.children. */
export function tcChildren(start: M.EvalTreeNode): M.EvalTreeNode[] {
  const result: M.EvalTreeNode[] = [];
  const queue: M.EvalTreeNode[] = [...start.children];
  while (queue.length > 0) {
    const next = queue.pop()!;
    if (!result.includes(next)) {
      result.push(next);
      queue.push(...next.children);
    }
  }
  return result;
}

