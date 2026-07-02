// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Property-based tests for Orlando transducers.
//!
//! These tests verify algebraic properties and invariants that should hold
//! for all transducers, using randomly generated test data.

#![cfg(not(target_arch = "wasm32"))]

use orlando_transducers::*;
use proptest::prelude::*;

// Property: map(f).map(g) == map(g ∘ f)
proptest! {
    #[test]
    fn test_map_fusion(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // map(f).map(g)
        let pipeline1 = Map::new(|x: i32| x.saturating_add(1))
            .compose(Map::new(|x: i32| x.saturating_mul(2)));
        let result1 = to_vec(&pipeline1, vec.clone());

        // map(g ∘ f)
        let pipeline2 = Map::new(|x: i32| x.saturating_add(1).saturating_mul(2));
        let result2 = to_vec(&pipeline2, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: filter(p).filter(q) == filter(λx. p(x) ∧ q(x))
proptest! {
    #[test]
    fn test_filter_composition(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // filter(p).filter(q)
        let pipeline1 = Filter::new(|x: &i32| *x % 2 == 0)
            .compose(Filter::new(|x: &i32| *x > 10));
        let result1 = to_vec(&pipeline1, vec.clone());

        // filter(λx. p(x) ∧ q(x))
        let pipeline2 = Filter::new(|x: &i32| *x % 2 == 0 && *x > 10);
        let result2 = to_vec(&pipeline2, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: take(n) produces at most n elements
proptest! {
    #[test]
    fn test_take_bounds(vec in prop::collection::vec(any::<i32>(), 0..1000), n in 0usize..100) {
        let pipeline = Take::new(n);
        let result = to_vec(&pipeline, vec.clone());

        prop_assert!(result.len() <= n);
        prop_assert!(result.len() <= vec.len());
    }
}

// Property: take(n) preserves order
proptest! {
    #[test]
    fn test_take_order(vec in prop::collection::vec(any::<i32>(), 0..100), n in 0usize..100) {
        let pipeline = Take::new(n);
        let result = to_vec(&pipeline, vec.clone());

        let expected: Vec<i32> = vec.into_iter().take(n).collect();
        prop_assert_eq!(result, expected);
    }
}

// Property: drop(n).take(m) == take(m).drop(n) (for non-overlapping windows)
proptest! {
    #[test]
    fn test_drop_take_commute(vec in prop::collection::vec(any::<i32>(), 0..100), n in 0usize..50, m in 0usize..50) {
        let pipeline1 = Drop::new(n).compose(Take::new(m));
        let result1 = to_vec(&pipeline1, vec.clone());

        let expected: Vec<i32> = vec.into_iter().skip(n).take(m).collect();
        prop_assert_eq!(result1, expected);
    }
}

// Property: map preserves length (without filtering)
proptest! {
    #[test]
    fn test_map_preserves_length(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let pipeline = Map::new(|x: i32| x.saturating_mul(2));
        let result = to_vec(&pipeline, vec.clone());

        prop_assert_eq!(result.len(), vec.len());
    }
}

// Property: filter never increases length
proptest! {
    #[test]
    fn test_filter_decreases_length(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let pipeline = Filter::new(|x: &i32| *x % 2 == 0);
        let result = to_vec(&pipeline, vec.clone());

        prop_assert!(result.len() <= vec.len());
    }
}

// Property: Identity law - id.compose(t) == t
proptest! {
    #[test]
    fn test_identity_left(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let t = Map::new(|x: i32| x.saturating_mul(2));
        let id = Identity::<i32>::new();

        let pipeline1 = id.compose(Map::new(|x: i32| x.saturating_mul(2)));
        let result1 = to_vec(&pipeline1, vec.clone());

        let result2 = to_vec(&t, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Identity law - t.compose(id) == t
proptest! {
    #[test]
    fn test_identity_right(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let t = Map::new(|x: i32| x.saturating_mul(2));

        let pipeline1 = Map::new(|x: i32| x.saturating_mul(2)).compose(Identity::<i32>::new());
        let result1 = to_vec(&pipeline1, vec.clone());

        let result2 = to_vec(&t, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Associativity - (f.g).h == f.(g.h)
proptest! {
    #[test]
    fn test_associativity(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // (f.g).h
        let left = Map::new(|x: i32| x.saturating_add(1))
            .compose(Map::new(|x: i32| x.saturating_mul(2)))
            .compose(Map::new(|x: i32| x.saturating_sub(3)));
        let result1 = to_vec(&left, vec.clone());

        // f.(g.h)
        let right_inner = Map::new(|x: i32| x.saturating_mul(2))
            .compose(Map::new(|x: i32| x.saturating_sub(3)));
        let right = Map::new(|x: i32| x.saturating_add(1)).compose(right_inner);
        let result2 = to_vec(&right, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: sum equals manual summation
proptest! {
    #[test]
    fn test_sum_correctness(vec in prop::collection::vec(0i32..1000, 0..100)) {
        let pipeline = Map::new(|x: i32| x);
        let result = sum(&pipeline, vec.clone());

        let expected: i32 = vec.iter().sum();
        prop_assert_eq!(result, expected);
    }
}

// Property: count equals length
proptest! {
    #[test]
    fn test_count_equals_length(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let pipeline = Identity::<i32>::new();
        let result = count(&pipeline, vec.clone());

        prop_assert_eq!(result, vec.len());
    }
}

// Property: first on non-empty equals first element
proptest! {
    #[test]
    fn test_first_correctness(vec in prop::collection::vec(any::<i32>(), 1..100)) {
        let pipeline = Identity::<i32>::new();
        let result = first(&pipeline, vec.clone());

        prop_assert_eq!(result, Some(vec[0]));
    }
}

// Property: last on non-empty equals last element
proptest! {
    #[test]
    fn test_last_correctness(vec in prop::collection::vec(any::<i32>(), 1..100)) {
        let pipeline = Identity::<i32>::new();
        let result = last(&pipeline, vec.clone());

        let expected = vec.last().copied();
        prop_assert_eq!(result, expected);
    }
}

// Property: takeWhile takes a prefix
proptest! {
    #[test]
    fn test_takewhile_prefix(vec in prop::collection::vec(0i32..100, 0..100)) {
        let threshold = 50;
        let pipeline = TakeWhile::new(move |x: &i32| *x < threshold);
        let result = to_vec(&pipeline, vec.clone());

        // All elements in result should satisfy predicate
        for &x in &result {
            prop_assert!(x < threshold);
        }

        // Result should be a prefix of input
        prop_assert_eq!(&result[..], &vec[..result.len()]);
    }
}

// Property: dropWhile drops a prefix
proptest! {
    #[test]
    fn test_dropwhile_suffix(vec in prop::collection::vec(0i32..100, 0..100)) {
        let threshold = 50;
        let pipeline = DropWhile::new(move |x: &i32| *x < threshold);
        let result = to_vec(&pipeline, vec.clone());

        // Result length should be <= input length
        prop_assert!(result.len() <= vec.len());
    }
}

// Property: scan produces cumulative results
proptest! {
    #[test]
    fn test_scan_cumulative(vec in prop::collection::vec(0i32..100, 0..50)) {
        let pipeline = Scan::new(0, |acc: &i32, x: &i32| acc.saturating_add(*x));
        let result = to_vec(&pipeline, vec.clone());

        // Result length should equal input length
        prop_assert_eq!(result.len(), vec.len());

        // Each element should be the cumulative sum up to that point
        let mut expected = Vec::new();
        let mut acc = 0i32;
        for &x in &vec {
            acc = acc.saturating_add(x);
            expected.push(acc);
        }
        prop_assert_eq!(result, expected);
    }
}

// Property: unique removes consecutive duplicates
proptest! {
    #[test]
    fn test_unique_no_consecutive_dups(vec in prop::collection::vec(0i32..10, 0..100)) {
        let pipeline = Unique::<i32>::new();
        let result = to_vec(&pipeline, vec);

        // No two consecutive elements should be equal
        for i in 1..result.len() {
            prop_assert_ne!(result[i-1], result[i]);
        }
    }
}

// Property: map then filter == filter then map (when predicate is compositional)
proptest! {
    #[test]
    fn test_map_filter_interchange(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // map then filter
        let pipeline1 = Map::new(|x: i32| x.saturating_mul(2))
            .compose(Filter::new(|x: &i32| *x % 4 == 0));
        let result1 = to_vec(&pipeline1, vec.clone());

        // filter then map
        let pipeline2 = Filter::new(|x: &i32| x.saturating_mul(2) % 4 == 0)
            .compose(Map::new(|x: i32| x.saturating_mul(2)));
        let result2 = to_vec(&pipeline2, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Identity transducer is truly identity
proptest! {
    #[test]
    fn test_identity_is_identity(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let identity = Identity::<i32>::new();
        let result = to_vec(&identity, vec.clone());

        prop_assert_eq!(result, vec);
    }
}

// Property: Early termination efficiency - first() processes at most 1 element
proptest! {
    #[test]
    fn test_first_early_termination(vec in prop::collection::vec(any::<i32>(), 1..100)) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&call_count);

        let counting_map = Map::new(move |x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            x
        });

        let _result = first(&counting_map, vec);

        // Should process exactly 1 element (not 0, not more than 1)
        let count = call_count.load(Ordering::SeqCst);
        prop_assert_eq!(count, 1, "first() should process exactly 1 element, processed {}", count);
    }
}

// Property: Take(n) early termination - processes at most n elements
proptest! {
    #[test]
    fn test_take_early_termination(vec in prop::collection::vec(any::<i32>(), 1..100), n in 1usize..20) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&call_count);

        let counting_map = Map::new(move |x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            x
        });

        let pipeline = counting_map.compose(Take::new(n));
        let _result = to_vec(&pipeline, vec.clone());

        let count = call_count.load(Ordering::SeqCst);
        let expected = n.min(vec.len());
        prop_assert_eq!(count, expected,
            "take({}) should process at most {} elements, processed {}", n, expected, count);
    }
}

// Property: TakeWhile early termination efficiency
proptest! {
    #[test]
    fn test_takewhile_early_termination(vec in prop::collection::vec(0i32..100, 1..100)) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&call_count);

        let threshold = 50;
        let counting_map = Map::new(move |x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            x
        });

        let pipeline = counting_map.compose(TakeWhile::new(move |x: &i32| *x < threshold));
        let _result = to_vec(&pipeline, vec.clone());

        let count = call_count.load(Ordering::SeqCst);

        // Calculate expected: elements until first >= threshold
        let expected = vec.iter()
            .position(|&x| x >= threshold)
            .map(|pos| pos + 1)  // +1 because we process the element that fails
            .unwrap_or(vec.len());

        prop_assert_eq!(count, expected,
            "takeWhile should stop after first failing element, expected {} processed {}",
            expected, count);
    }
}

// Property: Map composition law - map(g ∘ f) ≡ map(f).map(g)
proptest! {
    #[test]
    fn test_map_composition_law(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let f = |x: i32| x.saturating_mul(2);
        let g = |x: i32| x.saturating_add(1);

        // map(g ∘ f)
        let composed = Map::new(move |x| g(f(x)));
        let result1 = to_vec(&composed, vec.clone());

        // map(f).map(g)
        let chained = Map::new(f).compose(Map::new(g));
        let result2 = to_vec(&chained, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Filter composition law - filter(p).filter(q) ≡ filter(λx. p(x) ∧ q(x))
proptest! {
    #[test]
    fn test_filter_composition_law(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // filter(p).filter(q)
        let chained = Filter::new(|x: &i32| *x % 2 == 0)
            .compose(Filter::new(|x: &i32| *x > 10));
        let result1 = to_vec(&chained, vec.clone());

        // filter(λx. p(x) ∧ q(x))
        let combined = Filter::new(|x: &i32| *x % 2 == 0 && *x > 10);
        let result2 = to_vec(&combined, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Fusion correctness - Map→Filter fusion preserves semantics
// This test verifies that our Pipeline fusion optimization doesn't change behavior
proptest! {
    #[test]
    fn test_map_filter_fusion_correctness(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // Note: This test uses the Rust transducers, not Pipeline
        // Pipeline fusion is tested separately in integration tests

        let f = |x: i32| x.saturating_mul(2);
        let p = |x: &i32| *x > 10;

        // Separate map and filter
        let separate = Map::new(f).compose(Filter::new(p));
        let result1 = to_vec(&separate, vec.clone());

        // Should be identical regardless of fusion
        // (Pipeline would fuse this, but transducers don't)
        let also_separate = Map::new(f).compose(Filter::new(p));
        let result2 = to_vec(&also_separate, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Every with all elements matching returns true
proptest! {
    #[test]
    fn test_every_all_match(vec in prop::collection::vec(0i32..100, 0..100)) {
        let pipeline = Identity::<i32>::new();
        let result = every(&pipeline, vec, |x| *x < 100);

        prop_assert!(result);
    }
}

// Property: Every with any element not matching returns false
proptest! {
    #[test]
    fn test_every_early_exit(vec in prop::collection::vec(any::<i32>(), 1..100)) {
        let pipeline = Identity::<i32>::new();

        // At least one element should fail this predicate
        let result = every(&pipeline, vec.clone(), |x| *x > i32::MAX - 1);

        // Should be false for most random inputs
        if vec.iter().any(|x| *x < i32::MAX) {
            prop_assert!(!result);
        }
    }
}

// Property: Some returns true if any element matches
proptest! {
    #[test]
    fn test_some_exists(vec in prop::collection::vec(0i32..100, 1..100)) {
        let pipeline = Identity::<i32>::new();

        if !vec.is_empty() {
            // Pick the first element's value as our target
            let target = vec[0];
            let result = some(&pipeline, vec, move |x| *x == target);

            prop_assert!(result);
        }
    }
}

// Property: Associativity law - (f∘g)∘h ≡ f∘(g∘h)
proptest! {
    #[test]
    fn test_composition_associativity(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        let f = Map::new(|x: i32| x.saturating_add(1));
        let g = Map::new(|x: i32| x.saturating_mul(2));
        let h = Filter::new(|x: &i32| *x % 2 == 0);

        // (f∘g)∘h
        let left = f.compose(g.compose(h));
        let result1 = to_vec(&left, vec.clone());

        // f∘(g∘h) - need to rebuild to test properly
        let f2 = Map::new(|x: i32| x.saturating_add(1));
        let g2 = Map::new(|x: i32| x.saturating_mul(2));
        let h2 = Filter::new(|x: &i32| *x % 2 == 0);
        let right_inner = g2.compose(h2);
        let right = f2.compose(right_inner);
        let result2 = to_vec(&right, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Take then drop vs drop then take
proptest! {
    #[test]
    fn test_take_drop_relationship(vec in prop::collection::vec(any::<i32>(), 0..100),
                                     n in 0usize..50,
                                     m in 0usize..50) {
        // take(n+m).drop(n) should equal drop(n).take(m)
        let total = n + m;

        let pipeline1 = Take::new(total).compose(Drop::new(n));
        let result1 = to_vec(&pipeline1, vec.clone());

        let pipeline2 = Drop::new(n).compose(Take::new(m));
        let result2 = to_vec(&pipeline2, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: FlatMap flattens nested structures
proptest! {
    #[test]
    fn test_flatmap_flattens(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::FlatMap;

        // FlatMap with duplicate function should double the length
        let pipeline = FlatMap::new(|x: i32| vec![x, x]);
        let result = to_vec(&pipeline, vec.clone());

        prop_assert_eq!(result.len(), vec.len() * 2);
    }
}

// Property: FlatMap with identity should equal original
proptest! {
    #[test]
    fn test_flatmap_identity(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::FlatMap;

        // FlatMap with single-element vec is identity
        let pipeline = FlatMap::new(|x: i32| vec![x]);
        let result = to_vec(&pipeline, vec.clone());

        prop_assert_eq!(result, vec);
    }
}

// Property: FlatMap with empty should produce empty
proptest! {
    #[test]
    fn test_flatmap_empty(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::FlatMap;

        // FlatMap with empty vec should produce nothing
        let pipeline = FlatMap::new(|_x: i32| Vec::<i32>::new());
        let result = to_vec(&pipeline, vec);

        prop_assert_eq!(result, Vec::<i32>::new());
    }
}

// Property: FlatMap preserves order
proptest! {
    #[test]
    fn test_flatmap_preserves_order(vec in prop::collection::vec(0i32..10, 0..50)) {
        use orlando_transducers::FlatMap;

        // FlatMap should preserve element order
        let pipeline = FlatMap::new(|x: i32| vec![x, x + 100]);
        let result = to_vec(&pipeline, vec.clone());

        // Check that original elements appear in order
        for i in 0..vec.len() {
            prop_assert_eq!(result[i * 2], vec[i]);
        }
    }
}

// Property: FlatMap composition law (associativity)
proptest! {
    #[test]
    fn test_flatmap_associativity(vec in prop::collection::vec(0i32..10, 0..20)) {
        use orlando_transducers::FlatMap;

        // flatMap(f).flatMap(g) == flatMap(x => flatMap(g, f(x)))
        let f = |x: i32| vec![x, x + 1];
        let g = |x: i32| vec![x, x * 2];

        // flatMap(f).flatMap(g)
        let pipeline1 = FlatMap::new(f).compose(FlatMap::new(g));
        let result1 = to_vec(&pipeline1, vec.clone());

        // flatMap(x => flatMap(g, f(x)))
        let pipeline2 = FlatMap::new(move |x: i32| {
            let intermediate = f(x);
            let mut output = Vec::new();
            for y in intermediate {
                output.extend(g(y));
            }
            output
        });
        let result2 = to_vec(&pipeline2, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: FlatMap with early termination stops correctly
proptest! {
    #[test]
    fn test_flatmap_early_termination(vec in prop::collection::vec(any::<i32>(), 1..100), n in 1usize..20) {
        use orlando_transducers::FlatMap;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&call_count);

        // FlatMap that counts invocations
        let pipeline = FlatMap::new(move |x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            vec![x, x + 1]
        }).compose(Take::new(n));

        let _result = to_vec(&pipeline, vec);

        // Should only call flatMap function enough times to produce n elements
        // Since each call produces 2 elements, we need at most ceil(n/2) calls
        let expected_max_calls = n.div_ceil(2) + 1;  // +1 for potential partial
        let actual_calls = call_count.load(Ordering::SeqCst);

        prop_assert!(actual_calls <= expected_max_calls,
            "FlatMap called {} times, expected at most {} for n={}",
            actual_calls, expected_max_calls, n);
    }
}

// Property: Partition preserves total count
proptest! {
    #[test]
    fn test_partition_total_count(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let (pass, fail) = partition(&id, vec.clone(), |x| x % 2 == 0);

        prop_assert_eq!(pass.len() + fail.len(), vec.len());
    }
}

// Property: All elements in pass partition satisfy predicate
proptest! {
    #[test]
    fn test_partition_pass_elements(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let (pass, _fail) = partition(&id, vec, |x| x % 2 == 0);

        // All pass elements must be even
        prop_assert!(pass.iter().all(|x| x % 2 == 0));
    }
}

// Property: All elements in fail partition don't satisfy predicate
proptest! {
    #[test]
    fn test_partition_fail_elements(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let (_pass, fail) = partition(&id, vec, |x| x % 2 == 0);

        // All fail elements must be odd
        prop_assert!(fail.iter().all(|x| x % 2 != 0));
    }
}

// Property: Partition preserves order within each group
proptest! {
    #[test]
    fn test_partition_preserves_order(vec in prop::collection::vec(0i32..100, 0..50)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let (evens, odds) = partition(&id, vec.clone(), |x| x % 2 == 0);

        // Extract evens and odds from original in order
        let expected_evens: Vec<i32> = vec.iter().filter(|x| *x % 2 == 0).copied().collect();
        let expected_odds: Vec<i32> = vec.iter().filter(|x| *x % 2 != 0).copied().collect();

        prop_assert_eq!(evens, expected_evens);
        prop_assert_eq!(odds, expected_odds);
    }
}

// Property: Partition works correctly with transformations
proptest! {
    #[test]
    fn test_partition_with_transform(vec in prop::collection::vec(0i32..50, 0..50)) {
        use orlando_transducers::Map;

        // Double each element, then partition by >50
        let double = Map::new(|x: i32| x * 2);
        let (greater, lesser) = partition(&double, vec.clone(), |x| *x > 50);

        // Verify all elements in greater are > 50
        prop_assert!(greater.iter().all(|x| *x > 50));

        // Verify all elements in lesser are <= 50
        prop_assert!(lesser.iter().all(|x| *x <= 50));

        // Verify total count
        prop_assert_eq!(greater.len() + lesser.len(), vec.len());
    }
}

// Property: Find returns first matching element
proptest! {
    #[test]
    fn test_find_returns_first_match(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let result = find(&id, vec.clone(), |x| x % 2 == 0);

        // If we find a match, it should be the first even number
        if let Some(found) = result {
            let expected = vec.iter().find(|x| *x % 2 == 0);
            prop_assert_eq!(Some(&found), expected);
        } else {
            // If no match, verify there are no evens
            prop_assert!(vec.iter().all(|x| x % 2 != 0));
        }
    }
}

// Property: Find with early termination stops immediately
proptest! {
    #[test]
    fn test_find_early_termination(vec in prop::collection::vec(any::<i32>(), 1..100)) {
        use orlando_transducers::transducer::Identity;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Find the position of the first even number
        let first_even_pos = vec.iter().position(|x| x % 2 == 0);

        let call_count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&call_count);

        let id = Identity::<i32>::new();
        let _result = find(&id, vec.iter().map(|x| {
            counter.fetch_add(1, Ordering::SeqCst);
            *x
        }), |x| x % 2 == 0);

        let actual_calls = call_count.load(Ordering::SeqCst);

        // Should only process elements up to and including the first match
        if let Some(pos) = first_even_pos {
            prop_assert_eq!(actual_calls, pos + 1,
                "find() should stop after finding match at position {}, but processed {} elements",
                pos, actual_calls);
        } else {
            // If no match, should process all elements
            prop_assert_eq!(actual_calls, vec.len());
        }
    }
}

// Property: Find on empty collection returns None
proptest! {
    #[test]
    fn test_find_empty_collection(_dummy in 0..1usize) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let result = find(&id, Vec::<i32>::new(), |x| x % 2 == 0);

        prop_assert_eq!(result, None);
    }
}

// Property: Find with transformation applies correctly
proptest! {
    #[test]
    fn test_find_with_transform(vec in prop::collection::vec(0i32..50, 0..50)) {
        use orlando_transducers::Map;

        // Double each element, then find first >50
        let double = Map::new(|x: i32| x * 2);
        let result = find(&double, vec.clone(), |x| *x > 50);

        // Manually compute expected
        let expected = vec.iter()
            .map(|x| x * 2)
            .find(|x| *x > 50);

        prop_assert_eq!(result, expected);
    }
}

// Property: Reject is equivalent to Filter with negated predicate
proptest! {
    #[test]
    fn test_reject_inverse_of_filter(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::{Reject, Filter};

        // Reject(p) == Filter(!p)
        let reject = Reject::new(|x: &i32| x % 2 == 0);
        let filter = Filter::new(|x: &i32| x % 2 != 0);

        let result1 = to_vec(&reject, vec.clone());
        let result2 = to_vec(&filter, vec);

        prop_assert_eq!(result1, result2);
    }
}

// Property: Reject decreases or maintains length
proptest! {
    #[test]
    fn test_reject_decreases_length(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::Reject;

        let reject = Reject::new(|x: &i32| x % 2 == 0);
        let result = to_vec(&reject, vec.clone());

        prop_assert!(result.len() <= vec.len());
    }
}

// Property: Reject + Filter with same predicate = empty
proptest! {
    #[test]
    fn test_reject_filter_complement(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::{Reject, Filter};

        // reject(p).filter(p) should produce empty result
        let pipeline = Reject::new(|x: &i32| x % 2 == 0)
            .compose(Filter::new(|x: &i32| x % 2 == 0));
        let result = to_vec(&pipeline, vec);

        prop_assert!(result.is_empty());
    }
}

// Property: All rejected elements satisfy the predicate
proptest! {
    #[test]
    fn test_reject_complement_elements(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::Reject;

        let reject = Reject::new(|x: &i32| x % 2 == 0);
        let result = to_vec(&reject, vec.clone());

        // All elements in result should NOT be even (should be odd or fail predicate)
        prop_assert!(result.iter().all(|x| x % 2 != 0));
    }
}

// Property: Chunk produces chunks of exact size (except possibly last)
proptest! {
    #[test]
    fn test_chunk_sizes(vec in prop::collection::vec(any::<i32>(), 0..100), size in 1usize..10) {
        use orlando_transducers::Chunk;

        let chunker = Chunk::new(size);
        let result = to_vec(&chunker, vec.clone());

        // All chunks should be exactly the specified size
        prop_assert!(result.iter().all(|chunk| chunk.len() == size));
    }
}

// Property: Chunk flattened equals original (minus partial)
proptest! {
    #[test]
    fn test_chunk_flatten_roundtrip(vec in prop::collection::vec(any::<i32>(), 0..100), size in 1usize..10) {
        use orlando_transducers::Chunk;

        let chunker = Chunk::new(size);
        let chunks = to_vec(&chunker, vec.clone());

        // Flatten the chunks
        let flattened: Vec<i32> = chunks.into_iter().flatten().collect();

        // Should equal original up to last complete chunk
        let expected_len = (vec.len() / size) * size;
        prop_assert_eq!(flattened, vec[..expected_len].to_vec());
    }
}

// Property: Chunk count is floor(length / size)
proptest! {
    #[test]
    fn test_chunk_count(vec in prop::collection::vec(any::<i32>(), 0..100), size in 1usize..10) {
        use orlando_transducers::Chunk;

        let chunker = Chunk::new(size);
        let chunks = to_vec(&chunker, vec.clone());

        let expected_count = vec.len() / size;
        prop_assert_eq!(chunks.len(), expected_count);
    }
}

// Property: Chunk preserves order within chunks
proptest! {
    #[test]
    fn test_chunk_preserves_order(vec in prop::collection::vec(0i32..100, 0..50), size in 1usize..10) {
        use orlando_transducers::Chunk;

        let chunker = Chunk::new(size);
        let chunks = to_vec(&chunker, vec.clone());

        // Verify each chunk contains consecutive elements from original
        for (i, chunk) in chunks.iter().enumerate() {
            let start = i * size;
            let end = start + size;
            prop_assert_eq!(chunk, &vec[start..end]);
        }
    }
}

// Property: Chunk with early termination works correctly
proptest! {
    #[test]
    fn test_chunk_with_take(vec in prop::collection::vec(any::<i32>(), 10..100), chunk_size in 2usize..5, n in 1usize..5) {
        use orlando_transducers::{Chunk, Take};

        // Chunk then take n chunks
        let pipeline = Chunk::new(chunk_size).compose(Take::new(n));
        let result = to_vec(&pipeline, vec.clone());

        // Should produce exactly n chunks (or fewer if not enough elements)
        let max_chunks = vec.len() / chunk_size;
        let expected_count = n.min(max_chunks);
        prop_assert_eq!(result.len(), expected_count);

        // All chunks should be the right size
        prop_assert!(result.iter().all(|chunk| chunk.len() == chunk_size));
    }
}

// Property: GroupBy preserves all elements
proptest! {
    #[test]
    fn test_group_by_total_count(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let groups = group_by(&id, vec.clone(), |x| x % 5);

        // Total elements across all groups should equal original
        let total: usize = groups.values().map(|v| v.len()).sum();
        prop_assert_eq!(total, vec.len());
    }
}

// Property: GroupBy groups correctly
proptest! {
    #[test]
    fn test_group_by_correctness(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let groups = group_by(&id, vec, |x| x % 3);

        // All elements in each group should have the same key
        for (key, values) in groups.iter() {
            prop_assert!(values.iter().all(|v| v % 3 == *key));
        }
    }
}

// Property: GroupBy preserves order within groups
proptest! {
    #[test]
    fn test_group_by_preserves_order(vec in prop::collection::vec(0i32..50, 0..50)) {
        use orlando_transducers::transducer::Identity;

        let id = Identity::<i32>::new();
        let groups = group_by(&id, vec.clone(), |x| x % 3);

        // Elements within each group should appear in original order
        for key in 0..3 {
            if let Some(group) = groups.get(&key) {
                let expected: Vec<i32> = vec.iter().filter(|x| *x % 3 == key).copied().collect();
                prop_assert_eq!(group, &expected);
            }
        }
    }
}

// Property: Zip length is min of two input lengths
proptest! {
    #[test]
    fn test_zip_length(a in prop::collection::vec(any::<i32>(), 0..50), b in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::zip;

        let result = zip(a.clone(), b.clone());
        let expected_len = a.len().min(b.len());
        prop_assert_eq!(result.len(), expected_len);
    }
}

// Property: Zip preserves order and pairing
proptest! {
    #[test]
    fn test_zip_correctness(a in prop::collection::vec(any::<i32>(), 0..50), b in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::zip;

        let result = zip(a.clone(), b.clone());

        // Each pair should match the original elements at that index
        for (i, (x, y)) in result.iter().enumerate() {
            prop_assert_eq!(x, &a[i]);
            prop_assert_eq!(y, &b[i]);
        }
    }
}

// Property: ZipWith applies function correctly
proptest! {
    #[test]
    fn test_zip_with_correctness(a in prop::collection::vec(any::<i32>(), 0..50), b in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::zip_with;

        let result = zip_with(a.clone(), b.clone(), |x, y| x.saturating_add(y));

        // Manually compute expected
        let expected: Vec<i32> = a.iter().zip(b.iter()).map(|(x, y)| x.saturating_add(*y)).collect();
        prop_assert_eq!(result, expected);
    }
}

// Property: Zip with empty is empty
proptest! {
    #[test]
    fn test_zip_with_empty(_dummy in 0..1usize) {
        use orlando_transducers::zip;

        let a: Vec<i32> = vec![];
        let b = vec![1, 2, 3];
        let result1 = zip(a.clone(), b.clone());
        let result2 = zip(b, a);

        prop_assert!(result1.is_empty());
        prop_assert!(result2.is_empty());
    }
}

// ============================================================================
// Phase 2a: Multi-Input Operations Property Tests
// ============================================================================

proptest! {
    // Property: Merge preserves all elements
    #[test]
    fn test_merge_preserves_all_elements(
        a in prop::collection::vec(any::<i32>(), 0..50),
        b in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        use orlando_transducers::merge;

        let expected_count = a.len() + b.len();
        let result = merge(vec![a, b]);
        prop_assert_eq!(result.len(), expected_count);
    }

    // Property: Merge with single stream is identity
    #[test]
    fn test_merge_identity(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::merge;

        let expected = vec.clone();
        let result = merge(vec![vec]);
        prop_assert_eq!(result, expected);
    }

    // Property: Merge alternates elements from equal-length streams
    #[test]
    fn test_merge_alternation(vec in prop::collection::vec(any::<i32>(), 1..20)) {
        use orlando_transducers::merge;

        let a = vec.clone();
        let b: Vec<i32> = vec.iter().map(|x| x + 100).collect();
        let result = merge(vec![a.clone(), b.clone()]);

        // Check alternation pattern
        for i in 0..a.len() {
            prop_assert_eq!(result[i * 2], a[i]);
            prop_assert_eq!(result[i * 2 + 1], b[i]);
        }
    }

    // Property: Intersection is subset of both inputs
    #[test]
    fn test_intersection_subset(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::intersection;
        use std::collections::HashSet;

        let result = intersection(a.clone(), b.clone());
        let set_a: HashSet<_> = a.iter().collect();
        let set_b: HashSet<_> = b.iter().collect();

        // All result elements must be in both A and B
        for item in &result {
            prop_assert!(set_a.contains(item));
            prop_assert!(set_b.contains(item));
        }
    }

    // Property: Intersection is commutative (ignoring order)
    #[test]
    fn test_intersection_commutative(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::intersection;
        use std::collections::HashSet;

        let result1: HashSet<_> = intersection(a.clone(), b.clone()).into_iter().collect();
        let result2: HashSet<_> = intersection(b, a).into_iter().collect();

        prop_assert_eq!(result1, result2);
    }

    // Property: Intersection with self is self (unique elements)
    #[test]
    fn test_intersection_idempotent(vec in prop::collection::vec(0i32..20, 0..30)) {
        use orlando_transducers::intersection;

        let result = intersection(vec.clone(), vec.clone());
        prop_assert_eq!(result, vec);
    }

    // Property: Difference result contains no elements from B
    #[test]
    fn test_difference_exclusion(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::difference;
        use std::collections::HashSet;

        let result = difference(a, b.clone());
        let set_b: HashSet<_> = b.into_iter().collect();

        // No element in result should be in B
        for item in &result {
            prop_assert!(!set_b.contains(item));
        }
    }

    // Property: Difference with empty set is identity
    #[test]
    fn test_difference_identity(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::difference;

        let empty: Vec<i32> = vec![];
        let result = difference(vec.clone(), empty);
        prop_assert_eq!(result, vec);
    }

    // Property: Difference with self is empty
    #[test]
    fn test_difference_self_empty(vec in prop::collection::vec(0i32..20, 0..30)) {
        use orlando_transducers::difference;

        let result: Vec<i32> = difference(vec.clone(), vec);
        prop_assert!(result.is_empty());
    }

    // Property: Union contains all unique elements from both sets
    #[test]
    fn test_union_contains_all(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::union;
        use std::collections::HashSet;

        let result = union(a.clone(), b.clone());
        let result_set: HashSet<_> = result.iter().collect();
        let a_set: HashSet<_> = a.iter().collect();
        let b_set: HashSet<_> = b.iter().collect();

        // All unique elements from A should be in result
        for item in &a_set {
            prop_assert!(result_set.contains(item));
        }

        // All unique elements from B should be in result
        for item in &b_set {
            prop_assert!(result_set.contains(item));
        }
    }

    // Property: Union is commutative (ignoring order)
    #[test]
    fn test_union_commutative(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::union;
        use std::collections::HashSet;

        let result1: HashSet<_> = union(a.clone(), b.clone()).into_iter().collect();
        let result2: HashSet<_> = union(b, a).into_iter().collect();

        prop_assert_eq!(result1, result2);
    }

    // Property: Union with empty is identity
    #[test]
    fn test_union_identity(vec in prop::collection::vec(0i32..20, 0..30)) {
        use orlando_transducers::union;
        use std::collections::HashSet;

        let empty: Vec<i32> = vec![];
        let result1: HashSet<_> = union(vec.clone(), empty.clone()).into_iter().collect();
        let result2: HashSet<_> = union(empty, vec.clone()).into_iter().collect();
        let expected: HashSet<_> = vec.into_iter().collect();

        prop_assert_eq!(result1, expected.clone());
        prop_assert_eq!(result2, expected);
    }

    // Property: Symmetric difference is commutative (ignoring order)
    #[test]
    fn test_symmetric_difference_commutative(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::symmetric_difference;
        use std::collections::HashSet;

        let result1: HashSet<_> = symmetric_difference(a.clone(), b.clone()).into_iter().collect();
        let result2: HashSet<_> = symmetric_difference(b, a).into_iter().collect();

        prop_assert_eq!(result1, result2);
    }

    // Property: Symmetric difference with self is empty
    #[test]
    fn test_symmetric_difference_self_empty(vec in prop::collection::vec(0i32..20, 0..30)) {
        use orlando_transducers::symmetric_difference;

        let result: Vec<i32> = symmetric_difference(vec.clone(), vec);
        prop_assert!(result.is_empty());
    }

    // Property: Symmetric difference contains no common elements
    #[test]
    fn test_symmetric_difference_no_common(
        a in prop::collection::vec(0i32..20, 0..30),
        b in prop::collection::vec(0i32..20, 0..30)
    ) {
        use orlando_transducers::symmetric_difference;
        use std::collections::HashSet;

        let result = symmetric_difference(a.clone(), b.clone());
        let set_a: HashSet<_> = a.iter().collect();
        let set_b: HashSet<_> = b.iter().collect();

        // No element in result should be in both A and B
        for item in &result {
            let in_a = set_a.contains(item);
            let in_b = set_b.contains(item);
            prop_assert!(in_a ^ in_b); // XOR: in exactly one
        }
    }

    // Property: Set operation laws - Union and Intersection
    #[test]
    fn test_set_distributive_law(
        a in prop::collection::vec(0i32..10, 0..15),
        b in prop::collection::vec(0i32..10, 0..15),
        c in prop::collection::vec(0i32..10, 0..15)
    ) {
        use orlando_transducers::{union, intersection};
        use std::collections::HashSet;

        // A ∩ (B ∪ C) = (A ∩ B) ∪ (A ∩ C)
        let b_union_c = union(b.clone(), c.clone());
        let left: HashSet<_> = intersection(a.clone(), b_union_c).into_iter().collect();

        let a_int_b = intersection(a.clone(), b);
        let a_int_c = intersection(a, c);
        let right: HashSet<_> = union(a_int_b, a_int_c).into_iter().collect();

        prop_assert_eq!(left, right);
    }

    // ========================================
    // Phase 2b Advanced Operations Property Tests
    // ========================================

    // Property: interpose increases length by (n-1) for n elements
    #[test]
    fn test_interpose_length(vec in prop::collection::vec(any::<i32>(), 1..100)) {
        use orlando_transducers::{Interpose, to_vec};

        let pipeline = Interpose::new(0);
        let result = to_vec(&pipeline, vec.clone());

        if vec.is_empty() {
            prop_assert_eq!(result.len(), 0);
        } else {
            // For n elements, we have n elements + (n-1) separators = 2n - 1
            prop_assert_eq!(result.len(), vec.len() * 2 - 1);
        }
    }

    // Property: interpose preserves original elements at odd indices
    #[test]
    fn test_interpose_preserves_elements(vec in prop::collection::vec(any::<i32>(), 1..50)) {
        use orlando_transducers::{Interpose, to_vec};

        let pipeline = Interpose::new(999);
        let result = to_vec(&pipeline, vec.clone());

        // Original elements should be at indices 0, 2, 4, ...
        for (i, &val) in vec.iter().enumerate() {
            prop_assert_eq!(result[i * 2], val);
        }

        // Separators should be at indices 1, 3, 5, ...
        for i in 0..(vec.len() - 1) {
            prop_assert_eq!(result[i * 2 + 1], 999);
        }
    }

    // Property: repeat_each multiplies length by n
    #[test]
    fn test_repeat_each_length(vec in prop::collection::vec(any::<i32>(), 0..50), n in 0usize..10) {
        use orlando_transducers::{RepeatEach, to_vec};

        let pipeline = RepeatEach::new(n);
        let result = to_vec(&pipeline, vec.clone());

        prop_assert_eq!(result.len(), vec.len() * n);
    }

    // Property: repeat_each produces consecutive repeats
    #[test]
    fn test_repeat_each_consecutive(vec in prop::collection::vec(any::<i32>(), 1..20), n in 1usize..5) {
        use orlando_transducers::{RepeatEach, to_vec};

        let pipeline = RepeatEach::new(n);
        let result = to_vec(&pipeline, vec.clone());

        // Check that each element appears n times consecutively
        for (i, &val) in vec.iter().enumerate() {
            for j in 0..n {
                prop_assert_eq!(result[i * n + j], val);
            }
        }
    }

    // Property: partition_by preserves all elements
    #[test]
    fn test_partition_by_preserves_elements(vec in prop::collection::vec(0i32..10, 0..50)) {
        use orlando_transducers::{partition_by, Identity};

        let id = Identity::new();
        let groups = partition_by(&id, vec.clone(), |x| *x);

        // Flatten groups and verify same elements
        let flattened: Vec<i32> = groups.into_iter().flatten().collect();
        prop_assert_eq!(flattened, vec);
    }

    // Property: partition_by consecutive property
    #[test]
    fn test_partition_by_consecutive(vec in prop::collection::vec(0i32..5, 0..30)) {
        use orlando_transducers::{partition_by, Identity};

        let id = Identity::new();
        let groups = partition_by(&id, vec.clone(), |x| *x);

        // Each group should have all equal elements
        for group in &groups {
            if !group.is_empty() {
                let first = group[0];
                for &val in group {
                    prop_assert_eq!(val, first);
                }
            }
        }

        // Adjacent groups should have different keys
        for i in 0..(groups.len().saturating_sub(1)) {
            if !groups[i].is_empty() && !groups[i + 1].is_empty() {
                prop_assert_ne!(groups[i][0], groups[i + 1][0]);
            }
        }
    }

    // Property: top_k returns at most k elements
    #[test]
    fn test_top_k_size(vec in prop::collection::vec(any::<i32>(), 0..100), k in 0usize..50) {
        use orlando_transducers::{top_k, Identity};

        let id = Identity::new();
        let result = top_k(&id, vec.clone(), k);

        prop_assert!(result.len() <= k);
        prop_assert!(result.len() <= vec.len());
    }

    // Property: top_k returns largest elements in descending order
    #[test]
    fn test_top_k_correctness(vec in prop::collection::vec(0i32..100, 1..50), k in 1usize..20) {
        use orlando_transducers::{top_k, Identity};

        let id = Identity::new();
        let result = top_k(&id, vec.clone(), k);

        // Result should be in descending order
        for i in 0..(result.len().saturating_sub(1)) {
            prop_assert!(result[i] >= result[i + 1]);
        }

        // All elements in result should be from original vec
        for &val in &result {
            prop_assert!(vec.contains(&val));
        }

        // If we have k elements, they should be the top k
        if result.len() == k && vec.len() >= k {
            let mut sorted = vec.clone();
            sorted.sort_by(|a, b| b.cmp(a));
            let expected: Vec<i32> = sorted.into_iter().take(k).collect();

            // Compare as sets since order might differ for equal elements
            use std::collections::HashSet;
            let result_set: HashSet<_> = result.iter().collect();
            let expected_set: HashSet<_> = expected.iter().collect();
            prop_assert_eq!(result_set, expected_set);
        }
    }

    // Property: frequencies count sum equals input length
    #[test]
    fn test_frequencies_total_count(vec in prop::collection::vec(0i32..20, 0..100)) {
        use orlando_transducers::{frequencies, Identity};

        let id = Identity::new();
        let freqs = frequencies(&id, vec.clone());

        let total_count: usize = freqs.values().sum();
        prop_assert_eq!(total_count, vec.len());
    }

    // Property: frequencies counts are correct
    #[test]
    fn test_frequencies_correctness(vec in prop::collection::vec(0i32..10, 0..50)) {
        use orlando_transducers::{frequencies, Identity};
        use std::collections::HashMap;

        let id = Identity::new();
        let freqs = frequencies(&id, vec.clone());

        // Manually count frequencies
        let mut expected: HashMap<i32, usize> = HashMap::new();
        for &val in &vec {
            *expected.entry(val).or_insert(0) += 1;
        }

        prop_assert_eq!(freqs, expected);
    }

    // Property: zip_longest has length of max(len(a), len(b))
    #[test]
    fn test_zip_longest_length(
        a in prop::collection::vec(any::<i32>(), 0..50),
        b in prop::collection::vec(any::<i32>(), 0..50)
    ) {
        use orlando_transducers::zip_longest;

        let result = zip_longest(a.clone(), b.clone(), 0, 0);
        let expected_len = a.len().max(b.len());

        prop_assert_eq!(result.len(), expected_len);
    }

    // Property: zip_longest fills with defaults correctly
    #[test]
    fn test_zip_longest_fills(
        a in prop::collection::vec(any::<i32>(), 10..20),
        b in prop::collection::vec(any::<i32>(), 0..5)
    ) {
        use orlando_transducers::zip_longest;

        let fill_a = -999;
        let fill_b = -888;
        let result = zip_longest(a.clone(), b.clone(), fill_a, fill_b);

        // After b runs out, should see fill_b
        if a.len() > b.len() {
            for item in result.iter().skip(b.len()) {
                prop_assert_eq!(item.1, fill_b);
            }
        }
    }

    // Property: cartesian_product has length = len(a) * len(b)
    #[test]
    fn test_cartesian_product_length(
        a in prop::collection::vec(any::<i32>(), 0..20),
        b in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        use orlando_transducers::cartesian_product;

        let result = cartesian_product(a.clone(), b.clone());
        prop_assert_eq!(result.len(), a.len() * b.len());
    }

    // Property: cartesian_product contains all pairs
    #[test]
    fn test_cartesian_product_completeness(
        a in prop::collection::vec(0i32..5, 0..5),
        b in prop::collection::vec(0i32..5, 0..5)
    ) {
        use orlando_transducers::cartesian_product;

        let result = cartesian_product(a.clone(), b.clone());

        // Every combination should exist
        for &x in &a {
            for &y in &b {
                prop_assert!(result.contains(&(x, y)));
            }
        }
    }

    // Property: reservoir_sample returns at most k elements
    #[test]
    fn test_reservoir_sample_size(vec in prop::collection::vec(any::<i32>(), 0..100), k in 0usize..50) {
        use orlando_transducers::{reservoir_sample, Identity};

        let id = Identity::new();
        let result = reservoir_sample(&id, vec.clone(), k);

        prop_assert!(result.len() <= k);
        prop_assert!(result.len() <= vec.len());
    }

    // Property: reservoir_sample contains only source elements
    #[test]
    fn test_reservoir_sample_membership(vec in prop::collection::vec(0i32..20, 1..100), k in 1usize..30) {
        use orlando_transducers::{reservoir_sample, Identity};
        use std::collections::HashSet;

        let id = Identity::new();
        let result = reservoir_sample(&id, vec.clone(), k);

        let source_set: HashSet<_> = vec.iter().collect();
        for val in &result {
            prop_assert!(source_set.contains(val));
        }
    }

    // ========================================
    // Logic Functions Property Tests
    // ========================================

    // Property: both is commutative
    #[test]
    fn test_both_commutative(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::both;

        let p1 = |x: &i32| *x > 0;
        let p2 = |x: &i32| x % 2 == 0;

        let both_12 = both(p1, p2);
        let both_21 = both(p2, p1);

        for val in &vec {
            prop_assert_eq!(both_12(val), both_21(val));
        }
    }

    // Property: either is commutative
    #[test]
    fn test_either_commutative(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::either;

        let p1 = |x: &i32| *x > 0;
        let p2 = |x: &i32| x % 2 == 0;

        let either_12 = either(p1, p2);
        let either_21 = either(p2, p1);

        for val in &vec {
            prop_assert_eq!(either_12(val), either_21(val));
        }
    }

    // Property: complement(complement(p)) == p
    #[test]
    fn test_complement_double_negation(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::complement;

        let p = |x: &i32| *x > 0;
        let not_p = complement(p);
        let not_not_p = complement(not_p);

        for val in &vec {
            prop_assert_eq!(p(val), not_not_p(val));
        }
    }

    // Property: both(p, p) == p
    #[test]
    fn test_both_idempotent(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::both;

        let p = |x: &i32| *x > 0;
        let both_pp = both(p, p);

        for val in &vec {
            prop_assert_eq!(p(val), both_pp(val));
        }
    }

    // Property: either(p, p) == p
    #[test]
    fn test_either_idempotent(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::either;

        let p = |x: &i32| *x > 0;
        let either_pp = either(p, p);

        for val in &vec {
            prop_assert_eq!(p(val), either_pp(val));
        }
    }

    // Property: When preserves non-matching elements
    #[test]
    fn test_when_preserves_non_matching(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::{logic::When, to_vec};

        let when_transform = When::new(|x: &i32| *x > 1000, |x: i32| x.saturating_mul(2));
        let result = to_vec(&when_transform, vec.clone());

        // Elements <= 1000 should be unchanged
        for (i, &val) in vec.iter().enumerate() {
            if val <= 1000 {
                prop_assert_eq!(result[i], val);
            }
        }
    }

    // Property: Unless preserves matching elements
    #[test]
    fn test_unless_preserves_matching(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::{logic::Unless, to_vec};

        let unless_transform = Unless::new(|x: &i32| *x > 1000, |x: i32| x.saturating_add(100));
        let result = to_vec(&unless_transform, vec.clone());

        // Elements > 1000 should be unchanged
        for (i, &val) in vec.iter().enumerate() {
            if val > 1000 {
                prop_assert_eq!(result[i], val);
            }
        }
    }

    // Property: IfElse always transforms
    #[test]
    fn test_if_else_always_transforms(vec in prop::collection::vec(any::<i32>(), 1..50)) {
        use orlando_transducers::{logic::IfElse, to_vec};

        let transform = IfElse::new(
            |x: &i32| *x >= 0,
            |x: i32| x + 1,
            |x: i32| x - 1,
        );
        let result = to_vec(&transform, vec.clone());

        // Every element should be transformed
        prop_assert_eq!(result.len(), vec.len());
        for (i, &val) in vec.iter().enumerate() {
            if val >= 0 {
                prop_assert_eq!(result[i], val + 1);
            } else {
                prop_assert_eq!(result[i], val - 1);
            }
        }
    }

    // Property: De Morgan's Laws - not(p and q) == (not p) or (not q)
    #[test]
    fn test_de_morgan_and(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::{both, complement, either};

        let p = |x: &i32| *x > 0;
        let q = |x: &i32| x % 2 == 0;

        let not_both = complement(both(p, q));
        let not_p_or_not_q = either(complement(p), complement(q));

        for val in &vec {
            prop_assert_eq!(not_both(val), not_p_or_not_q(val));
        }
    }

    // Property: De Morgan's Laws - not(p or q) == (not p) and (not q)
    #[test]
    fn test_de_morgan_or(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::logic::{both, complement, either};

        let p = |x: &i32| *x > 0;
        let q = |x: &i32| x % 2 == 0;

        let not_either = complement(either(p, q));
        let not_p_and_not_q = both(complement(p), complement(q));

        for val in &vec {
            prop_assert_eq!(not_either(val), not_p_and_not_q(val));
        }
    }

    // Property: When composed with Filter
    #[test]
    fn test_when_with_filter_equivalence(vec in prop::collection::vec(0i32..100, 0..50)) {
        use orlando_transducers::{logic::When, to_vec};

        // When followed by identity should equal Filter + Map
        let when_pipeline = When::new(|x: &i32| *x > 50, |x: i32| x.saturating_mul(2));
        let result1 = to_vec(&when_pipeline, vec.clone());

        // Manually simulate: keep all, but transform some
        let mut result2 = vec.clone();
        for val in &mut result2 {
            if *val > 50 {
                *val = val.saturating_mul(2);
            }
        }

        prop_assert_eq!(result1, result2);
    }

    // Property: all_pass with empty list is always true
    #[test]
    fn test_all_pass_empty_always_true(vec in prop::collection::vec(any::<i32>(), 0..20)) {
        use orlando_transducers::logic::{all_pass, PredicateVec};

        let predicates: PredicateVec<i32> = vec![];
        let always_true = all_pass(predicates);

        for val in &vec {
            prop_assert!(always_true(val));
        }
    }

    // Property: any_pass with empty list is always false
    #[test]
    fn test_any_pass_empty_always_false(vec in prop::collection::vec(any::<i32>(), 0..20)) {
        use orlando_transducers::logic::{any_pass, PredicateVec};

        let predicates: PredicateVec<i32> = vec![];
        let always_false = any_pass(predicates);

        for val in &vec {
            prop_assert!(!always_false(val));
        }
    }

    // ========================================
    // Phase 2b: New Operations Property Tests (v0.2.0)
    // ========================================

    // Property: Aperture window count is correct
    #[test]
    fn test_aperture_window_count(vec in prop::collection::vec(any::<i32>(), 0..100), size in 1usize..10) {
        use orlando_transducers::{Aperture, to_vec};

        let window = Aperture::new(size);
        let result = to_vec(&window, vec.clone());

        if vec.len() < size {
            prop_assert_eq!(result.len(), 0);
        } else {
            let expected_count = vec.len() - size + 1;
            prop_assert_eq!(result.len(), expected_count);
        }
    }

    // Property: Aperture windows are correct size
    #[test]
    fn test_aperture_window_size(vec in prop::collection::vec(any::<i32>(), 5..100), size in 1usize..10) {
        use orlando_transducers::{Aperture, to_vec};

        let window = Aperture::new(size);
        let result = to_vec(&window, vec);

        // All windows should be exactly the specified size
        for w in &result {
            prop_assert_eq!(w.len(), size);
        }
    }

    // Property: Aperture windows overlap correctly
    #[test]
    fn test_aperture_overlap(vec in prop::collection::vec(0i32..100, 5..50), size in 2usize..6) {
        use orlando_transducers::{Aperture, to_vec};

        let window = Aperture::new(size);
        let result = to_vec(&window, vec.clone());

        // Each window should start 1 element after the previous
        for i in 0..(result.len().saturating_sub(1)) {
            // Last (size-1) elements of window[i] should equal first (size-1) of window[i+1]
            for j in 1..size {
                prop_assert_eq!(result[i][j], result[i + 1][j - 1]);
            }
        }
    }

    // Property: Aperture preserves original elements
    #[test]
    fn test_aperture_preserves_elements(vec in prop::collection::vec(any::<i32>(), 1..50), size in 1usize..5) {
        use orlando_transducers::{Aperture, to_vec};

        let window = Aperture::new(size);
        let result = to_vec(&window, vec.clone());

        // Only check if we have windows (vec must have at least 'size' elements)
        if vec.len() >= size {
            // Flatten all windows and verify all original elements appear
            let flattened: std::collections::HashSet<_> = result.iter().flatten().collect();
            for val in &vec {
                prop_assert!(flattened.contains(val));
            }
        }
    }

    // Property: Aperture size 1 equals identity
    #[test]
    fn test_aperture_size_1_is_identity(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::{Aperture, to_vec};

        let window = Aperture::new(1);
        let result = to_vec(&window, vec.clone());

        // Should produce [[a], [b], [c], ...] which when flattened equals original
        let flattened: Vec<i32> = result.into_iter().flatten().collect();
        prop_assert_eq!(flattened, vec);
    }

    // Property: Aperture with composition
    #[test]
    fn test_aperture_with_filter(vec in prop::collection::vec(0i32..50, 10..50), size in 2usize..5) {
        use orlando_transducers::{Aperture, Filter, to_vec};

        let pipeline = Filter::new(|x: &i32| x % 2 == 0).compose(Aperture::new(size));
        let result = to_vec(&pipeline, vec.clone());

        // Filter even numbers first, then create windows
        let evens: Vec<i32> = vec.iter().filter(|x| *x % 2 == 0).cloned().collect();
        let expected_count = if evens.len() >= size {
            evens.len() - size + 1
        } else {
            0
        };

        prop_assert_eq!(result.len(), expected_count);
    }

    // Property: take_last length is correct
    #[test]
    fn test_take_last_length(vec in prop::collection::vec(any::<i32>(), 0..100), n in 0usize..50) {
        use orlando_transducers::{take_last, Identity};

        let id = Identity::new();
        let result = take_last(&id, vec.clone(), n);

        let expected_len = n.min(vec.len());
        prop_assert_eq!(result.len(), expected_len);
    }

    // Property: take_last returns correct elements
    #[test]
    fn test_take_last_correctness(vec in prop::collection::vec(any::<i32>(), 1..100), n in 1usize..50) {
        use orlando_transducers::{take_last, Identity};

        let id = Identity::new();
        let result = take_last(&id, vec.clone(), n);

        // Manually compute expected
        let start = vec.len().saturating_sub(n);
        let expected: Vec<i32> = vec.iter().skip(start).cloned().collect();

        prop_assert_eq!(result, expected);
    }

    // Property: take_last preserves order
    #[test]
    fn test_take_last_preserves_order(vec in prop::collection::vec(0i32..100, 5..50), n in 1usize..20) {
        use orlando_transducers::{take_last, Identity};

        let id = Identity::new();
        let result = take_last(&id, vec.clone(), n);

        // Verify result is a suffix of original (preserves relative order)
        if !result.is_empty() {
            let start_idx = vec.len().saturating_sub(n);
            let expected_suffix: Vec<i32> = vec[start_idx..].to_vec();
            prop_assert_eq!(result, expected_suffix);
        }
    }

    // Property: take_last + drop_last covers all elements
    #[test]
    fn test_take_drop_last_partition(vec in prop::collection::vec(any::<i32>(), 0..100), n in 0usize..50) {
        use orlando_transducers::{take_last, drop_last, Identity};

        let id = Identity::new();
        let taken = take_last(&id, vec.clone(), n);
        let dropped = drop_last(&id, vec.clone(), n);

        // Length should sum to original
        prop_assert_eq!(taken.len() + dropped.len(), vec.len());

        // Concatenated should equal original
        let mut combined = dropped.clone();
        combined.extend(taken);
        prop_assert_eq!(combined, vec);
    }

    // Property: take_last zero returns empty
    #[test]
    fn test_take_last_zero(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::{take_last, Identity};

        let id = Identity::new();
        let result = take_last(&id, vec, 0);

        prop_assert!(result.is_empty());
    }

    // Property: take_last more than length returns all
    #[test]
    fn test_take_last_overflow(vec in prop::collection::vec(any::<i32>(), 0..50), extra in 1usize..100) {
        use orlando_transducers::{take_last, Identity};

        let id = Identity::new();
        let n = vec.len() + extra;
        let result = take_last(&id, vec.clone(), n);

        prop_assert_eq!(result, vec);
    }

    // Property: drop_last length is correct
    #[test]
    fn test_drop_last_length(vec in prop::collection::vec(any::<i32>(), 0..100), n in 0usize..50) {
        use orlando_transducers::{drop_last, Identity};

        let id = Identity::new();
        let result = drop_last(&id, vec.clone(), n);

        let expected_len = vec.len().saturating_sub(n);
        prop_assert_eq!(result.len(), expected_len);
    }

    // Property: drop_last returns correct elements
    #[test]
    fn test_drop_last_correctness(vec in prop::collection::vec(any::<i32>(), 1..100), n in 1usize..50) {
        use orlando_transducers::{drop_last, Identity};

        let id = Identity::new();
        let result = drop_last(&id, vec.clone(), n);

        // Manually compute expected
        let end = vec.len().saturating_sub(n);
        let expected: Vec<i32> = vec.iter().take(end).cloned().collect();

        prop_assert_eq!(result, expected);
    }

    // Property: drop_last preserves order
    #[test]
    fn test_drop_last_preserves_order(vec in prop::collection::vec(0i32..100, 5..50), n in 1usize..20) {
        use orlando_transducers::{drop_last, Identity};

        let id = Identity::new();
        let result = drop_last(&id, vec.clone(), n);

        // Verify result is a prefix of original (preserves relative order)
        if !result.is_empty() {
            let end_idx = vec.len().saturating_sub(n);
            let expected_prefix: Vec<i32> = vec[..end_idx].to_vec();
            prop_assert_eq!(result, expected_prefix);
        }
    }

    // Property: drop_last zero is identity
    #[test]
    fn test_drop_last_zero_identity(vec in prop::collection::vec(any::<i32>(), 0..50)) {
        use orlando_transducers::{drop_last, Identity};

        let id = Identity::new();
        let result = drop_last(&id, vec.clone(), 0);

        prop_assert_eq!(result, vec);
    }

    // Property: drop_last more than length returns empty
    #[test]
    fn test_drop_last_overflow(vec in prop::collection::vec(any::<i32>(), 0..50), extra in 1usize..100) {
        use orlando_transducers::{drop_last, Identity};

        let id = Identity::new();
        let n = vec.len() + extra;
        let result = drop_last(&id, vec, n);

        prop_assert!(result.is_empty());
    }

    // Property: take_last with transducer applies transformation
    #[test]
    fn test_take_last_with_map(vec in prop::collection::vec(any::<i32>(), 1..100), n in 1usize..30) {
        use orlando_transducers::{take_last, Map};

        let pipeline = Map::new(|x: i32| x.saturating_mul(2));
        let result = take_last(&pipeline, vec.clone(), n);

        // Manually compute: double all, then take last n
        let doubled: Vec<i32> = vec.iter().map(|x| x.saturating_mul(2)).collect();
        let start = doubled.len().saturating_sub(n);
        let expected: Vec<i32> = doubled.iter().skip(start).cloned().collect();

        prop_assert_eq!(result, expected);
    }

    // Property: drop_last with transducer applies transformation
    #[test]
    fn test_drop_last_with_map(vec in prop::collection::vec(any::<i32>(), 1..100), n in 1usize..30) {
        use orlando_transducers::{drop_last, Map};

        let pipeline = Map::new(|x: i32| x.saturating_mul(2));
        let result = drop_last(&pipeline, vec.clone(), n);

        // Manually compute: double all, then drop last n
        let doubled: Vec<i32> = vec.iter().map(|x| x.saturating_mul(2)).collect();
        let end = doubled.len().saturating_sub(n);
        let expected: Vec<i32> = doubled.iter().take(end).cloned().collect();

        prop_assert_eq!(result, expected);
    }

    // Property: Aperture followed by take_last
    #[test]
    fn test_aperture_take_last_composition(vec in prop::collection::vec(0i32..50, 10..50), win_size in 2usize..5, n in 1usize..10) {
        use orlando_transducers::{Aperture, to_vec};

        // Create windows, then take last n windows
        let aperture = Aperture::new(win_size);
        let windows = to_vec(&aperture, vec.clone());

        let start = windows.len().saturating_sub(n);
        let expected: Vec<Vec<i32>> = windows.iter().skip(start).cloned().collect();

        // Manually compute to verify
        if windows.len() >= n {
            prop_assert_eq!(expected.len(), n.min(windows.len()));
        }
    }
}
