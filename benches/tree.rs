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

    // b.bench_function("realtime tree move", |b| {
    //     let doc_a = LoroDoc::default();
    //     let doc_b = LoroDoc::default();
    //     let tree_a = doc_a.get_tree("tree");
    //     let tree_b = doc_b.get_tree("tree");
    //     let mut ids = vec![];
    //     let size = 1000;
    //     for _ in 0..size {
    //         ids.push(
    //             doc_a
    //                 .with_txn(|txn| tree_a.create_with_txn(txn, None))
    //                 .unwrap(),
    //         )
    //     }
    //     doc_b.import(&doc_a.export_snapshot()).unwrap();
    //     let mut rng: StdRng = rand::SeedableRng::seed_from_u64(0);
    //     let n = 1000;
    //     b.iter(|| {
    //         for t in 0..n {
    //             let i = rng.gen::<usize>() % size;
    //             let j = rng.gen::<usize>() % size;
    //             if t % 2 == 0 {
    //                 let mut txn = doc_a.txn().unwrap();
    //                 tree_a
    //                     .mov_with_txn(&mut txn, ids[i], ids[j])
    //                     .unwrap_or_default();
    //                 doc_b.import(&doc_a.export_from(&doc_b.oplog_vv())).unwrap();
    //             } else {
    //                 let mut txn = doc_b.txn().unwrap();
    //                 tree_b
    //                     .mov_with_txn(&mut txn, ids[i], ids[j])
    //                     .unwrap_or_default();
    //                 doc_a.import(&doc_b.export_from(&doc_a.oplog_vv())).unwrap();
    //             }
    //         }
    //     })
    // });
}

criterion_group!(benches, tree_move);
criterion_main!(benches);
