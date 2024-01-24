use criterion::{criterion_group, criterion_main, Criterion};
use movable_tree::{evan::EvanTree, martin::MartinTree, MovableTree};
use rand::{rngs::StdRng, Rng};

const CREATE_NODE_NUM: usize = 10000;
const MOVE_NODE_NUM: usize = 1000;
const MOVE_TIMES: usize = 10000;

pub fn tree_move(c: &mut Criterion) {
    let mut b = c.benchmark_group(format!("tree create {} nodes", CREATE_NODE_NUM));
    b.sample_size(10);
    b.bench_function("evan", |b| {
        let mut tree = MovableTree::<EvanTree>::new(0);
        b.iter(|| {
            for _ in 0..CREATE_NODE_NUM {
                tree.create(None);
            }
        })
    });
    b.bench_function("martin", |b| {
        let mut tree = MovableTree::<MartinTree>::new(0);
        b.iter(|| {
            for _ in 0..CREATE_NODE_NUM {
                tree.create(None);
            }
        })
    });
    b.finish();

    let mut b = c.benchmark_group(format!(
        "tree {} nodes move {} times",
        MOVE_NODE_NUM, MOVE_TIMES
    ));
    b.sample_size(10);
    b.bench_function("evan", |b| {
        let mut tree = MovableTree::<EvanTree>::new(0);
        let mut ids = vec![];
        for _ in 0..MOVE_NODE_NUM {
            ids.push(tree.create(None));
        }
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        b.iter(|| {
            for _ in 0..MOVE_TIMES {
                let i = rng.gen::<usize>() % MOVE_NODE_NUM;
                let j = rng.gen::<usize>() % MOVE_NODE_NUM;
                tree.mov(ids[i], ids[j]).unwrap_or_default();
            }
        })
    });

    b.bench_function("martin", |b| {
        let mut tree = MovableTree::<MartinTree>::new(0);
        let mut ids = vec![];
        for _ in 0..MOVE_NODE_NUM {
            ids.push(tree.create(None));
        }
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        b.iter(|| {
            for _ in 0..MOVE_TIMES {
                let i = rng.gen::<usize>() % MOVE_NODE_NUM;
                let j = rng.gen::<usize>() % MOVE_NODE_NUM;
                tree.mov(ids[i], ids[j]).unwrap_or_default();
            }
        })
    });

    b.finish();

    let mut b = c.benchmark_group("realtime tree move");
    b.sample_size(10);
    b.bench_function("evan", |b| {
        let mut tree_a = MovableTree::<EvanTree>::new(0);
        let mut tree_b = MovableTree::<EvanTree>::new(1);
        let mut ids = vec![];
        let size = MOVE_NODE_NUM;
        for _ in 0..size {
            ids.push(tree_a.create(None));
        }
        tree_b.merge(&tree_a);
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        b.iter(|| {
            for t in 0..MOVE_TIMES {
                let i = rng.gen::<usize>() % size;
                let j = rng.gen::<usize>() % size;
                if t % 2 == 0 {
                    tree_a.mov(ids[i], ids[j]).unwrap_or_default();
                    tree_b.merge(&tree_a);
                } else {
                    tree_b.mov(ids[i], ids[j]).unwrap_or_default();
                    tree_a.merge(&tree_b);
                }
            }
        })
    });

    b.bench_function("martin", |b| {
        let mut tree_a = MovableTree::<MartinTree>::new(0);
        let mut tree_b = MovableTree::<MartinTree>::new(1);
        let mut ids = vec![];
        let size = MOVE_NODE_NUM;
        for _ in 0..size {
            ids.push(tree_a.create(None));
        }
        tree_b.merge(&tree_a);
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        b.iter(|| {
            for t in 0..MOVE_TIMES {
                let i = rng.gen::<usize>() % size;
                let j = rng.gen::<usize>() % size;
                if t % 2 == 0 {
                    tree_a.mov(ids[i], ids[j]).unwrap_or_default();
                    tree_b.merge(&tree_a);
                } else {
                    tree_b.mov(ids[i], ids[j]).unwrap_or_default();
                    tree_a.merge(&tree_b);
                }
            }
        })
    });

    b.finish();
}

criterion_group!(benches, tree_move);
criterion_main!(benches);
