#![no_main]

use libfuzzer_sys::fuzz_target;
use movable_tree::fuzz::{fuzz_tree, Action};

fuzz_target!(|actions: Vec<Action>| {
    // fuzzed code goes here
    fuzz_tree(5, &mut actions.clone())
});
