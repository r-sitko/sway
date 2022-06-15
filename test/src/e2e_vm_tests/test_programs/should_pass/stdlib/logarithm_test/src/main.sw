script;

use std::{math::*, assert::assert};

fn main() -> bool {
    assert(4.logarithm(2) == 2);
    assert(16.logarithm(2) == 4);
    assert(9.logarithm(3) == 2);
    assert(100.logarithm(10) == 2);
    assert(9.logarithm(2) == 3);

    true
}
