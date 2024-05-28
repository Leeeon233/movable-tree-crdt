use std::time::Instant;

use movable_tree::{evan::EvanTree, MovableTree};
use rand::{rngs::StdRng, Rng};

fn main() {
    let start = Instant::now();
    let mut tree = MovableTree::<EvanTree>::new(0);
    let mut ids = vec![];
    let size = 100;
    for _ in 0..size {
        let id = tree.create(None);
        ids.push(id);
    }
    let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
    for _ in 0..100000 {
        let i = rng.gen::<usize>() % size;
        let j = rng.gen::<usize>() % size;
        tree.mov(ids[i], ids[j]).unwrap_or_default();
    }
    println!("Elapsed: {:?}", start.elapsed());
}
