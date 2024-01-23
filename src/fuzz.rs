use crate::{array_mut_ref, evan::EvanTree, martin::MartinTree, MovableTree};
use arbitrary::Arbitrary;
use enum_as_inner::EnumAsInner;

// struct Tree {
//     tree: HashMap<NodeID, Option<NodeID>>,
// }

// impl Tree {
//     fn new() -> Self {
//         let mut tree = HashMap::new();
//         tree.insert(ROOT_ID, None);
//         Self { tree }
//     }

//     fn create(&mut self, id: NodeID) {
//         self.tree.insert(id, Some(ROOT_ID));
//     }

//     fn mov(&mut self, target: NodeID, parent: NodeID) {
//         assert!(self.tree.contains_key(&target));
//         self.tree.insert(target, Some(parent));
//     }

//     fn to_string(&self) -> String {
//         TreeNode::from_state(&self.tree).to_string("".into(), true)
//     }
// }

#[derive(Debug, Clone, Copy, Arbitrary, EnumAsInner)]
pub enum Action {
    Create { site: u8, parent: u32 },
    Move { site: u8, target: u32, parent: u32 },
    Sync,
}

struct CRDTFuzzer {
    actors: Vec<Actor>,
}

impl CRDTFuzzer {
    fn new(site: usize) -> Self {
        let mut actors = Vec::new();
        for i in 0..site {
            actors.push(Actor::new(i as u64));
        }
        CRDTFuzzer { actors }
    }

    fn pre_process(&self, action: &mut Action) {
        match action {
            Action::Create { site, .. } => {
                *site = self.actors.len() as u8;
            }
            Action::Move { site, .. } => {
                *site = self.actors.len() as u8;
            }
            Action::Sync => {}
        }
        for actor in &self.actors {
            actor.pre_process(action);
        }
    }

    fn apply(&mut self, action: Action) {
        let site = match action {
            Action::Create { site, .. } => site % self.actors.len() as u8,
            Action::Move { site, .. } => site % self.actors.len() as u8,
            Action::Sync => {
                for i in 1..self.actors.len() {
                    let (a, b) = array_mut_ref!(&mut self.actors, [0, i]);
                    a.martin_tree.merge(&b.martin_tree);
                    a.evan_tree.merge(&b.evan_tree);
                }
                for i in 1..self.actors.len() {
                    let (a, b) = array_mut_ref!(&mut self.actors, [0, i]);
                    b.martin_tree.merge(&a.martin_tree);
                    b.evan_tree.merge(&a.evan_tree);
                }
                return;
            }
        };
        let actor = &mut self.actors[site as usize];
        actor.apply(action);
    }

    fn check_eq(&mut self) {
        for i in 0..self.actors.len() {
            for j in i + 1..self.actors.len() {
                let (a, b) = array_mut_ref!(&mut self.actors, [i, j]);
                a.martin_tree.merge(&b.martin_tree);
                a.evan_tree.merge(&b.evan_tree);
                b.martin_tree.merge(&a.martin_tree);
                b.evan_tree.merge(&a.evan_tree);
                assert_eq!(a.martin_tree.to_string(), b.martin_tree.to_string());
                assert_eq!(a.evan_tree.to_string(), b.evan_tree.to_string());
            }
        }
        println!("{}", self.actors[0].martin_tree.to_string());
    }
}

struct Actor {
    pub peer: u64,
    pub martin_tree: MovableTree<MartinTree>,
    pub evan_tree: MovableTree<EvanTree>,
}

impl Actor {
    fn new(peer: u64) -> Self {
        Actor {
            peer,
            martin_tree: MovableTree::new(peer),
            evan_tree: MovableTree::new(peer),
        }
    }

    fn pre_process(&self, action: &mut Action) {
        let tree_num = self.martin_tree.nodes().len();
        if tree_num < 2 {
            *action = Action::Create {
                site: self.peer as u8,
                parent: 0,
            };
            return;
        }
        match action {
            Action::Move {
                site: _,
                target,
                parent,
            } => {
                let target_idx = *target as usize % tree_num;
                let mut parent_idx = *parent as usize % tree_num;
                while target_idx == parent_idx {
                    parent_idx = (target_idx + 1) % tree_num;
                }
                *target = target_idx as u32;
                *parent = parent_idx as u32;
            }
            Action::Create { site: _, parent } => {
                let parent_idx = *parent as usize % tree_num;
                *parent = parent_idx as u32;
            }
            _ => {}
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Create { site: _, parent } => {
                let tree_num = self.martin_tree.nodes().len();
                let parent = if tree_num == 0 {
                    None
                } else {
                    Some(*self.martin_tree.nodes().get(parent as usize).unwrap())
                };
                self.martin_tree.create(parent);
                self.evan_tree.create(parent);
            }
            Action::Move {
                site: _,
                target,
                parent,
            } => {
                let target = *self.martin_tree.nodes().get(target as usize).unwrap();
                let parent = *self.martin_tree.nodes().get(parent as usize).unwrap();
                if self.martin_tree.mov(target, parent).is_err() {
                    return;
                };
                self.evan_tree.mov(target, parent).unwrap();
            }
            _ => {}
        }
    }
}

pub fn fuzz_tree(site: usize, actions: &mut [Action]) {
    let mut fuzzer = CRDTFuzzer::new(site);
    for action in actions {
        fuzzer.pre_process(action);
        fuzzer.apply(*action);
    }
    fuzzer.check_eq();
}

#[cfg(test)]
mod test {
    use super::*;
    use Action::*;
    #[test]
    fn create() {
        fuzz_tree(
            5,
            &mut [
                Create {
                    site: 51,
                    parent: 0,
                },
                Create {
                    site: 52,
                    parent: 0,
                },
                Sync,
                Create {
                    site: 217,
                    parent: 0,
                },
                Sync,
                Create { site: 0, parent: 0 },
            ],
        )
    }

    #[test]
    fn mov() {
        fuzz_tree(
            5,
            &mut [
                Sync,
                Sync,
                Sync,
                Create {
                    site: 255,
                    parent: 0,
                },
                Move {
                    site: 0,
                    target: 0,
                    parent: 0,
                },
            ],
        )
    }
}
