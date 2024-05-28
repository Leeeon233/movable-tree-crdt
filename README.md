<h2 align="center">Movable Tree CRDTs</h3>

---

This repo implements two algorithms for Movable Tree CRDTs by Rust.

1. **_[A highly-available move operation for replicated trees](https://martin.kleppmann.com/2021/10/07/crdt-tree-move-operation.html)_** by M. Kleppmann, et al.
2. **_[CRDT: Mutable Tree Hierarchy](https://madebyevan.com/algos/crdt-mutable-tree-hierarchy/)_** by [Evan Wallace](https://madebyevan.com/)
   - This rust implementation is translated from the original JavaScript source code in the blog website.

### Correctness

A fuzzing test is built for making sure the correctness of the two implementations. Especially the consistency after synchronization.

### Benchmark

|                                | Kleppmann et al. | Evan      |
| ------------------------------ | ---------------- | --------- |
| create 10000 nodes             | 1.4 ms           | 2.0 ms    |
| 1000 nodes move 10000 times[1] | 9.9 ms           | 2795.6 ms |
| realtime move 10000 times[2]   | 16.1 ms          | 5691.5 ms |

- [1]: only benchmark the move operation
- [2]: two peers take turns to perform a move operation and then synchronize immediately

The current Benchmark is only used as a reference, which does not represent the performance of the real-world, because it may lack the necessary optimization.
