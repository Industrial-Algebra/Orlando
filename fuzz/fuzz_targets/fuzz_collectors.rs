// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use libfuzzer_sys::fuzz_target;
use orlando::*;

fuzz_target!(|data: (Vec<i32>, u8)| {
    let (input, op) = data;
    
    // Only fuzz on reasonable input sizes
    if input.len() > 10000 {
        return;
    }
    
    let id = Identity::<i32>::new();
    
    // Test different collectors
    match op % 8 {
        0 => {
            // to_vec
            let result = to_vec(&id, input.clone());
            assert_eq!(result, input);
        }
        1 => {
            // sum
            let result = sum(&id, input.clone());
            let expected: i32 = input.iter().fold(0i32, |acc, &x| acc.saturating_add(x));
            assert_eq!(result, expected);
        }
        2 => {
            // count
            let result = count(&id, input.clone());
            assert_eq!(result, input.len());
        }
        3 => {
            // first
            let result = first(&id, input.clone());
            assert_eq!(result, input.first().copied());
        }
        4 => {
            // last
            let result = last(&id, input.clone());
            assert_eq!(result, input.last().copied());
        }
        5 => {
            // every
            let result = every(&id, input.clone(), |x| *x >= i32::MIN);
            assert_eq!(result, true);
        }
        6 => {
            // some
            let result = some(&id, input.clone(), |x| *x == 0);
            let expected = input.contains(&0);
            assert_eq!(result, expected);
        }
        _ => {
            // reduce
            let result = reduce(&id, input.clone(), 0i32, |acc, x| cont(acc.saturating_add(x)));
            let expected: i32 = input.iter().fold(0i32, |acc, &x| acc.saturating_add(x));
            assert_eq!(result, expected);
        }
    }
});
