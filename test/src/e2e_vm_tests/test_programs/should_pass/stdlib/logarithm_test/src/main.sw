script;

use std::{math::*, assert::assert};
use std::logging::log;

fn main() -> bool {
    assert(4.logarithm(2) == 2);
    assert(16.logarithm(2) == 4);
    assert(9.logarithm(3) == 2);
    assert(100.logarithm(10) == 2);
    assert(9.logarithm(2) == 3);

    log(1000.logarithm(10));
    // log(1000000.logarithm(10));
    // assert(1000.logarithm(10) == 2);
    // assert(1_000_000.logarithm(10) == 6);
    // assert(2.logarithm(1) == 0);
    true
}
