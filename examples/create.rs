use movable_tree::{evan::EvanTree, MovableTree};

fn main() {
    let mut tree = MovableTree::<EvanTree>::new(0);
    let size = 10000;
    for _ in 0..size {
        tree.create(None);
    }
}
