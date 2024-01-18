use movable_tree::{
    evan::{EvanTree, ROOT_ID},
    martin::MartinTree,
    MovableTree,
};

#[test]
fn tree() {
    let mut tree = MovableTree::<EvanTree>::new(0);
    let child = tree.create(ROOT_ID);
    let child2 = tree.create(ROOT_ID);
    let mut tree2 = MovableTree::<EvanTree>::new(1);
    tree2.merge(&tree);
    tree.mov(child2, child).unwrap();
    tree2.mov(child, child2).unwrap();

    tree.merge(&tree2);
    tree2.merge(&tree);
    assert_eq!(tree.to_string(), tree2.to_string());
}

#[test]
fn tree2() {
    let mut tree = MovableTree::<MartinTree>::new(0);
    let child = tree.create(ROOT_ID);
    let child2 = tree.create(ROOT_ID);
    let mut tree2 = MovableTree::<MartinTree>::new(1);
    tree2.merge(&tree);
    tree.mov(child2, child).unwrap();
    tree2.mov(child, child2).unwrap();

    tree.merge(&tree2);
    tree2.merge(&tree);
    assert_eq!(tree.to_string(), tree2.to_string());
}
