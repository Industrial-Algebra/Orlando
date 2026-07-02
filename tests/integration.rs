// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Orlando transducers.

use orlando_transducers::*;

#[test]
fn test_map_filter_take_pipeline() {
    let pipeline = Map::new(|x: i32| x * 2)
        .compose(Filter::new(|x: &i32| *x % 3 == 0))
        .compose(Take::new(5));

    let result = to_vec(&pipeline, 1..100);
    assert_eq!(result, vec![6, 12, 18, 24, 30]);
}

#[test]
fn test_complex_pipeline() {
    // Build a complex pipeline with 10+ operations
    let pipeline = Map::new(|x: i32| x + 1)
        .compose(Filter::new(|x: &i32| *x % 2 == 0))
        .compose(Map::new(|x: i32| x * 3))
        .compose(Filter::new(|x: &i32| *x > 10))
        .compose(Take::new(5))
        .compose(Map::new(|x: i32| x - 1));

    let result = to_vec(&pipeline, 0..100);
    assert_eq!(result, vec![11, 17, 23, 29, 35]);
}

#[test]
fn test_take_while() {
    let pipeline = TakeWhile::new(|x: &i32| *x < 10);
    let result = to_vec(&pipeline, 1..100);
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_drop() {
    let pipeline = Drop::new(5);
    let result = to_vec(&pipeline, 1..11);
    assert_eq!(result, vec![6, 7, 8, 9, 10]);
}

#[test]
fn test_drop_while() {
    let pipeline = DropWhile::new(|x: &i32| *x < 5);
    let result = to_vec(&pipeline, 1..11);
    assert_eq!(result, vec![5, 6, 7, 8, 9, 10]);
}

#[test]
fn test_unique() {
    let pipeline = Unique::<i32>::new();
    let result = to_vec(&pipeline, vec![1, 1, 2, 2, 3, 3, 2, 1]);
    assert_eq!(result, vec![1, 2, 3, 2, 1]);
}

#[test]
fn test_unique_by() {
    let pipeline = UniqueBy::new(|x: &i32| x.abs());
    let result = to_vec(&pipeline, vec![1, -1, 2, -2, 3, -3]);
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_scan() {
    // Running sum
    let pipeline = Scan::new(0, |acc: &i32, x: &i32| acc + x);
    let result = to_vec(&pipeline, vec![1, 2, 3, 4, 5]);
    assert_eq!(result, vec![1, 3, 6, 10, 15]);
}

#[test]
fn test_tap_side_effects() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let logged = Rc::new(RefCell::new(Vec::new()));
    let logged_clone = Rc::clone(&logged);

    let pipeline = Tap::new(move |x: &i32| {
        logged_clone.borrow_mut().push(*x);
    })
    .compose(Filter::new(|x: &i32| *x % 2 == 0));

    let result = to_vec(&pipeline, vec![1, 2, 3, 4, 5]);

    assert_eq!(result, vec![2, 4]);
    assert_eq!(*logged.borrow(), vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_collectors_sum() {
    let pipeline = Map::new(|x: i32| x * 2);
    let result = sum(&pipeline, vec![1, 2, 3, 4, 5]);
    assert_eq!(result, 30);
}

#[test]
fn test_collectors_count() {
    let pipeline = Filter::new(|x: &i32| *x % 2 == 0);
    let result = count(&pipeline, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(result, 5);
}

#[test]
fn test_collectors_first() {
    let pipeline = Filter::new(|x: &i32| *x % 5 == 0);
    let result = first(&pipeline, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(result, Some(5));
}

#[test]
fn test_collectors_last() {
    let pipeline = Filter::new(|x: &i32| *x % 2 == 0);
    let result = last(&pipeline, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(result, Some(10));
}

#[test]
fn test_collectors_every() {
    let id = Identity::<i32>::new();
    assert!(every(&id, vec![2, 4, 6, 8], |x| x % 2 == 0));
    assert!(!every(&id, vec![2, 3, 6, 8], |x| x % 2 == 0));
}

#[test]
fn test_collectors_some() {
    let id = Identity::<i32>::new();
    assert!(some(&id, vec![1, 3, 4, 5], |x| x % 2 == 0));
    assert!(!some(&id, vec![1, 3, 5, 7], |x| x % 2 == 0));
}

#[test]
fn test_early_termination_efficiency() {
    // This should only process 10 elements, not 1 million
    use std::cell::RefCell;
    use std::rc::Rc;

    let processed = Rc::new(RefCell::new(0));
    let processed_clone = Rc::clone(&processed);

    let pipeline = Tap::new(move |_: &i32| {
        *processed_clone.borrow_mut() += 1;
    })
    .compose(Take::new(10));

    let result = to_vec(&pipeline, 1..1_000_000);

    assert_eq!(result.len(), 10);
    assert_eq!(*processed.borrow(), 10); // Should only process 10 elements
}

#[test]
fn test_large_dataset() {
    let pipeline = Map::new(|x: i32| x * 2)
        .compose(Filter::new(|x: &i32| *x % 7 == 0))
        .compose(Take::new(100));

    let result = to_vec(&pipeline, 1..1_000_000);
    assert_eq!(result.len(), 100);
    assert_eq!(result[0], 14);
    assert_eq!(result[99], 1400);
}

#[test]
fn test_composition_associativity() {
    // (f ∘ g) ∘ h = f ∘ (g ∘ h)
    let f = Map::new(|x: i32| x + 1);
    let g = Map::new(|x: i32| x * 2);
    let h = Map::new(|x: i32| x - 3);

    let left = f.compose(g).compose(h);
    let right_inner = Map::new(|x: i32| x * 2).compose(Map::new(|x: i32| x - 3));
    let right = Map::new(|x: i32| x + 1).compose(right_inner);

    let data = vec![1, 2, 3, 4, 5];
    assert_eq!(to_vec(&left, data.clone()), to_vec(&right, data));
}

#[test]
fn test_identity_laws() {
    let f = Map::new(|x: i32| x * 2);
    let id = Identity::<i32>::new();

    let data = vec![1, 2, 3, 4, 5];

    // id ∘ f = f
    let left = id.compose(Map::new(|x: i32| x * 2));
    assert_eq!(to_vec(&left, data.clone()), to_vec(&f, data.clone()));

    // f ∘ id = f
    let right = Map::new(|x: i32| x * 2).compose(Identity::<i32>::new());
    assert_eq!(to_vec(&right, data.clone()), to_vec(&f, data.clone()));
}

// ========================================
// Phase 2b Advanced Operations Tests
// ========================================

#[test]
fn test_interpose() {
    let pipeline = Interpose::new(0);
    let result = to_vec(&pipeline, vec![1, 2, 3, 4]);
    assert_eq!(result, vec![1, 0, 2, 0, 3, 0, 4]);
}

#[test]
fn test_interpose_empty() {
    let pipeline = Interpose::new(0);
    let result = to_vec(&pipeline, Vec::<i32>::new());
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_interpose_single() {
    let pipeline = Interpose::new(0);
    let result = to_vec(&pipeline, vec![1]);
    assert_eq!(result, vec![1]);
}

#[test]
fn test_interpose_with_strings() {
    let pipeline = Interpose::new(",".to_string());
    let result = to_vec(
        &pipeline,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
    );
    assert_eq!(
        result,
        vec![
            "a".to_string(),
            ",".to_string(),
            "b".to_string(),
            ",".to_string(),
            "c".to_string()
        ]
    );
}

#[test]
fn test_interpose_composition() {
    let pipeline = Map::new(|x: i32| x * 2).compose(Interpose::new(0));
    let result = to_vec(&pipeline, vec![1, 2, 3]);
    assert_eq!(result, vec![2, 0, 4, 0, 6]);
}

#[test]
fn test_repeat_each() {
    let pipeline = RepeatEach::new(3);
    let result = to_vec(&pipeline, vec![1, 2, 3]);
    assert_eq!(result, vec![1, 1, 1, 2, 2, 2, 3, 3, 3]);
}

#[test]
fn test_repeat_each_zero() {
    let pipeline = RepeatEach::new(0);
    let result = to_vec(&pipeline, vec![1, 2, 3]);
    assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_repeat_each_one() {
    let pipeline = RepeatEach::new(1);
    let result = to_vec(&pipeline, vec![1, 2, 3]);
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_repeat_each_composition() {
    let pipeline = Map::new(|x: i32| x * 2).compose(RepeatEach::new(2));
    let result = to_vec(&pipeline, vec![1, 2, 3]);
    assert_eq!(result, vec![2, 2, 4, 4, 6, 6]);
}

#[test]
fn test_repeat_each_with_take() {
    // RepeatEach should respect early termination
    let pipeline = RepeatEach::new(3).compose(Take::new(5));
    let result = to_vec(&pipeline, vec![1, 2, 3]);
    assert_eq!(result, vec![1, 1, 1, 2, 2]); // Stops after 5 elements
}

#[test]
fn test_partition_by() {
    let id = Identity::new();
    let data = vec![1, 1, 2, 2, 3, 1, 1];
    let groups = partition_by(&id, data, |x| *x);

    assert_eq!(groups.len(), 4);
    assert_eq!(groups[0], vec![1, 1]);
    assert_eq!(groups[1], vec![2, 2]);
    assert_eq!(groups[2], vec![3]);
    assert_eq!(groups[3], vec![1, 1]);
}

#[test]
fn test_partition_by_empty() {
    let id = Identity::new();
    let data: Vec<i32> = vec![];
    let groups = partition_by(&id, data, |x| *x);
    assert_eq!(groups.len(), 0);
}

#[test]
fn test_partition_by_single_group() {
    let id = Identity::new();
    let data = vec![1, 1, 1, 1];
    let groups = partition_by(&id, data, |x| *x);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0], vec![1, 1, 1, 1]);
}

#[test]
fn test_partition_by_with_transform() {
    let pipeline = Map::new(|x: i32| x % 3);
    let data = vec![3, 6, 9, 1, 4, 7, 2, 5];
    let groups = partition_by(&pipeline, data, |x| *x);

    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0], vec![0, 0, 0]); // 3%3, 6%3, 9%3
    assert_eq!(groups[1], vec![1, 1, 1]); // 1%3, 4%3, 7%3
    assert_eq!(groups[2], vec![2, 2]); // 2%3, 5%3
}

#[test]
fn test_top_k() {
    let id = Identity::new();
    let data = vec![1, 5, 2, 8, 3, 9, 4, 6, 7];
    let top3 = top_k(&id, data, 3);

    assert_eq!(top3.len(), 3);
    assert_eq!(top3, vec![9, 8, 7]); // In descending order
}

#[test]
fn test_top_k_k_greater_than_n() {
    let id = Identity::new();
    let data = vec![1, 2, 3];
    let top10 = top_k(&id, data, 10);

    assert_eq!(top10.len(), 3);
    assert_eq!(top10, vec![3, 2, 1]);
}

#[test]
fn test_top_k_empty() {
    let id = Identity::new();
    let data: Vec<i32> = vec![];
    let top3 = top_k(&id, data, 3);
    assert_eq!(top3.len(), 0);
}

#[test]
fn test_top_k_with_transform() {
    let pipeline = Map::new(|x: i32| x * 2);
    let data = vec![1, 5, 2, 8, 3];
    let top3 = top_k(&pipeline, data, 3);

    assert_eq!(top3.len(), 3);
    assert_eq!(top3, vec![16, 10, 6]); // Top 3 after doubling
}

#[test]
fn test_frequencies() {
    let id = Identity::new();
    let data = vec![1, 2, 3, 1, 2, 1];
    let freqs = frequencies(&id, data);

    assert_eq!(freqs.get(&1), Some(&3));
    assert_eq!(freqs.get(&2), Some(&2));
    assert_eq!(freqs.get(&3), Some(&1));
}

#[test]
fn test_frequencies_empty() {
    let id = Identity::new();
    let data: Vec<i32> = vec![];
    let freqs = frequencies(&id, data);
    assert_eq!(freqs.len(), 0);
}

#[test]
fn test_frequencies_with_transform() {
    let pipeline = Map::new(|x: i32| x % 3);
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let freqs = frequencies(&pipeline, data);

    assert_eq!(freqs.get(&0), Some(&3)); // 3, 6, 9
    assert_eq!(freqs.get(&1), Some(&3)); // 1, 4, 7
    assert_eq!(freqs.get(&2), Some(&3)); // 2, 5, 8
}

#[test]
fn test_zip_longest() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5];
    let result = zip_longest(a, b, 0, 0);

    assert_eq!(result, vec![(1, 4), (2, 5), (3, 0)]);
}

#[test]
fn test_zip_longest_equal_length() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];
    let result = zip_longest(a, b, 0, 0);

    assert_eq!(result, vec![(1, 4), (2, 5), (3, 6)]);
}

#[test]
fn test_zip_longest_first_longer() {
    let a = vec![1, 2, 3, 4];
    let b = vec![5, 6];
    let result = zip_longest(a, b, -1, -1);

    assert_eq!(result, vec![(1, 5), (2, 6), (3, -1), (4, -1)]);
}

#[test]
fn test_zip_longest_empty() {
    let a: Vec<i32> = vec![];
    let b: Vec<i32> = vec![];
    let result = zip_longest(a, b, 0, 0);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_cartesian_product() {
    let a = vec![1, 2];
    let b = vec![3, 4, 5];
    let result = cartesian_product(a, b);

    assert_eq!(result.len(), 6);
    assert!(result.contains(&(1, 3)));
    assert!(result.contains(&(1, 4)));
    assert!(result.contains(&(1, 5)));
    assert!(result.contains(&(2, 3)));
    assert!(result.contains(&(2, 4)));
    assert!(result.contains(&(2, 5)));
}

#[test]
fn test_cartesian_product_empty() {
    let a: Vec<i32> = vec![];
    let b = vec![1, 2, 3];
    let result = cartesian_product(a, b);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_cartesian_product_single() {
    let a = vec![1];
    let b = vec![2];
    let result = cartesian_product(a, b);
    assert_eq!(result, vec![(1, 2)]);
}

#[test]
fn test_reservoir_sample() {
    let id = Identity::new();
    let data: Vec<i32> = (1..=100).collect();
    let sample = reservoir_sample(&id, data.clone(), 10);

    // Verify size
    assert_eq!(sample.len(), 10);

    // Verify all sampled values are from the original data
    for value in &sample {
        assert!(data.contains(value));
    }
}

#[test]
fn test_reservoir_sample_k_greater_than_n() {
    let id = Identity::new();
    let data = vec![1, 2, 3];
    let sample = reservoir_sample(&id, data.clone(), 10);

    // Should return all elements when k > n
    assert_eq!(sample.len(), 3);
    assert!(sample.contains(&1));
    assert!(sample.contains(&2));
    assert!(sample.contains(&3));
}

#[test]
fn test_reservoir_sample_empty() {
    let id = Identity::new();
    let data: Vec<i32> = vec![];
    let sample = reservoir_sample(&id, data, 10);
    assert_eq!(sample.len(), 0);
}

#[test]
fn test_reservoir_sample_with_transform() {
    let pipeline = Map::new(|x: i32| x * 2);
    let data: Vec<i32> = (1..=50).collect();
    let sample = reservoir_sample(&pipeline, data, 5);

    // Verify size
    assert_eq!(sample.len(), 5);

    // Verify all values are even (from the doubling transform)
    for value in &sample {
        assert_eq!(value % 2, 0);
    }
}

// ========================================
// Logic Functions Tests
// ========================================

#[test]
fn test_both_predicate() {
    use orlando_transducers::logic::both;

    let is_positive = |x: &i32| *x > 0;
    let is_even = |x: &i32| x % 2 == 0;
    let is_positive_even = both(is_positive, is_even);

    let pipeline = Filter::new(is_positive_even);
    let result = to_vec(&pipeline, vec![-2, -1, 0, 1, 2, 3, 4, 5, 6]);
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_either_predicate() {
    use orlando_transducers::logic::either;

    let is_small = |x: &i32| *x < 10;
    let is_large = |x: &i32| *x > 90;
    let is_extreme = either(is_small, is_large);

    let pipeline = Filter::new(is_extreme);
    let result = to_vec(&pipeline, vec![5, 25, 50, 75, 95]);
    assert_eq!(result, vec![5, 95]);
}

#[test]
fn test_complement_predicate() {
    use orlando_transducers::logic::complement;

    let is_even = |x: &i32| x % 2 == 0;
    let is_odd = complement(is_even);

    let pipeline = Filter::new(is_odd);
    let result = to_vec(&pipeline, vec![1, 2, 3, 4, 5, 6]);
    assert_eq!(result, vec![1, 3, 5]);
}

#[test]
fn test_all_pass_predicate() {
    use orlando_transducers::logic::{all_pass, PredicateVec};

    let predicates: PredicateVec<i32> = vec![
        Box::new(|x: &i32| *x > 0),
        Box::new(|x: &i32| *x < 100),
        Box::new(|x: &i32| x % 2 == 0),
    ];

    let is_valid = all_pass(predicates);
    let pipeline = Filter::new(is_valid);
    let result = to_vec(&pipeline, vec![-10, 3, 20, 50, 101, 150]);
    assert_eq!(result, vec![20, 50]);
}

#[test]
fn test_any_pass_predicate() {
    use orlando_transducers::logic::{any_pass, PredicateVec};

    let predicates: PredicateVec<i32> = vec![
        Box::new(|x: &i32| *x == 0),
        Box::new(|x: &i32| x % 10 == 0),
        Box::new(|x: &i32| *x > 100),
    ];

    let is_special = any_pass(predicates);
    let pipeline = Filter::new(is_special);
    let result = to_vec(&pipeline, vec![0, 7, 20, 50, 150]);
    assert_eq!(result, vec![0, 20, 50, 150]);
}

#[test]
fn test_when_transducer() {
    use orlando_transducers::logic::When;

    let double_if_positive = When::new(|x: &i32| *x > 0, |x: i32| x * 2);
    let result = to_vec(&double_if_positive, vec![-5, -2, 0, 3, 7]);
    assert_eq!(result, vec![-5, -2, 0, 6, 14]);
}

#[test]
fn test_unless_transducer() {
    use orlando_transducers::logic::Unless;

    let zero_if_negative = Unless::new(|x: &i32| *x >= 0, |_| 0);
    let result = to_vec(&zero_if_negative, vec![-5, -2, 0, 3, 7]);
    assert_eq!(result, vec![0, 0, 0, 3, 7]);
}

#[test]
fn test_if_else_transducer() {
    use orlando_transducers::logic::IfElse;

    let transform = IfElse::new(
        |x: &i32| *x >= 0,
        |x: i32| x * 2, // double if positive
        |x: i32| x / 2, // halve if negative
    );
    let result = to_vec(&transform, vec![-10, -4, 0, 5, 8]);
    assert_eq!(result, vec![-5, -2, 0, 10, 16]);
}

#[test]
fn test_when_composition() {
    use orlando_transducers::logic::When;

    // When composed with Map
    let pipeline = Map::new(|x: i32| x * 2).compose(When::new(|x: &i32| *x > 10, |x: i32| x + 100));

    let result = to_vec(&pipeline, vec![1, 5, 10, 15]);
    // 1*2=2, 5*2=10, 10*2=20 -> 120, 15*2=30 -> 130
    assert_eq!(result, vec![2, 10, 120, 130]);
}

#[test]
fn test_if_else_with_filter() {
    use orlando_transducers::logic::IfElse;

    // Transform then filter
    let pipeline = IfElse::new(|x: &i32| *x % 2 == 0, |x: i32| x / 2, |x: i32| x * 3)
        .compose(Filter::new(|x: &i32| *x > 5));

    let result = to_vec(&pipeline, vec![2, 3, 4, 5, 10]);
    // 2/2=1, 3*3=9, 4/2=2, 5*3=15, 10/2=5
    // Filter > 5: [9, 15]
    assert_eq!(result, vec![9, 15]);
}

#[test]
fn test_nested_logic_predicates() {
    use orlando_transducers::logic::{both, either};

    // (positive AND even) OR (negative AND odd)
    let is_positive_even = both(|x: &i32| *x > 0, |x: &i32| x % 2 == 0);
    let is_negative_odd = both(|x: &i32| *x < 0, |x: &i32| x % 2 != 0);
    let complex_pred = either(is_positive_even, is_negative_odd);

    let pipeline = Filter::new(complex_pred);
    let result = to_vec(&pipeline, vec![-4, -3, -2, -1, 0, 1, 2, 3, 4]);
    // Positive evens: 2, 4
    // Negative odds: -3, -1
    assert_eq!(result, vec![-3, -1, 2, 4]);
}

#[test]
fn test_when_with_take() {
    use orlando_transducers::logic::When;

    // When should respect early termination
    let pipeline = When::new(|x: &i32| *x > 0, |x: i32| x * 10).compose(Take::new(3));

    let result = to_vec(&pipeline, vec![-1, 2, -3, 4, 5, 6]);
    // -1, 20, -3 (take 3)
    assert_eq!(result, vec![-1, 20, -3]);
}

#[test]
fn test_complex_logic_pipeline() {
    use orlando_transducers::logic::{all_pass, IfElse, PredicateVec};

    // Multi-stage validation and transformation
    let is_valid_range: PredicateVec<i32> =
        vec![Box::new(|x: &i32| *x > -100), Box::new(|x: &i32| *x < 100)];

    let pipeline = Filter::new(all_pass(is_valid_range))
        .compose(IfElse::new(
            |x: &i32| *x >= 0,
            |x: i32| x + 10,
            |x: i32| x - 10,
        ))
        .compose(Filter::new(|x: &i32| x.abs() > 5));

    let result = to_vec(&pipeline, vec![-50, -2, 0, 3, 50, 150]);
    // Filter -100..100: [-50, -2, 0, 3, 50]
    // Transform: [-60, -12, 10, 13, 60]
    // Filter abs > 5: [-60, -12, 10, 13, 60]
    assert_eq!(result, vec![-60, -12, 10, 13, 60]);
}

// ========================================
// Optics Integration Tests
// ========================================

#[test]
fn test_prism_with_enum_pipeline() {
    use orlando_transducers::optics::Prism;

    #[derive(Clone, Debug, PartialEq)]
    enum Value {
        Int(i32),
        Str(String),
    }

    let int_prism = Prism::new(
        |v: &Value| match v {
            Value::Int(n) => Some(*n),
            _ => None,
        },
        |n: i32| Value::Int(n),
    );

    // Use prism to extract and transform
    let values = [Value::Int(1), Value::Str("hi".into()), Value::Int(3)];
    let doubled: Vec<Value> = values
        .iter()
        .map(|v| int_prism.over(v, |n| n * 2))
        .collect();

    assert_eq!(
        doubled,
        vec![Value::Int(2), Value::Str("hi".into()), Value::Int(6)]
    );
}

#[test]
fn test_iso_bidirectional() {
    use orlando_transducers::optics::Iso;

    // Radians ↔ Degrees
    let rad_deg = Iso::new(
        |r: &f64| r * 180.0 / std::f64::consts::PI,
        |d: f64| d * std::f64::consts::PI / 180.0,
    );

    let pi = std::f64::consts::PI;
    let degrees = rad_deg.to(&pi);
    assert!((degrees - 180.0).abs() < 1e-10);

    let back = rad_deg.from(degrees);
    assert!((back - pi).abs() < 1e-10);

    // Reverse
    let deg_rad = rad_deg.reverse();
    let radians = deg_rad.to(&180.0);
    assert!((radians - pi).abs() < 1e-10);
}

#[test]
fn test_fold_aggregate() {
    use orlando_transducers::optics::Fold;

    #[derive(Clone, Debug)]
    struct Order {
        items: Vec<f64>,
    }

    let prices_fold = Fold::new(|order: &Order| order.items.clone());

    let order = Order {
        items: vec![9.99, 24.50, 3.75],
    };

    let prices = prices_fold.fold_of(&order);
    let total: f64 = prices.iter().sum();
    assert!((total - 38.24).abs() < 1e-10);
    assert_eq!(prices_fold.length(&order), 3);
}

#[test]
fn test_traversal_with_nested_structs() {
    use orlando_transducers::optics::Traversal;

    #[derive(Clone, Debug, PartialEq)]
    struct Config {
        values: Vec<i32>,
        name: String,
    }

    let values_traversal = Traversal::new(
        |cfg: &Config| cfg.values.clone(),
        |cfg: &Config, f: &dyn Fn(i32) -> i32| Config {
            values: cfg.values.iter().map(|x| f(*x)).collect(),
            name: cfg.name.clone(),
        },
    );

    let cfg = Config {
        values: vec![1, 2, 3],
        name: "test".into(),
    };

    // Transform all values
    let updated = values_traversal.over_all(&cfg, |x| x * 10);
    assert_eq!(updated.values, vec![10, 20, 30]);
    assert_eq!(updated.name, "test");

    // Set all values
    let zeroed = values_traversal.set_all(&cfg, 0);
    assert_eq!(zeroed.values, vec![0, 0, 0]);

    // Convert to fold
    let fold = values_traversal.as_fold();
    assert_eq!(fold.fold_of(&cfg), vec![1, 2, 3]);
}

#[test]
fn test_iso_as_lens_composition() {
    use orlando_transducers::optics::{Iso, Lens};

    #[derive(Clone, Debug, PartialEq)]
    struct Wrapper {
        inner: f64,
    }

    let inner_lens = Lens::new(
        |w: &Wrapper| w.inner,
        |_w: &Wrapper, v: f64| Wrapper { inner: v },
    );

    // Celsius value inside a wrapper
    let celsius_fahrenheit = Iso::new(
        |c: &f64| *c * 9.0 / 5.0 + 32.0,
        |f: f64| (f - 32.0) * 5.0 / 9.0,
    );

    // Compose: wrapper → inner (lens) → fahrenheit (iso as lens)
    let wrapper_to_f = inner_lens.compose(celsius_fahrenheit.as_lens());

    let w = Wrapper { inner: 100.0 };
    let f = wrapper_to_f.get(&w);
    assert!((f - 212.0).abs() < 1e-10);

    let updated = wrapper_to_f.set(&w, 32.0);
    assert!((updated.inner - 0.0).abs() < 1e-10);
}

// ===== Geometric Optics Integration Tests =====

#[test]
fn test_geometric_optics_with_pipeline() {
    use orlando_transducers::geometric_optics;

    // Pipeline: stream of 3D multivectors → extract bivector part → filter by norm → take first 2
    let multivectors = vec![
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // scalar only
        vec![0.0, 0.0, 0.0, 5.0, 0.0, 5.0, 5.0, 0.0], // bivector norm = sqrt(75)
        vec![0.0, 0.0, 0.0, 0.1, 0.0, 0.0, 0.0, 0.0], // small bivector
        vec![0.0, 0.0, 0.0, 3.0, 0.0, 4.0, 0.0, 0.0], // bivector norm = 5
    ];

    // Extract bivector norms, filter by threshold, take 2
    let pipeline = Map::new(move |mv: Vec<f64>| {
        let bv = geometric_optics::grade_extract(3, 2, &mv);
        geometric_optics::norm(&bv)
    })
    .compose(Filter::new(|n: &f64| *n > 1.0))
    .compose(Take::new(2));

    let result = to_vec(&pipeline, multivectors);
    assert_eq!(result.len(), 2);
    assert!((result[0] - 75.0f64.sqrt()).abs() < 1e-10);
    assert!((result[1] - 5.0).abs() < 1e-10);
}

#[test]
fn test_geometric_optics_grade_project_in_pipeline() {
    use orlando_transducers::geometric_optics;

    // Pipeline: project each multivector to its vector part
    let multivectors = vec![
        vec![10.0, 1.0, 2.0, 99.0, 3.0, 99.0, 99.0, 99.0],
        vec![20.0, 4.0, 5.0, 88.0, 6.0, 88.0, 88.0, 88.0],
    ];

    let pipeline = Map::new(move |mv: Vec<f64>| geometric_optics::grade_project(3, 1, &mv));

    let result = to_vec(&pipeline, multivectors);
    assert_eq!(result[0], vec![0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0]);
    assert_eq!(result[1], vec![0.0, 4.0, 5.0, 0.0, 6.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_geometric_optics_pure_grade_filter() {
    use orlando_transducers::geometric_optics;

    // Filter to only pure k-vectors (single grade)
    let multivectors = vec![
        vec![0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0], // pure vector
        vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // mixed (scalar + vector)
        vec![0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 0.0], // pure bivector
    ];

    let pipeline = Filter::new(move |mv: &Vec<f64>| geometric_optics::is_pure_grade(3, mv));

    let result = to_vec(&pipeline, multivectors);
    assert_eq!(result.len(), 2); // only the pure vector and pure bivector
}

// ===== Phase 7: Reactive Streams Integration Tests =====

#[test]
fn test_signal_with_transducer() {
    use orlando_transducers::signal::Signal;

    // Signal + map chain
    let temperature_c = Signal::new(0.0);
    let temperature_f = temperature_c.map(|c| c * 9.0 / 5.0 + 32.0);

    assert!((*temperature_f.get() - 32.0f64).abs() < 1e-10);
    temperature_c.set(100.0);
    assert!((*temperature_f.get() - 212.0f64).abs() < 1e-10);
}

#[test]
fn test_stream_fold_with_pipeline_pattern() {
    use orlando_transducers::stream::Stream;

    // Stream fold pattern: accumulate click events into a count
    let events = Stream::new();
    let count = events.fold(0i32, |acc, n: &i32| acc + n);

    events.emit(1);
    events.emit(5);
    events.emit(3);
    assert_eq!(*count.get(), 9);
}

#[test]
fn test_stream_map_filter_take_pipeline() {
    use orlando_transducers::stream::Stream;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Stream with map → filter → take: same semantics as pull-based pipeline
    let source = Stream::new();
    let result = source.map(|x: &i32| x * 2).filter(|x: &i32| *x > 5).take(3);

    let output = Rc::new(RefCell::new(Vec::new()));
    let output_clone = output.clone();
    let _sub = result.subscribe(move |v: &i32| {
        output_clone.borrow_mut().push(*v);
    });

    // Emit: 1→2, 2→4, 3→6, 4→8, 5→10, 6→12, 7→14
    // Filter >5: 6, 8, 10, 12, 14
    // Take 3: 6, 8, 10
    for i in 1..=7 {
        source.emit(i);
    }
    assert_eq!(*output.borrow(), vec![6, 8, 10]);
}

#[test]
fn test_signal_combine_multiple() {
    use orlando_transducers::signal::Signal;

    let x = Signal::new(1.0);
    let y = Signal::new(2.0);
    let z = Signal::new(3.0);

    // Combine x and y, then combine with z
    let xy = x.combine(&y, |a, b| a + b);
    let xyz = xy.combine(&z, |ab, c| ab * c);

    assert!((*xyz.get() - 9.0f64).abs() < 1e-10); // (1+2)*3
    x.set(4.0);
    assert!((*xyz.get() - 18.0f64).abs() < 1e-10); // (4+2)*3
}
