use std::collections::HashMap;

use crate::{MovableTreeAlgorithm, NodeID, Op, TreeNode, TreeOp, ID};

pub const CREATE_ROOT_ID: ID = ID {
    lamport: 0,
    peer: 0,
};

#[derive(Debug)]
struct OpWrapper {
    op: crate::Op,
    old_parent: Option<NodeID>,
}

#[derive(Debug, Default)]
pub struct MartinTree {
    tree: HashMap<NodeID, Option<NodeID>>,
    sorted_ops: Vec<OpWrapper>,
    applied_end: usize,
}

impl MartinTree {
    fn mov(&mut self, target: NodeID, parent: NodeID) {
        assert!(self.tree.contains_key(&target));
        if self.is_ancestor_of(target, parent) {
            return;
        }
        self.tree.insert(target, Some(parent));
    }

    fn apply_pending_ops(&mut self) {
        for i in self.applied_end..self.sorted_ops.len() {
            let OpWrapper { op, old_parent } = &mut self.sorted_ops[i];
            match op.op {
                TreeOp::Create { parent } => {
                    self.tree.entry(parent).or_insert(None);
                    self.tree.insert(op.id.into(), Some(parent));
                }
                TreeOp::Move { target, parent } => {
                    *old_parent = self.tree.get(&target).copied().flatten();
                    self.mov(target, parent);
                }
            }
        }

        self.applied_end = self.sorted_ops.len();
    }

    fn revert_until(&mut self, id: &ID) -> Vec<Op> {
        let trim_start = match self.sorted_ops.binary_search_by_key(&id, |x| &x.op.id) {
            Ok(_) => unreachable!(),
            Err(i) => i,
        };
        let ans: Vec<OpWrapper> = self.sorted_ops.drain(trim_start..).collect();
        for op in ans.iter().rev() {
            match op.op.op {
                TreeOp::Create { .. } => {}
                TreeOp::Move { target, .. } => {
                    self.tree.insert(target, op.old_parent);
                }
            }
        }

        self.applied_end = self.sorted_ops.len();
        ans.into_iter().map(|x| x.op).collect()
    }

    fn get_parent(&self, tree_id: NodeID) -> Option<NodeID> {
        self.tree.get(&tree_id).copied().flatten()
    }
}

impl MovableTreeAlgorithm for MartinTree {
    fn new() -> Self {
        Self::default()
    }

    fn apply(&mut self, op: crate::Op) -> Option<NodeID> {
        let mut old_parent = None;
        let mut ans = None;
        match op.op {
            TreeOp::Create { parent } => {
                self.tree.entry(parent).or_insert(None);
                self.tree.insert(op.id.into(), Some(parent));
                ans = Some(op.id.into());
            }
            TreeOp::Move { target, parent } => {
                old_parent = self.tree.get(&target).copied().flatten();
                self.mov(target, parent);
            }
        };
        self.sorted_ops.push(OpWrapper { op, old_parent });
        self.applied_end = self.sorted_ops.len();
        ans
    }

    fn merge(&mut self, mut ops: Vec<crate::Op>) {
        if ops.is_empty() {
            return;
        }
        let start_id = ops.iter().min().unwrap();
        let mut popped = self.revert_until(&start_id.id);
        ops.append(&mut popped);
        ops.sort();
        for op in ops {
            self.sorted_ops.push(OpWrapper {
                op,
                old_parent: None,
            })
        }
        self.apply_pending_ops();
    }

    fn nodes(&self) -> Vec<NodeID> {
        self.tree.keys().copied().collect()
    }

    fn parent(&self, node: NodeID) -> Option<NodeID> {
        self.get_parent(node)
    }

    fn get_root(&self) -> crate::TreeNode {
        TreeNode::from_state(&self.tree)
    }
}
