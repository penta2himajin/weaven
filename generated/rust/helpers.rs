#[allow(unused_imports)]
use crate::models::*;

/// Transitive closure traversal for EvalTreeNode.children.
#[allow(dead_code)]
pub fn tc_children(start: &EvalTreeNode) -> Vec<EvalTreeNode> {
    let mut result = Vec::new();
    let mut queue: Vec<&EvalTreeNode> = start.children.iter().collect();
    while let Some(next) = queue.pop() {
        if !result.contains(next) {
            result.push(next.clone());
            queue.extend(next.children.iter());
        }
    }
    result
}

