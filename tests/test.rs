use movable_tree::{
    evan::{EvanTree, ROOT_ID},
    MovableTree,
};

#[test]
fn tree() {
    let mut tree = MovableTree::<EvanTree>::new(0);
    let child = tree.create(ROOT_ID);
    let child2 = tree.create(ROOT_ID);
    let mut tree2 = MovableTree::<EvanTree>::new(1);
    tree2.merge(&tree);
    tree.mov(child2, child);
    tree2.mov(child, child2);

    tree.merge(&tree2);
    tree2.merge(&tree);
    assert_eq!(tree.to_string(), tree2.to_string());
}
