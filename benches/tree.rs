use criterion::{criterion_group, criterion_main, Criterion};
use movable_tree::{evan::EvanTree, martin::MartinTree, MovableTree};
use rand::{rngs::StdRng, Rng};

pub fn tree_move(c: &mut Criterion) {
    let mut b = c.benchmark_group("tree create 10^4 node");
    b.sample_size(10);
    b.bench_function("evan", |b| {
        let size = 10000;
        let mut tree = MovableTree::<EvanTree>::new(0);
        b.iter(|| {
            for _ in 0..size {
                tree.create(None);
            }
        })
    });
    b.bench_function("martin", |b| {
        let size = 10000;
        let mut tree = MovableTree::<MartinTree>::new(0);
        b.iter(|| {
            for _ in 0..size {
                tree.create(None);
            }
        })
    });
    b.finish();

    let mut b = c.benchmark_group("10^3 tree move 10^4");
    b.sample_size(10);
    b.bench_function("evan", |b| {
        let mut tree = MovableTree::<EvanTree>::new(0);
        let mut ids = vec![];
        let size = 1000;
        for _ in 0..size {
            ids.push(tree.create(None));
        }
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        let n = 10000;
        b.iter(|| {
            for _ in 0..n {
                let i = rng.gen::<usize>() % size;
                let j = rng.gen::<usize>() % size;
                tree.mov(ids[i], ids[j]).unwrap_or_default();
            }
        })
    });

    b.bench_function("martin", |b| {
        let mut tree = MovableTree::<MartinTree>::new(0);
        let mut ids = vec![];
        let size = 1000;
        for _ in 0..size {
            ids.push(tree.create(None));
        }
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        let n = 10000;
        b.iter(|| {
            for _ in 0..n {
                let i = rng.gen::<usize>() % size;
                let j = rng.gen::<usize>() % size;
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
        let size = 1000;
        for _ in 0..size {
            ids.push(tree_a.create(None));
        }
        tree_b.merge(&tree_a);
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        let n = 100;
        b.iter(|| {
            for t in 0..n {
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
        let size = 100;
        for _ in 0..size {
            ids.push(tree_a.create(None));
        }
        tree_b.merge(&tree_a);
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
        let n = 1000;
        b.iter(|| {
            for t in 0..n {
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
