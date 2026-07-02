// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use libfuzzer_sys::fuzz_target;
use orlando::*;

fuzz_target!(|data: (Vec<i32>, u8, u8)| {
    let (input, ops, n) = data;
    
    // Only fuzz on reasonable input sizes
    if input.len() > 10000 {
        return;
    }
    
    // Build a random pipeline based on the ops byte
    let result = match ops % 8 {
        0 => {
            // map + filter
            let pipeline = Map::new(|x: i32| x.saturating_mul(2))
                .compose(Filter::new(|x: &i32| *x % 2 == 0));
            to_vec(&pipeline, input)
        }
        1 => {
            // take
            let pipeline = Take::new(n as usize);
            to_vec(&pipeline, input)
        }
        2 => {
            // drop + map
            let pipeline = Drop::new(n as usize)
                .compose(Map::new(|x: i32| x.saturating_add(1)));
            to_vec(&pipeline, input)
        }
        3 => {
            // filter + take
            let pipeline = Filter::new(|x: &i32| *x > 0)
                .compose(Take::new(n as usize));
            to_vec(&pipeline, input)
        }
        4 => {
            // unique
            let pipeline = Unique::<i32>::new();
            to_vec(&pipeline, input)
        }
        5 => {
            // scan
            let pipeline = Scan::new(0, |acc: &i32, x: &i32| acc.saturating_add(*x));
            to_vec(&pipeline, input)
        }
        6 => {
            // complex pipeline
            let pipeline = Map::new(|x: i32| x.saturating_mul(2))
                .compose(Filter::new(|x: &i32| *x % 3 == 0))
                .compose(Take::new(n as usize))
                .compose(Map::new(|x: i32| x.saturating_add(1)));
            to_vec(&pipeline, input)
        }
        _ => {
            // takeWhile + dropWhile
            let pipeline = DropWhile::new(|x: &i32| *x < 0)
                .compose(TakeWhile::new(|x: &i32| *x < 100));
            to_vec(&pipeline, input)
        }
    };
    
    // Basic sanity checks
    assert!(result.len() <= input.len());
});
