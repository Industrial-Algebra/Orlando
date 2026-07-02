// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Terminal operations (collectors) for transducers.
//!
//! Collectors are reducing functions that consume the output of a transducer
//! and produce a final result.

use crate::step::{cont, Step};
use crate::transducer::Transducer;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Execute a transducer over an iterator and collect results into a vector.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::to_vec;
/// use orlando_transducers::transforms::Map;
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = to_vec(&double, vec![1, 2, 3].into_iter());
/// assert_eq!(result, vec![2, 4, 6]);
/// ```
pub fn to_vec<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Vec<U>
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
{
    let reducer = |mut acc: Vec<U>, x: U| {
        acc.push(x);
        cont(acc)
    };

    let transformed = transducer.apply(reducer);
    let mut result = Vec::new();

    for item in source {
        match transformed(result, item) {
            Step::Continue(new_result) => result = new_result,
            Step::Stop(final_result) => {
                result = final_result;
                break;
            }
        }
    }

    result
}

/// Reduce with a custom reducer function.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::reduce;
/// use orlando_transducers::transforms::Map;
/// use orlando_transducers::step::cont;
///
/// let double = Map::new(|x: i32| x * 2);
/// let sum = reduce(&double, vec![1, 2, 3].into_iter(), 0, |acc, x| cont(acc + x));
/// assert_eq!(sum, 12); // (1+2+3)*2
/// ```
pub fn reduce<T, U, Acc, Iter, R>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    initial: Acc,
    reducer: R,
) -> Acc
where
    T: 'static,
    U: 'static,
    Acc: 'static,
    Iter: IntoIterator<Item = T>,
    R: Fn(Acc, U) -> Step<Acc> + 'static,
{
    let transformed = transducer.apply(reducer);
    let mut acc = initial;

    for item in source {
        match transformed(acc, item) {
            Step::Continue(new_acc) => acc = new_acc,
            Step::Stop(final_acc) => {
                acc = final_acc;
                break;
            }
        }
    }

    acc
}

/// Sum numeric values.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::sum;
/// use orlando_transducers::transforms::Map;
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = sum(&double, vec![1, 2, 3].into_iter());
/// assert_eq!(result, 12);
/// ```
pub fn sum<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> U
where
    T: 'static,
    U: std::ops::Add<Output = U> + Default + 'static,
    Iter: IntoIterator<Item = T>,
{
    reduce(transducer, source, U::default(), |acc, x| cont(acc + x))
}

/// Count the number of elements.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::count;
/// use orlando_transducers::transforms::Filter;
///
/// let evens = Filter::new(|x: &i32| x % 2 == 0);
/// let result = count(&evens, vec![1, 2, 3, 4, 5].into_iter());
/// assert_eq!(result, 2);
/// ```
pub fn count<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> usize
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
{
    reduce(transducer, source, 0usize, |acc, _| cont(acc + 1))
}

/// Get the first element (utilizes early termination).
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::first;
/// use orlando_transducers::transforms::Filter;
///
/// let evens = Filter::new(|x: &i32| x % 2 == 0);
/// let result = first(&evens, vec![1, 3, 4, 5].into_iter());
/// assert_eq!(result, Some(4));
/// ```
pub fn first<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<U>
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
{
    use crate::step::stop;

    let reducer = |_acc: Option<U>, x: U| stop(Some(x));
    reduce(transducer, source, None, reducer)
}

/// Get the last element.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::last;
/// use orlando_transducers::transforms::Filter;
///
/// let evens = Filter::new(|x: &i32| x % 2 == 0);
/// let result = last(&evens, vec![2, 3, 4, 5, 6].into_iter());
/// assert_eq!(result, Some(6));
/// ```
pub fn last<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<U>
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
{
    reduce(transducer, source, None, |_acc, x| cont(Some(x)))
}

/// Test if all elements match a predicate.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::every;
/// use orlando_transducers::transducer::Identity;
///
/// let id = Identity::<i32>::new();
/// let result = every(&id, vec![2, 4, 6].into_iter(), |x| x % 2 == 0);
/// assert_eq!(result, true);
/// ```
pub fn every<T, U, Iter, P>(transducer: &impl Transducer<T, U>, source: Iter, predicate: P) -> bool
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
    P: Fn(&U) -> bool + 'static,
{
    use crate::step::stop;

    let reducer = move |_acc: bool, x: U| {
        if predicate(&x) {
            cont(true)
        } else {
            stop(false)
        }
    };

    reduce(transducer, source, true, reducer)
}

/// Test if any element matches a predicate.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::some;
/// use orlando_transducers::transducer::Identity;
///
/// let id = Identity::<i32>::new();
/// let result = some(&id, vec![1, 3, 4, 5].into_iter(), |x| x % 2 == 0);
/// assert_eq!(result, true);
/// ```
pub fn some<T, U, Iter, P>(transducer: &impl Transducer<T, U>, source: Iter, predicate: P) -> bool
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
    P: Fn(&U) -> bool + 'static,
{
    use crate::step::stop;

    let reducer = move |_acc: bool, x: U| {
        if predicate(&x) {
            stop(true)
        } else {
            cont(false)
        }
    };

    reduce(transducer, source, false, reducer)
}

/// Partition elements into two groups based on a predicate.
///
/// Returns a tuple of (pass, fail) where `pass` contains elements that
/// satisfy the predicate and `fail` contains those that don't.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::partition;
/// use orlando_transducers::transducer::Identity;
///
/// let id = Identity::<i32>::new();
/// let (evens, odds) = partition(&id, vec![1, 2, 3, 4, 5].into_iter(), |x| x % 2 == 0);
/// assert_eq!(evens, vec![2, 4]);
/// assert_eq!(odds, vec![1, 3, 5]);
/// ```
pub fn partition<T, U, Iter, P>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    predicate: P,
) -> (Vec<U>, Vec<U>)
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
    P: Fn(&U) -> bool + 'static,
{
    let reducer = move |mut acc: (Vec<U>, Vec<U>), x: U| {
        if predicate(&x) {
            acc.0.push(x);
        } else {
            acc.1.push(x);
        }
        cont(acc)
    };

    reduce(transducer, source, (Vec::new(), Vec::new()), reducer)
}

/// Find the first element that satisfies a predicate.
///
/// Returns `Some(element)` if found, `None` otherwise.
/// Utilizes early termination to stop as soon as a match is found.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::find;
/// use orlando_transducers::transducer::Identity;
///
/// let id = Identity::<i32>::new();
/// let result = find(&id, vec![1, 3, 4, 5].into_iter(), |x| x % 2 == 0);
/// assert_eq!(result, Some(4));
/// ```
pub fn find<T, U, Iter, P>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    predicate: P,
) -> Option<U>
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
    P: Fn(&U) -> bool + 'static,
{
    use crate::step::stop;

    let reducer = move |_acc: Option<U>, x: U| {
        if predicate(&x) {
            stop(Some(x))
        } else {
            cont(None)
        }
    };

    reduce(transducer, source, None, reducer)
}

/// Group elements by a key function into a HashMap.
///
/// Returns a HashMap where keys are produced by the key function and values
/// are vectors of elements that share that key.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::group_by;
/// use orlando_transducers::transducer::Identity;
/// use std::collections::HashMap;
///
/// let id = Identity::<i32>::new();
/// let groups = group_by(&id, vec![1, 2, 3, 4, 5, 6].into_iter(), |x| x % 3);
///
/// assert_eq!(groups.get(&0), Some(&vec![3, 6]));
/// assert_eq!(groups.get(&1), Some(&vec![1, 4]));
/// assert_eq!(groups.get(&2), Some(&vec![2, 5]));
/// ```
pub fn group_by<T, U, K, Iter, F>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    key_fn: F,
) -> HashMap<K, Vec<U>>
where
    T: 'static,
    U: 'static,
    K: Eq + Hash + 'static,
    Iter: IntoIterator<Item = T>,
    F: Fn(&U) -> K + 'static,
{
    let reducer = move |mut acc: HashMap<K, Vec<U>>, x: U| {
        let key = key_fn(&x);
        acc.entry(key).or_default().push(x);
        cont(acc)
    };

    reduce(transducer, source, HashMap::new(), reducer)
}

/// Test if NO elements match a predicate (inverse of `some`).
///
/// Returns true if all elements fail the predicate, false if any match.
/// Utilizes early termination to stop as soon as a match is found.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::none;
/// use orlando_transducers::transducer::Identity;
///
/// let id = Identity::<i32>::new();
/// assert!(none(&id, vec![1, 3, 5, 7].into_iter(), |x| x % 2 == 0)); // No evens
/// assert!(!none(&id, vec![1, 2, 3].into_iter(), |x| x % 2 == 0)); // Has evens
/// ```
pub fn none<T, U, Iter, P>(transducer: &impl Transducer<T, U>, source: Iter, predicate: P) -> bool
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
    P: Fn(&U) -> bool + 'static,
{
    use crate::step::stop;

    // Inverse of some - return false (stop) if any element matches
    let reducer = move |_acc: bool, x: U| {
        if predicate(&x) {
            stop(false) // Found a match, return false
        } else {
            cont(true) // Keep looking
        }
    };

    reduce(transducer, source, true, reducer)
}

/// Test if the collection contains a specific value.
///
/// Returns true if any element equals the target value, false otherwise.
/// Utilizes early termination to stop as soon as the value is found.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::contains;
/// use orlando_transducers::transducer::Identity;
///
/// let id = Identity::<i32>::new();
/// assert!(contains(&id, vec![1, 2, 3, 4, 5].into_iter(), &3));
/// assert!(!contains(&id, vec![1, 2, 4, 5].into_iter(), &3));
/// ```
pub fn contains<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter, value: &U) -> bool
where
    T: 'static,
    U: PartialEq + Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    use crate::step::stop;

    let target = value.clone();
    let reducer = move |_acc: bool, x: U| {
        if x == target {
            stop(true) // Found it!
        } else {
            cont(false) // Keep looking
        }
    };

    reduce(transducer, source, false, reducer)
}

/// Zip two iterators into pairs (helper function, not a transducer).
///
/// This doesn't fit the single-input transducer model, so it's implemented
/// as a standalone helper function. Stops when either iterator is exhausted.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::zip;
///
/// let a = vec![1, 2, 3];
/// let b = vec!['a', 'b', 'c', 'd'];
/// let result = zip(a, b);
/// assert_eq!(result, vec![(1, 'a'), (2, 'b'), (3, 'c')]);
/// ```
pub fn zip<T, U, IterT, IterU>(iter_a: IterT, iter_b: IterU) -> Vec<(T, U)>
where
    IterT: IntoIterator<Item = T>,
    IterU: IntoIterator<Item = U>,
{
    iter_a.into_iter().zip(iter_b).collect()
}

/// Zip two iterators with a combining function (helper function, not a transducer).
///
/// Like `zip`, but applies a function to combine the elements instead of
/// creating tuples. Stops when either iterator is exhausted.
///
/// # Examples
///
/// ```
/// use orlando_transducers::collectors::zip_with;
///
/// let a = vec![1, 2, 3];
/// let b = vec![10, 20, 30];
/// let result = zip_with(a, b, |x, y| x + y);
/// assert_eq!(result, vec![11, 22, 33]);
/// ```
pub fn zip_with<T, U, V, IterT, IterU, F>(iter_a: IterT, iter_b: IterU, combine: F) -> Vec<V>
where
    IterT: IntoIterator<Item = T>,
    IterU: IntoIterator<Item = U>,
    F: Fn(T, U) -> V,
{
    iter_a
        .into_iter()
        .zip(iter_b)
        .map(|(a, b)| combine(a, b))
        .collect()
}

/// Merge multiple iterators by interleaving their elements in round-robin fashion.
///
/// Takes elements from each iterator in turn until all iterators are exhausted.
/// If iterators have different lengths, continues with remaining iterators.
///
/// # Examples
///
/// ```
/// use orlando_transducers::merge;
///
/// let a = vec![1, 2, 3];
/// let b = vec![4, 5, 6];
/// let result = merge(vec![a, b]);
/// assert_eq!(result, vec![1, 4, 2, 5, 3, 6]);
/// ```
///
/// ```
/// use orlando_transducers::merge;
///
/// // Different length iterators
/// let a = vec![1, 2];
/// let b = vec![3, 4, 5, 6];
/// let result = merge(vec![a, b]);
/// assert_eq!(result, vec![1, 3, 2, 4, 5, 6]);
/// ```
pub fn merge<T, I>(iterators: Vec<I>) -> Vec<T>
where
    I: IntoIterator<Item = T>,
{
    let mut iters: Vec<_> = iterators.into_iter().map(|i| i.into_iter()).collect();
    let mut result = Vec::new();
    let mut active = true;

    while active {
        active = false;
        for iter in &mut iters {
            if let Some(val) = iter.next() {
                result.push(val);
                active = true;
            }
        }
    }

    result
}

/// Compute the intersection of two iterators (elements in both A and B).
///
/// Returns elements that appear in both iterators, preserving order from the first iterator.
/// Duplicates from the first iterator are included if the element exists in the second.
///
/// # Examples
///
/// ```
/// use orlando_transducers::intersection;
///
/// let a = vec![1, 2, 3, 4];
/// let b = vec![3, 4, 5, 6];
/// let result = intersection(a, b);
/// assert_eq!(result, vec![3, 4]);
/// ```
///
/// ```
/// use orlando_transducers::intersection;
///
/// let a = vec![1, 2, 2, 3];
/// let b = vec![2, 3, 4];
/// let result = intersection(a, b);
/// assert_eq!(result, vec![2, 2, 3]);
/// ```
pub fn intersection<T, IterA, IterB>(iter_a: IterA, iter_b: IterB) -> Vec<T>
where
    T: Eq + Hash + Clone,
    IterA: IntoIterator<Item = T>,
    IterB: IntoIterator<Item = T>,
{
    let set_b: HashSet<T> = iter_b.into_iter().collect();
    iter_a
        .into_iter()
        .filter(|item| set_b.contains(item))
        .collect()
}

/// Compute the difference of two iterators (elements in A but not in B).
///
/// Returns elements from the first iterator that don't appear in the second,
/// preserving order from the first iterator.
///
/// # Examples
///
/// ```
/// use orlando_transducers::difference;
///
/// let a = vec![1, 2, 3, 4];
/// let b = vec![3, 4, 5, 6];
/// let result = difference(a, b);
/// assert_eq!(result, vec![1, 2]);
/// ```
///
/// ```
/// use orlando_transducers::difference;
///
/// let a = vec![1, 2, 2, 3];
/// let b = vec![2];
/// let result = difference(a, b);
/// assert_eq!(result, vec![1, 3]);
/// ```
pub fn difference<T, IterA, IterB>(iter_a: IterA, iter_b: IterB) -> Vec<T>
where
    T: Eq + Hash + Clone,
    IterA: IntoIterator<Item = T>,
    IterB: IntoIterator<Item = T>,
{
    let set_b: HashSet<T> = iter_b.into_iter().collect();
    iter_a
        .into_iter()
        .filter(|item| !set_b.contains(item))
        .collect()
}

/// Compute the union of two iterators (unique elements from both A and B).
///
/// Returns all unique elements that appear in either iterator.
/// Order is preserved: all unique elements from A first, then unique elements from B.
///
/// # Examples
///
/// ```
/// use orlando_transducers::union;
///
/// let a = vec![1, 2, 3];
/// let b = vec![3, 4, 5];
/// let result = union(a, b);
/// assert_eq!(result, vec![1, 2, 3, 4, 5]);
/// ```
///
/// ```
/// use orlando_transducers::union;
///
/// let a = vec![1, 2, 2, 3];
/// let b = vec![3, 4, 4, 5];
/// let result = union(a, b);
/// assert_eq!(result, vec![1, 2, 3, 4, 5]);
/// ```
pub fn union<T, IterA, IterB>(iter_a: IterA, iter_b: IterB) -> Vec<T>
where
    T: Eq + Hash + Clone,
    IterA: IntoIterator<Item = T>,
    IterB: IntoIterator<Item = T>,
{
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    // Add all unique elements from A
    for item in iter_a {
        if seen.insert(item.clone()) {
            result.push(item);
        }
    }

    // Add all unique elements from B that aren't in A
    for item in iter_b {
        if seen.insert(item.clone()) {
            result.push(item);
        }
    }

    result
}

/// Compute the symmetric difference of two iterators (elements in A or B but not both).
///
/// Returns elements that appear in exactly one of the two iterators.
/// Order: unique-to-A elements first, then unique-to-B elements.
///
/// # Examples
///
/// ```
/// use orlando_transducers::symmetric_difference;
///
/// let a = vec![1, 2, 3, 4];
/// let b = vec![3, 4, 5, 6];
/// let result = symmetric_difference(a, b);
/// assert_eq!(result, vec![1, 2, 5, 6]);
/// ```
///
/// ```
/// use orlando_transducers::symmetric_difference;
///
/// let a = vec![1, 2];
/// let b = vec![3, 4];
/// let result = symmetric_difference(a, b);
/// assert_eq!(result, vec![1, 2, 3, 4]);
/// ```
pub fn symmetric_difference<T, IterA, IterB>(iter_a: IterA, iter_b: IterB) -> Vec<T>
where
    T: Eq + Hash + Clone,
    IterA: IntoIterator<Item = T>,
    IterB: IntoIterator<Item = T>,
{
    let vec_a: Vec<T> = iter_a.into_iter().collect();
    let vec_b: Vec<T> = iter_b.into_iter().collect();

    let set_a: HashSet<&T> = vec_a.iter().collect();
    let set_b: HashSet<&T> = vec_b.iter().collect();

    let mut result = Vec::new();
    let mut seen = HashSet::new();

    // Elements in A but not B (preserving order from A)
    for item in &vec_a {
        if !set_b.contains(item) && seen.insert(item) {
            result.push(item.clone());
        }
    }

    // Elements in B but not A (preserving order from B)
    for item in &vec_b {
        if !set_a.contains(item) && seen.insert(item) {
            result.push(item.clone());
        }
    }

    result
}

// ============================================================================
// Advanced Collectors (Phase 2b)
// ============================================================================

/// Perform reservoir sampling to randomly sample n elements from a stream.
///
/// Uses Algorithm R for uniform random sampling with constant memory (O(n)).
/// Each element has an equal probability of being selected, even for streams
/// of unknown size.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{reservoir_sample, Map};
///
/// let pipeline = Map::new(|x: i32| x * 2);
/// let sample = reservoir_sample(&pipeline, 1..1000, 10);
/// assert_eq!(sample.len(), 10);
/// // Each element from the processed stream has equal 10/999 probability
/// ```
///
/// ```
/// use orlando_transducers::{reservoir_sample, transducer::Identity};
///
/// // Sample 5 items from large dataset
/// let id = Identity::<i32>::new();
/// let sample = reservoir_sample(&id, 1..1_000_000, 5);
/// assert_eq!(sample.len(), 5);
/// ```
pub fn reservoir_sample<T, U, Iter>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    n: usize,
) -> Vec<U>
where
    T: 'static,
    U: 'static + Clone,
    Iter: IntoIterator<Item = T>,
{
    use rand::Rng;
    use std::cell::RefCell;
    use std::rc::Rc;

    let rng = Rc::new(RefCell::new(rand::thread_rng()));
    let reservoir: Rc<RefCell<Vec<U>>> = Rc::new(RefCell::new(Vec::with_capacity(n)));
    let count = Rc::new(RefCell::new(0usize));

    let reducer = {
        let rng = Rc::clone(&rng);
        let reservoir = Rc::clone(&reservoir);
        let count = Rc::clone(&count);

        move |_acc: (), x: U| {
            let mut c = count.borrow_mut();
            *c += 1;

            let mut res = reservoir.borrow_mut();
            if res.len() < n {
                // Fill reservoir
                res.push(x);
            } else {
                // Randomly replace elements with decreasing probability
                let mut rng_mut = rng.borrow_mut();
                let j = rng_mut.gen_range(0..*c);
                if j < n {
                    res[j] = x;
                }
            }

            cont(())
        }
    };

    reduce(transducer, source, (), reducer);
    Rc::try_unwrap(reservoir)
        .unwrap_or_else(|_| panic!("Failed to unwrap reservoir"))
        .into_inner()
}

/// Group consecutive elements by a key function (SQL-like PARTITION BY).
///
/// Unlike `group_by`, this only groups consecutive elements with the same key.
/// This is more memory-efficient for pre-sorted data and preserves streaming.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{partition_by, transducer::Identity};
///
/// let data = vec![1, 1, 2, 2, 1, 1];
/// let id = Identity::new();
/// let groups = partition_by(&id, data, |x| *x);
///
/// assert_eq!(groups.len(), 3);
/// assert_eq!(groups[0], vec![1, 1]);
/// assert_eq!(groups[1], vec![2, 2]);
/// assert_eq!(groups[2], vec![1, 1]);
/// ```
///
/// ```
/// use orlando_transducers::{partition_by, Map};
///
/// let pipeline = Map::new(|x: i32| x % 3);
/// let data = vec![3, 6, 9, 1, 4, 7, 2, 5];
/// let groups = partition_by(&pipeline, data, |x| *x);
/// // Groups by result: [[0,0,0], [1,1,1], [2,2]]
/// assert_eq!(groups.len(), 3);
/// ```
pub fn partition_by<T, U, K, Iter, F>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    key_fn: F,
) -> Vec<Vec<U>>
where
    T: 'static,
    U: 'static + Clone,
    K: Eq + Hash + 'static,
    Iter: IntoIterator<Item = T>,
    F: Fn(&U) -> K + 'static,
{
    use std::cell::RefCell;
    use std::rc::Rc;

    let result: Rc<RefCell<Vec<Vec<U>>>> = Rc::new(RefCell::new(Vec::new()));
    let current_group: Rc<RefCell<Vec<U>>> = Rc::new(RefCell::new(Vec::new()));
    let current_key: Rc<RefCell<Option<K>>> = Rc::new(RefCell::new(None));

    let reducer = {
        let result = Rc::clone(&result);
        let current_group = Rc::clone(&current_group);
        let current_key = Rc::clone(&current_key);

        move |_acc: (), x: U| {
            let key = key_fn(&x);

            let mut key_ref = current_key.borrow_mut();
            let mut group_ref = current_group.borrow_mut();

            match key_ref.as_ref() {
                None => {
                    // First element
                    *key_ref = Some(key);
                    group_ref.push(x);
                }
                Some(prev_key) if key == *prev_key => {
                    // Same group
                    group_ref.push(x);
                }
                Some(_) => {
                    // New group - save old group and start new one
                    if !group_ref.is_empty() {
                        result.borrow_mut().push(group_ref.clone());
                        group_ref.clear();
                    }
                    *key_ref = Some(key);
                    group_ref.push(x);
                }
            }

            cont(())
        }
    };

    reduce(transducer, source, (), reducer);

    // Don't forget the last group
    let final_group = current_group.borrow().clone();
    if !final_group.is_empty() {
        result.borrow_mut().push(final_group);
    }

    Rc::try_unwrap(result)
        .unwrap_or_else(|_| panic!("Failed to unwrap result"))
        .into_inner()
}

/// Find the top K elements using a min-heap (O(n log k) complexity).
///
/// This is much more efficient than sorting the entire collection when k << n.
/// Uses a binary heap to maintain only the top k elements.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{top_k, transducer::Identity};
///
/// let data = vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3];
/// let id = Identity::new();
/// let top3 = top_k(&id, data, 3);
///
/// assert_eq!(top3.len(), 3);
/// assert!(top3.contains(&9));
/// assert!(top3.contains(&6));
/// assert!(top3.contains(&5));
/// ```
///
/// ```
/// use orlando_transducers::{top_k, Map};
///
/// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// struct Product { sales: i32, name: &'static str }
///
/// let pipeline = Map::new(|p: Product| p);
/// let products = vec![
///     Product { sales: 100, name: "A" },
///     Product { sales: 500, name: "B" },
///     Product { sales: 200, name: "C" },
/// ];
///
/// let top2 = top_k(&pipeline, products, 2);
/// assert_eq!(top2.len(), 2);
/// assert_eq!(top2[0].name, "B"); // Highest sales
/// ```
pub fn top_k<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter, k: usize) -> Vec<U>
where
    T: 'static,
    U: Ord + Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    use std::cell::RefCell;
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    use std::rc::Rc;

    let heap: Rc<RefCell<BinaryHeap<Reverse<U>>>> =
        Rc::new(RefCell::new(BinaryHeap::with_capacity(k + 1)));

    let reducer = {
        let heap = Rc::clone(&heap);

        move |_acc: (), x: U| {
            let mut heap_ref = heap.borrow_mut();
            if heap_ref.len() < k {
                heap_ref.push(Reverse(x));
            } else if let Some(Reverse(min)) = heap_ref.peek() {
                if &x > min {
                    heap_ref.push(Reverse(x));
                    if heap_ref.len() > k {
                        heap_ref.pop();
                    }
                }
            }
            cont(())
        }
    };

    reduce(transducer, source, (), reducer);

    // Extract and reverse (to get descending order)
    let final_heap = Rc::try_unwrap(heap)
        .unwrap_or_else(|_| panic!("Failed to unwrap heap"))
        .into_inner();
    let mut result: Vec<U> = final_heap.into_iter().map(|Reverse(x)| x).collect();
    result.sort_by(|a, b| b.cmp(a)); // Descending order
    result
}

/// Count the frequency of each element.
///
/// Returns a HashMap mapping each unique element to its count.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{frequencies, transducer::Identity};
/// use std::collections::HashMap;
///
/// let data = vec![1, 2, 2, 3, 3, 3];
/// let id = Identity::new();
/// let freq = frequencies(&id, data);
///
/// assert_eq!(freq.get(&1), Some(&1));
/// assert_eq!(freq.get(&2), Some(&2));
/// assert_eq!(freq.get(&3), Some(&3));
/// ```
///
/// ```
/// use orlando_transducers::{frequencies, Map};
///
/// let pipeline = Map::new(|s: &str| s.to_lowercase());
/// let words = vec!["Hello", "World", "hello", "WORLD"];
/// let freq = frequencies(&pipeline, words);
///
/// assert_eq!(freq.get("hello"), Some(&2));
/// assert_eq!(freq.get("world"), Some(&2));
/// ```
pub fn frequencies<T, U, Iter>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
) -> HashMap<U, usize>
where
    T: 'static,
    U: Eq + Hash + Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    let reducer = |mut acc: HashMap<U, usize>, x: U| {
        *acc.entry(x).or_insert(0) += 1;
        cont(acc)
    };

    reduce(transducer, source, HashMap::new(), reducer)
}

/// Zip two iterators, continuing until both are exhausted (unlike `zip`).
///
/// When one iterator is shorter, uses the provided fill value for missing elements.
///
/// # Examples
///
/// ```
/// use orlando_transducers::zip_longest;
///
/// let a = vec![1, 2, 3];
/// let b = vec![4, 5];
/// let result = zip_longest(a, b, 0, 0);
///
/// assert_eq!(result, vec![(1, 4), (2, 5), (3, 0)]);
/// ```
///
/// ```
/// use orlando_transducers::zip_longest;
///
/// let short = vec![1, 2];
/// let long = vec![10, 20, 30, 40];
/// let result = zip_longest(short, long, 999, 0);
///
/// assert_eq!(result, vec![(1, 10), (2, 20), (999, 30), (999, 40)]);
/// ```
pub fn zip_longest<T, U, IterT, IterU>(
    iter_a: IterT,
    iter_b: IterU,
    fill_a: T,
    fill_b: U,
) -> Vec<(T, U)>
where
    T: Clone,
    U: Clone,
    IterT: IntoIterator<Item = T>,
    IterU: IntoIterator<Item = U>,
{
    let mut iter_a = iter_a.into_iter();
    let mut iter_b = iter_b.into_iter();
    let mut result = Vec::new();

    loop {
        match (iter_a.next(), iter_b.next()) {
            (Some(a), Some(b)) => result.push((a, b)),
            (Some(a), None) => result.push((a, fill_b.clone())),
            (None, Some(b)) => result.push((fill_a.clone(), b)),
            (None, None) => break,
        }
    }

    result
}

/// Compute the cartesian product of two iterators.
///
/// Returns all possible pairs (a, b) where a is from the first iterator
/// and b is from the second iterator.
///
/// # Examples
///
/// ```
/// use orlando_transducers::cartesian_product;
///
/// let colors = vec!["red", "blue"];
/// let sizes = vec!["S", "M", "L"];
/// let products = cartesian_product(colors, sizes);
///
/// assert_eq!(products.len(), 6);
/// assert!(products.contains(&("red", "S")));
/// assert!(products.contains(&("blue", "L")));
/// ```
///
/// ```
/// use orlando_transducers::cartesian_product;
///
/// let a = vec![1, 2];
/// let b = vec![3, 4];
/// let result = cartesian_product(a, b);
///
/// assert_eq!(result, vec![(1, 3), (1, 4), (2, 3), (2, 4)]);
/// ```
pub fn cartesian_product<T, U, IterT, IterU>(iter_a: IterT, iter_b: IterU) -> Vec<(T, U)>
where
    T: Clone,
    U: Clone,
    IterT: IntoIterator<Item = T>,
    IterU: IntoIterator<Item = U>,
{
    let vec_a: Vec<T> = iter_a.into_iter().collect();
    let vec_b: Vec<U> = iter_b.into_iter().collect();

    let mut result = Vec::with_capacity(vec_a.len() * vec_b.len());

    for a in &vec_a {
        for b in &vec_b {
            result.push((a.clone(), b.clone()));
        }
    }

    result
}

/// Take the last N elements from a stream.
///
/// This operation requires buffering the entire stream since it needs to know
/// which elements are the "last" ones. It uses a circular buffer for efficiency.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{take_last, transducer::Identity};
///
/// let id = Identity::new();
/// let result = take_last(&id, vec![1, 2, 3, 4, 5], 3);
/// assert_eq!(result, vec![3, 4, 5]);
/// ```
///
/// ```
/// use orlando_transducers::{take_last, Map};
///
/// // Take last 2 after doubling
/// let pipeline = Map::new(|x: i32| x * 2);
/// let result = take_last(&pipeline, vec![1, 2, 3, 4, 5], 2);
/// assert_eq!(result, vec![8, 10]); // Last 2 after doubling
/// ```
///
/// ```
/// use orlando_transducers::{take_last, transducer::Identity};
///
/// // Requesting more than available returns all elements
/// let id = Identity::new();
/// let result = take_last(&id, vec![1, 2, 3], 10);
/// assert_eq!(result, vec![1, 2, 3]);
/// ```
pub fn take_last<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter, n: usize) -> Vec<U>
where
    T: 'static,
    U: Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    // Collect all elements first
    let all_elements = to_vec(transducer, source);

    // Take the last n elements
    if all_elements.len() <= n {
        all_elements
    } else {
        all_elements[all_elements.len() - n..].to_vec()
    }
}

/// Drop the last N elements from a stream.
///
/// This operation requires buffering the entire stream since it needs to know
/// which elements are the "last" ones to drop.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{drop_last, transducer::Identity};
///
/// let id = Identity::new();
/// let result = drop_last(&id, vec![1, 2, 3, 4, 5], 2);
/// assert_eq!(result, vec![1, 2, 3]);
/// ```
///
/// ```
/// use orlando_transducers::{drop_last, Map};
///
/// // Drop last 2 after doubling
/// let pipeline = Map::new(|x: i32| x * 2);
/// let result = drop_last(&pipeline, vec![1, 2, 3, 4, 5], 2);
/// assert_eq!(result, vec![2, 4, 6]); // [2, 4, 6, 8, 10] with last 2 dropped
/// ```
///
/// ```
/// use orlando_transducers::{drop_last, transducer::Identity};
///
/// // Dropping more than available returns empty vector
/// let id = Identity::new();
/// let result = drop_last(&id, vec![1, 2, 3], 10);
/// assert_eq!(result, Vec::<i32>::new());
/// ```
pub fn drop_last<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter, n: usize) -> Vec<U>
where
    T: 'static,
    U: Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    // Collect all elements first
    let all_elements = to_vec(transducer, source);

    // Drop the last n elements
    if n >= all_elements.len() {
        Vec::new()
    } else {
        all_elements[..all_elements.len() - n].to_vec()
    }
}

// ============================================================================
// Phase 4: Aggregation & Statistical Operations
// ============================================================================

/// Multiply all elements together (product).
///
/// Returns the product of all elements after applying the transducer.
/// For empty sequences, returns 1 (multiplicative identity).
///
/// # Examples
///
/// ```
/// use orlando_transducers::{product, transducer::Identity};
///
/// let id = Identity::new();
/// let result = product(&id, vec![2, 3, 4]);
/// assert_eq!(result, 24);
/// ```
///
/// ```
/// use orlando_transducers::{product, transforms::Map};
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = product(&double, vec![1, 2, 3]);
/// assert_eq!(result, 48); // (1*2) * (2*2) * (3*2) = 2 * 4 * 6
/// ```
pub fn product<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> U
where
    T: 'static,
    U: std::ops::Mul<Output = U> + From<u8> + 'static,
    Iter: IntoIterator<Item = T>,
{
    reduce(transducer, source, U::from(1u8), |acc, x| cont(acc * x))
}

/// Calculate the arithmetic mean (average) of elements.
///
/// Returns `None` for empty sequences, otherwise returns `Some(mean)`.
/// All elements are converted to `f64` for the calculation.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{mean, transducer::Identity};
///
/// let id = Identity::new();
/// let result = mean(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
/// assert_eq!(result, Some(3.0));
/// ```
///
/// ```
/// use orlando_transducers::{mean, transforms::Map};
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = mean(&double, vec![1, 2, 3, 4, 5]);
/// assert_eq!(result, Some(6.0)); // Mean of [2, 4, 6, 8, 10]
/// ```
pub fn mean<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<f64>
where
    T: 'static,
    U: Into<f64> + 'static,
    Iter: IntoIterator<Item = T>,
{
    let elements = to_vec(transducer, source);
    if elements.is_empty() {
        None
    } else {
        let len = elements.len();
        let sum: f64 = elements.into_iter().map(|x| x.into()).sum();
        Some(sum / (len as f64))
    }
}

/// Find the median (middle value) of elements.
///
/// Returns `None` for empty sequences. For sequences with an even number of
/// elements, returns the average of the two middle values.
/// Requires sorting, O(n log n) complexity.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{median, transducer::Identity};
///
/// let id = Identity::new();
/// let result = median(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
/// assert_eq!(result, Some(3.0));
/// ```
///
/// ```
/// use orlando_transducers::{median, transducer::Identity};
///
/// // Even number of elements - returns average of middle two
/// let id = Identity::new();
/// let result = median(&id, vec![1.0, 2.0, 3.0, 4.0]);
/// assert_eq!(result, Some(2.5));
/// ```
pub fn median<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<f64>
where
    T: 'static,
    U: Into<f64> + PartialOrd + 'static,
    Iter: IntoIterator<Item = T>,
{
    let elements = to_vec(transducer, source);
    if elements.is_empty() {
        return None;
    }

    let mut values: Vec<f64> = elements.into_iter().map(|x| x.into()).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = values.len();
    if len % 2 == 1 {
        Some(values[len / 2])
    } else {
        Some((values[len / 2 - 1] + values[len / 2]) / 2.0)
    }
}

/// Find the minimum element.
///
/// Returns `None` for empty sequences, otherwise returns `Some(min)`.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{min, transducer::Identity};
///
/// let id = Identity::new();
/// let result = min(&id, vec![3, 1, 4, 1, 5]);
/// assert_eq!(result, Some(1));
/// ```
///
/// ```
/// use orlando_transducers::{min, transforms::Map};
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = min(&double, vec![3, 1, 4, 1, 5]);
/// assert_eq!(result, Some(2)); // Min of [6, 2, 8, 2, 10]
/// ```
pub fn min<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<U>
where
    T: 'static,
    U: Ord + 'static,
    Iter: IntoIterator<Item = T>,
{
    let elements = to_vec(transducer, source);
    elements.into_iter().min()
}

/// Find the maximum element.
///
/// Returns `None` for empty sequences, otherwise returns `Some(max)`.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{max, transducer::Identity};
///
/// let id = Identity::new();
/// let result = max(&id, vec![3, 1, 4, 1, 5]);
/// assert_eq!(result, Some(5));
/// ```
///
/// ```
/// use orlando_transducers::{max, transforms::Map};
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = max(&double, vec![3, 1, 4, 1, 5]);
/// assert_eq!(result, Some(10)); // Max of [6, 2, 8, 2, 10]
/// ```
pub fn max<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<U>
where
    T: 'static,
    U: Ord + 'static,
    Iter: IntoIterator<Item = T>,
{
    let elements = to_vec(transducer, source);
    elements.into_iter().max()
}

/// Find the minimum element by comparing a key extracted from each element.
///
/// Returns `None` for empty sequences, otherwise returns `Some(element)` with
/// the minimum key value.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{min_by, transducer::Identity};
///
/// #[derive(Debug, PartialEq)]
/// struct Product { name: &'static str, price: i32 }
///
/// let id = Identity::new();
/// let products = vec![
///     Product { name: "Apple", price: 100 },
///     Product { name: "Banana", price: 50 },
///     Product { name: "Cherry", price: 150 },
/// ];
///
/// let cheapest = min_by(&id, products, |p| p.price);
/// assert_eq!(cheapest.unwrap().name, "Banana");
/// ```
pub fn min_by<T, U, K, Iter, F>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    key_fn: F,
) -> Option<U>
where
    T: 'static,
    U: 'static,
    K: Ord,
    Iter: IntoIterator<Item = T>,
    F: Fn(&U) -> K,
{
    let elements = to_vec(transducer, source);
    elements.into_iter().min_by_key(key_fn)
}

/// Find the maximum element by comparing a key extracted from each element.
///
/// Returns `None` for empty sequences, otherwise returns `Some(element)` with
/// the maximum key value.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{max_by, transducer::Identity};
///
/// #[derive(Debug, PartialEq)]
/// struct Product { name: &'static str, price: i32 }
///
/// let id = Identity::new();
/// let products = vec![
///     Product { name: "Apple", price: 100 },
///     Product { name: "Banana", price: 50 },
///     Product { name: "Cherry", price: 150 },
/// ];
///
/// let most_expensive = max_by(&id, products, |p| p.price);
/// assert_eq!(most_expensive.unwrap().name, "Cherry");
/// ```
pub fn max_by<T, U, K, Iter, F>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    key_fn: F,
) -> Option<U>
where
    T: 'static,
    U: 'static,
    K: Ord,
    Iter: IntoIterator<Item = T>,
    F: Fn(&U) -> K,
{
    let elements = to_vec(transducer, source);
    elements.into_iter().max_by_key(key_fn)
}

/// Calculate the variance of elements.
///
/// Returns `None` for empty sequences or sequences with only one element,
/// otherwise returns `Some(variance)`.
/// Uses the sample variance formula (dividing by n-1).
///
/// # Examples
///
/// ```
/// use orlando_transducers::{variance, transducer::Identity};
///
/// let id = Identity::new();
/// let result = variance(&id, vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
/// // Variance is approximately 4.571
/// assert!((result.unwrap() - 4.571).abs() < 0.01);
/// ```
pub fn variance<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<f64>
where
    T: 'static,
    U: Into<f64> + Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    let elements = to_vec(transducer, source);
    if elements.len() < 2 {
        return None;
    }

    let values: Vec<f64> = elements.into_iter().map(|x| x.into()).collect();
    let n = values.len() as f64;
    let mean_val = values.iter().sum::<f64>() / n;

    let sum_squared_diff: f64 = values
        .iter()
        .map(|x| {
            let diff = x - mean_val;
            diff * diff
        })
        .sum();

    Some(sum_squared_diff / (n - 1.0))
}

/// Calculate the standard deviation of elements.
///
/// Returns `None` for empty sequences or sequences with only one element,
/// otherwise returns `Some(std_dev)`.
/// Standard deviation is the square root of variance.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{std_dev, transducer::Identity};
///
/// let id = Identity::new();
/// let result = std_dev(&id, vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
/// // Std dev is approximately 2.138
/// assert!((result.unwrap() - 2.138).abs() < 0.01);
/// ```
pub fn std_dev<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<f64>
where
    T: 'static,
    U: Into<f64> + Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    variance(transducer, source).map(|v| v.sqrt())
}

/// Calculate a quantile (percentile) value.
///
/// `p` should be between 0.0 and 1.0, where 0.0 is the minimum,
/// 0.5 is the median, and 1.0 is the maximum.
/// Returns `None` for empty sequences or invalid `p` values.
/// Uses linear interpolation between closest ranks.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{quantile, transducer::Identity};
///
/// let id = Identity::new();
/// // Median (50th percentile)
/// let result = quantile(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0], 0.5);
/// assert_eq!(result, Some(3.0));
/// ```
///
/// ```
/// use orlando_transducers::{quantile, transducer::Identity};
///
/// let id = Identity::new();
/// // 95th percentile
/// let result = quantile(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0], 0.95);
/// assert_eq!(result, Some(4.8));
/// ```
pub fn quantile<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter, p: f64) -> Option<f64>
where
    T: 'static,
    U: Into<f64> + PartialOrd + 'static,
    Iter: IntoIterator<Item = T>,
{
    if !(0.0..=1.0).contains(&p) {
        return None;
    }

    let elements = to_vec(transducer, source);
    if elements.is_empty() {
        return None;
    }

    let mut values: Vec<f64> = elements.into_iter().map(|x| x.into()).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = values.len();
    if len == 1 {
        return Some(values[0]);
    }

    // Linear interpolation
    let index = p * (len - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        Some(values[lower])
    } else {
        let weight = index - lower as f64;
        Some(values[lower] * (1.0 - weight) + values[upper] * weight)
    }
}

/// Find the mode (most frequent element).
///
/// Returns `None` for empty sequences, otherwise returns `Some(mode)`.
/// If multiple elements have the same maximum frequency, returns any one of them.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{mode, transducer::Identity};
///
/// let id = Identity::new();
/// let result = mode(&id, vec![1, 2, 2, 3, 3, 3, 4]);
/// assert_eq!(result, Some(3));
/// ```
///
/// ```
/// use orlando_transducers::{mode, transforms::Map};
///
/// let mod_3 = Map::new(|x: i32| x % 3);
/// let result = mode(&mod_3, vec![3, 6, 9, 12, 1, 2]);
/// // Most common mod 3 value is 0: [3, 6, 9, 12] = 4 times
/// // mod 3 = 1: [1] = 1 time, mod 3 = 2: [2] = 1 time
/// assert_eq!(result, Some(0));
/// ```
pub fn mode<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Option<U>
where
    T: 'static,
    U: Eq + Hash + Clone + 'static,
    Iter: IntoIterator<Item = T>,
{
    let elements = to_vec(transducer, source);
    if elements.is_empty() {
        return None;
    }

    let mut freq_map: HashMap<U, usize> = HashMap::new();
    for elem in elements {
        *freq_map.entry(elem).or_insert(0) += 1;
    }

    freq_map
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(value, _)| value)
}

// ============================================================================
// Phase 5: Collection Utilities & Advanced Helpers
// ============================================================================

// Phase 5a: Sorting & Reversal

/// Sort elements by a key function.
///
/// Returns a new vector with elements sorted according to the key function.
/// This is not a transducer as it requires the full collection to sort.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{sort_by, transducer::Identity};
///
/// #[derive(Debug, Clone, PartialEq)]
/// struct Person { name: &'static str, age: i32 }
///
/// let id = Identity::new();
/// let people = vec![
///     Person { name: "Alice", age: 30 },
///     Person { name: "Bob", age: 25 },
///     Person { name: "Charlie", age: 35 },
/// ];
///
/// let sorted = sort_by(&id, people, |p| p.age);
/// assert_eq!(sorted[0].name, "Bob");
/// assert_eq!(sorted[2].name, "Charlie");
/// ```
pub fn sort_by<T, U, K, Iter, F>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    key_fn: F,
) -> Vec<U>
where
    T: 'static,
    U: Clone + 'static,
    K: Ord,
    Iter: IntoIterator<Item = T>,
    F: Fn(&U) -> K,
{
    let mut elements = to_vec(transducer, source);
    elements.sort_by_key(key_fn);
    elements
}

/// Sort elements with a custom comparator function.
///
/// Returns a new vector with elements sorted according to the comparator.
/// This is not a transducer as it requires the full collection to sort.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{sort_with, transducer::Identity};
/// use std::cmp::Ordering;
///
/// let id = Identity::new();
/// let numbers = vec![3, 1, 4, 1, 5, 9, 2, 6];
///
/// // Sort in descending order
/// let sorted = sort_with(&id, numbers, |a, b| b.cmp(a));
/// assert_eq!(sorted, vec![9, 6, 5, 4, 3, 2, 1, 1]);
/// ```
pub fn sort_with<T, U, Iter, F>(
    transducer: &impl Transducer<T, U>,
    source: Iter,
    comparator: F,
) -> Vec<U>
where
    T: 'static,
    U: Clone + 'static,
    Iter: IntoIterator<Item = T>,
    F: Fn(&U, &U) -> std::cmp::Ordering,
{
    let mut elements = to_vec(transducer, source);
    elements.sort_by(comparator);
    elements
}

/// Reverse the order of elements.
///
/// Returns a new vector with elements in reversed order.
/// This is not a transducer as it requires the full collection.
///
/// # Examples
///
/// ```
/// use orlando_transducers::{reverse, transducer::Identity};
///
/// let id = Identity::new();
/// let result = reverse(&id, vec![1, 2, 3, 4, 5]);
/// assert_eq!(result, vec![5, 4, 3, 2, 1]);
/// ```
///
/// ```
/// use orlando_transducers::{reverse, transforms::Map};
///
/// let double = Map::new(|x: i32| x * 2);
/// let result = reverse(&double, vec![1, 2, 3, 4, 5]);
/// assert_eq!(result, vec![10, 8, 6, 4, 2]);
/// ```
pub fn reverse<T, U, Iter>(transducer: &impl Transducer<T, U>, source: Iter) -> Vec<U>
where
    T: 'static,
    U: 'static,
    Iter: IntoIterator<Item = T>,
{
    let mut elements = to_vec(transducer, source);
    elements.reverse();
    elements
}

// Phase 5b: Generators & Sequences

/// Generate a sequence of numbers from start to end (exclusive) with a given step.
///
/// Similar to Python's range() or JavaScript's Array.from().
///
/// # Examples
///
/// ```
/// use orlando_transducers::range;
///
/// let result = range(0, 10, 1);
/// assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
/// ```
///
/// ```
/// use orlando_transducers::range;
///
/// // Even numbers
/// let evens = range(0, 20, 2);
/// assert_eq!(evens, vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18]);
/// ```
///
/// ```
/// use orlando_transducers::range;
///
/// // Descending
/// let desc = range(10, 0, -1);
/// assert_eq!(desc, vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
/// ```
pub fn range(start: i32, end: i32, step: i32) -> Vec<i32> {
    if step == 0 {
        panic!("Step cannot be zero");
    }

    let mut result = Vec::new();
    let mut current = start;

    if step > 0 {
        while current < end {
            result.push(current);
            current += step;
        }
    } else {
        while current > end {
            result.push(current);
            current += step;
        }
    }

    result
}

/// Repeat a value N times.
///
/// Creates a vector containing N copies of the given value.
///
/// # Examples
///
/// ```
/// use orlando_transducers::repeat;
///
/// let zeros = repeat(0, 5);
/// assert_eq!(zeros, vec![0, 0, 0, 0, 0]);
/// ```
///
/// ```
/// use orlando_transducers::repeat;
///
/// let words = repeat("hello", 3);
/// assert_eq!(words, vec!["hello", "hello", "hello"]);
/// ```
pub fn repeat<T: Clone>(value: T, n: usize) -> Vec<T> {
    vec![value; n]
}

/// Repeat a collection N times.
///
/// Creates a vector by repeating the input collection N times.
///
/// # Examples
///
/// ```
/// use orlando_transducers::cycle;
///
/// let pattern = cycle(vec![1, 2, 3], 3);
/// assert_eq!(pattern, vec![1, 2, 3, 1, 2, 3, 1, 2, 3]);
/// ```
///
/// ```
/// use orlando_transducers::cycle;
///
/// let repeated = cycle(vec!["a", "b"], 2);
/// assert_eq!(repeated, vec!["a", "b", "a", "b"]);
/// ```
pub fn cycle<T: Clone>(vec: Vec<T>, n: usize) -> Vec<T> {
    let mut result = Vec::with_capacity(vec.len() * n);
    for _ in 0..n {
        result.extend(vec.iter().cloned());
    }
    result
}

/// Generate a sequence by repeatedly applying a function to a seed value.
///
/// Similar to Haskell's `unfoldr`. The function returns `Some(next_value)` to continue
/// or `None` to stop generation. The limit parameter prevents infinite loops.
///
/// # Examples
///
/// ```
/// use orlando_transducers::unfold;
///
/// // Generate powers of 2
/// let powers = unfold(1, |x| {
///     let next = x * 2;
///     if next <= 1000 { Some(next) } else { None }
/// }, 20);
/// assert_eq!(powers, vec![2, 4, 8, 16, 32, 64, 128, 256, 512]);
/// ```
///
/// ```
/// use orlando_transducers::unfold;
///
/// // Countdown
/// let countdown = unfold(5, |x| {
///     if *x > 0 { Some(x - 1) } else { None }
/// }, 10);
/// assert_eq!(countdown, vec![4, 3, 2, 1, 0]);
/// ```
pub fn unfold<T, F>(seed: T, f: F, limit: usize) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> Option<T>,
{
    let mut result = Vec::new();
    let mut current = seed;
    let mut count = 0;

    while count < limit {
        match f(&current) {
            Some(next) => {
                result.push(next.clone());
                current = next;
                count += 1;
            }
            None => break,
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::{Filter, Map};

    #[test]
    fn test_to_vec() {
        let double = Map::new(|x: i32| x * 2);
        let result = to_vec(&double, vec![1, 2, 3]);
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_sum() {
        let double = Map::new(|x: i32| x * 2);
        let result = sum(&double, vec![1, 2, 3]);
        assert_eq!(result, 12);
    }

    #[test]
    fn test_count() {
        let evens = Filter::new(|x: &i32| x % 2 == 0);
        let result = count(&evens, vec![1, 2, 3, 4, 5]);
        assert_eq!(result, 2);
    }

    #[test]
    fn test_first() {
        let evens = Filter::new(|x: &i32| x % 2 == 0);
        let result = first(&evens, vec![1, 3, 4, 5]);
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_every() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(every(&id, vec![2, 4, 6], |x| x % 2 == 0));
        assert!(!every(&id, vec![2, 3, 6], |x| x % 2 == 0));
    }

    #[test]
    fn test_partition() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let (evens, odds) = partition(&id, vec![1, 2, 3, 4, 5], |x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_partition_with_transform() {
        // Partition after transformation
        let double = Map::new(|x: i32| x * 2);
        let (greater, lesser) = partition(&double, vec![1, 2, 3, 4, 5], |x| *x > 5);
        assert_eq!(greater, vec![6, 8, 10]); // doubled: 3->6, 4->8, 5->10
        assert_eq!(lesser, vec![2, 4]); // doubled: 1->2, 2->4
    }

    #[test]
    fn test_partition_all_pass() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let (pass, fail) = partition(&id, vec![2, 4, 6], |x| x % 2 == 0);
        assert_eq!(pass, vec![2, 4, 6]);
        assert_eq!(fail, Vec::<i32>::new());
    }

    #[test]
    fn test_partition_all_fail() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let (pass, fail) = partition(&id, vec![1, 3, 5], |x| x % 2 == 0);
        assert_eq!(pass, Vec::<i32>::new());
        assert_eq!(fail, vec![1, 3, 5]);
    }

    #[test]
    fn test_find() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let result = find(&id, vec![1, 3, 4, 5], |x| x % 2 == 0);
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_find_with_transform() {
        let double = Map::new(|x: i32| x * 2);
        let result = find(&double, vec![1, 2, 3, 4, 5], |x| *x > 5);
        assert_eq!(result, Some(6)); // 3*2 = 6, first element >5
    }

    #[test]
    fn test_find_not_found() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let result = find(&id, vec![1, 3, 5, 7], |x| x % 2 == 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_empty() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let result = find(&id, Vec::<i32>::new(), |x| x % 2 == 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_group_by() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let groups = group_by(&id, vec![1, 2, 3, 4, 5, 6], |x| x % 3);

        assert_eq!(groups.get(&0), Some(&vec![3, 6]));
        assert_eq!(groups.get(&1), Some(&vec![1, 4]));
        assert_eq!(groups.get(&2), Some(&vec![2, 5]));
    }

    #[test]
    fn test_group_by_with_transform() {
        // Group after doubling
        let double = Map::new(|x: i32| x * 2);
        let groups = group_by(&double, vec![1, 2, 3, 4, 5, 6], |x| x % 4);

        assert_eq!(groups.get(&0), Some(&vec![4, 8, 12])); // 2*2=4, 4*2=8, 6*2=12
        assert_eq!(groups.get(&2), Some(&vec![2, 6, 10])); // 1*2=2, 3*2=6, 5*2=10
    }

    #[test]
    fn test_group_by_empty() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let groups = group_by(&id, Vec::<i32>::new(), |x| x % 3);

        assert!(groups.is_empty());
    }

    #[test]
    fn test_group_by_single_group() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        let groups = group_by(&id, vec![3, 6, 9, 12], |x| x % 3);

        // All should be in group 0
        assert_eq!(groups.get(&0), Some(&vec![3, 6, 9, 12]));
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_none() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(none(&id, vec![1, 3, 5, 7], |x| x % 2 == 0)); // No evens
        assert!(!none(&id, vec![1, 2, 3], |x| x % 2 == 0)); // Has evens
    }

    #[test]
    fn test_none_empty() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(none(&id, vec![], |_x| true)); // Empty collection = none match
    }

    #[test]
    fn test_none_with_transducer() {
        use crate::transforms::Map;
        let pipeline = Map::new(|x: i32| x * 2);
        assert!(none(&pipeline, vec![1, 2, 3], |x| *x > 10));
        assert!(!none(&pipeline, vec![1, 2, 6], |x| *x > 10)); // 6*2 = 12 > 10
    }

    #[test]
    fn test_none_all_match() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        // None should return false when all elements match
        assert!(!none(&id, vec![2, 4, 6, 8], |x| x % 2 == 0));
    }

    #[test]
    fn test_contains() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(contains(&id, vec![1, 2, 3, 4, 5], &3));
        assert!(!contains(&id, vec![1, 2, 4, 5], &3));
    }

    #[test]
    fn test_contains_empty() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(!contains(&id, vec![], &42));
    }

    #[test]
    fn test_contains_first_element() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(contains(&id, vec![1, 2, 3], &1));
    }

    #[test]
    fn test_contains_last_element() {
        use crate::transducer::Identity;
        let id = Identity::<i32>::new();
        assert!(contains(&id, vec![1, 2, 3, 4, 5], &5));
    }

    #[test]
    fn test_contains_with_transducer() {
        use crate::transforms::Map;
        let pipeline = Map::new(|x: i32| x * 2);
        assert!(contains(&pipeline, vec![1, 2, 3], &4)); // 2 * 2 = 4
        assert!(!contains(&pipeline, vec![1, 2, 3], &5)); // No element maps to 5
    }

    #[test]
    fn test_zip() {
        let a = vec![1, 2, 3];
        let b = vec!['a', 'b', 'c', 'd'];
        let result = zip(a, b);
        assert_eq!(result, vec![(1, 'a'), (2, 'b'), (3, 'c')]);
    }

    #[test]
    fn test_zip_equal_length() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let result = zip(a, b);
        assert_eq!(result, vec![(1, 4), (2, 5), (3, 6)]);
    }

    #[test]
    fn test_zip_with() {
        let a = vec![1, 2, 3];
        let b = vec![10, 20, 30];
        let result = zip_with(a, b, |x, y| x + y);
        assert_eq!(result, vec![11, 22, 33]);
    }

    #[test]
    fn test_zip_with_different_types() {
        let numbers = vec![1, 2, 3];
        let strings = vec!["a", "b", "c"];
        let result = zip_with(numbers, strings, |n, s| format!("{}{}", n, s));
        assert_eq!(result, vec!["1a", "2b", "3c"]);
    }

    // Phase 2a: Multi-Input Operations Tests

    #[test]
    fn test_merge_two_equal_length() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let result = merge(vec![a, b]);
        assert_eq!(result, vec![1, 4, 2, 5, 3, 6]);
    }

    #[test]
    fn test_merge_different_lengths() {
        let a = vec![1, 2];
        let b = vec![3, 4, 5, 6];
        let result = merge(vec![a, b]);
        assert_eq!(result, vec![1, 3, 2, 4, 5, 6]);
    }

    #[test]
    fn test_merge_three_streams() {
        let a = vec![1, 2];
        let b = vec![3, 4];
        let c = vec![5, 6];
        let result = merge(vec![a, b, c]);
        assert_eq!(result, vec![1, 3, 5, 2, 4, 6]);
    }

    #[test]
    fn test_merge_empty_stream() {
        let a = vec![1, 2, 3];
        let b: Vec<i32> = vec![];
        let result = merge(vec![a, b]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_single_stream() {
        let a = vec![1, 2, 3];
        let result = merge(vec![a]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_merge_with_transducers() {
        // Hybrid composition: process streams, then merge
        let pipeline_a = Map::new(|x: i32| x * 2);
        let pipeline_b = Map::new(|x: i32| x + 10);

        let a_result = to_vec(&pipeline_a, vec![1, 2, 3]);
        let b_result = to_vec(&pipeline_b, vec![1, 2, 3]);

        let merged = merge(vec![a_result, b_result]);
        assert_eq!(merged, vec![2, 11, 4, 12, 6, 13]);
    }

    #[test]
    fn test_intersection_basic() {
        let a = vec![1, 2, 3, 4];
        let b = vec![3, 4, 5, 6];
        let result = intersection(a, b);
        assert_eq!(result, vec![3, 4]);
    }

    #[test]
    fn test_intersection_no_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let result: Vec<i32> = intersection(a, b);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_intersection_complete_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3];
        let result = intersection(a, b);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_intersection_preserves_duplicates() {
        let a = vec![1, 2, 2, 3];
        let b = vec![2, 3, 4];
        let result = intersection(a, b);
        assert_eq!(result, vec![2, 2, 3]);
    }

    #[test]
    fn test_intersection_with_transducers() {
        // Hybrid composition: process then intersect
        let pipeline = Map::new(|x: i32| x * 2);
        let a_processed = to_vec(&pipeline, vec![1, 2, 3, 4]);
        let b_processed = to_vec(&pipeline, vec![3, 4, 5, 6]);

        let result = intersection(a_processed, b_processed);
        assert_eq!(result, vec![6, 8]); // 3*2=6, 4*2=8
    }

    #[test]
    fn test_difference_basic() {
        let a = vec![1, 2, 3, 4];
        let b = vec![3, 4, 5, 6];
        let result = difference(a, b);
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_difference_no_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let result = difference(a, b);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_difference_complete_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3];
        let result: Vec<i32> = difference(a, b);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_difference_removes_all_occurrences() {
        let a = vec![1, 2, 2, 3];
        let b = vec![2];
        let result = difference(a, b);
        assert_eq!(result, vec![1, 3]);
    }

    #[test]
    fn test_union_basic() {
        let a = vec![1, 2, 3];
        let b = vec![3, 4, 5];
        let result = union(a, b);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_union_no_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let result = union(a, b);
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_union_complete_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3];
        let result = union(a, b);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_union_removes_duplicates() {
        let a = vec![1, 2, 2, 3];
        let b = vec![3, 4, 4, 5];
        let result = union(a, b);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_union_empty() {
        let a: Vec<i32> = vec![];
        let b = vec![1, 2, 3];
        let result = union(a, b);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_symmetric_difference_basic() {
        let a = vec![1, 2, 3, 4];
        let b = vec![3, 4, 5, 6];
        let result = symmetric_difference(a, b);
        assert_eq!(result, vec![1, 2, 5, 6]);
    }

    #[test]
    fn test_symmetric_difference_no_overlap() {
        let a = vec![1, 2];
        let b = vec![3, 4];
        let result = symmetric_difference(a, b);
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_symmetric_difference_complete_overlap() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3];
        let result: Vec<i32> = symmetric_difference(a, b);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_symmetric_difference_preserves_order() {
        let a = vec![4, 3, 2, 1];
        let b = vec![6, 5, 2, 1];
        let result = symmetric_difference(a, b);
        assert_eq!(result, vec![4, 3, 6, 5]);
    }

    #[test]
    fn test_symmetric_difference_duplicates() {
        let a = vec![1, 1, 2, 3];
        let b = vec![3, 4, 4];
        let result = symmetric_difference(a, b);
        assert_eq!(result, vec![1, 2, 4]);
    }

    // Phase 2b: New operations tests

    #[test]
    fn test_take_last_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec![1, 2, 3, 4, 5], 3);
        assert_eq!(result, vec![3, 4, 5]);
    }

    #[test]
    fn test_take_last_more_than_available() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec![1, 2, 3], 10);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_take_last_zero() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec![1, 2, 3, 4, 5], 0);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_take_last_with_transducer() {
        let pipeline = Map::new(|x: i32| x * 2);
        let result = take_last(&pipeline, vec![1, 2, 3, 4, 5], 2);
        assert_eq!(result, vec![8, 10]); // Last 2 after doubling
    }

    #[test]
    fn test_drop_last_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec![1, 2, 3, 4, 5], 2);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_drop_last_more_than_available() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec![1, 2, 3], 10);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_drop_last_zero() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec![1, 2, 3, 4, 5], 0);
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_drop_last_with_transducer() {
        let pipeline = Map::new(|x: i32| x * 2);
        let result = drop_last(&pipeline, vec![1, 2, 3, 4, 5], 2);
        assert_eq!(result, vec![2, 4, 6]); // [2, 4, 6, 8, 10] with last 2 dropped
    }

    #[test]
    fn test_take_last_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, Vec::<i32>::new(), 5);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_drop_last_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, Vec::<i32>::new(), 5);
        assert_eq!(result, Vec::<i32>::new());
    }

    // Additional comprehensive tests for take_last

    #[test]
    fn test_take_last_one() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec![1, 2, 3, 4, 5], 1);
        assert_eq!(result, vec![5]);
    }

    #[test]
    fn test_take_last_all() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec![1, 2, 3], 3);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_take_last_single_element() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec![42], 1);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_take_last_with_filter() {
        let pipeline = Filter::new(|x: &i32| x % 2 == 0);
        let result = take_last(&pipeline, vec![1, 2, 3, 4, 5, 6, 7, 8], 2);
        assert_eq!(result, vec![6, 8]); // Last 2 evens
    }

    #[test]
    fn test_take_last_strings() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = take_last(&id, vec!["a", "b", "c", "d"], 2);
        assert_eq!(result, vec!["c", "d"]);
    }

    #[test]
    fn test_take_last_large_data() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let data: Vec<i32> = (1..=1000).collect();
        let result = take_last(&id, data, 10);
        assert_eq!(
            result,
            vec![991, 992, 993, 994, 995, 996, 997, 998, 999, 1000]
        );
    }

    #[test]
    fn test_take_last_composed_operations() {
        // Map then take last
        let pipeline = Map::new(|x: i32| x * x);
        let result = take_last(&pipeline, vec![1, 2, 3, 4, 5], 3);
        assert_eq!(result, vec![9, 16, 25]);
    }

    // Additional comprehensive tests for drop_last

    #[test]
    fn test_drop_last_one() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec![1, 2, 3, 4, 5], 1);
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_drop_last_all() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec![1, 2, 3], 3);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_drop_last_single_element() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec![42], 1);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_drop_last_with_filter() {
        let pipeline = Filter::new(|x: &i32| x % 2 == 0);
        let result = drop_last(&pipeline, vec![1, 2, 3, 4, 5, 6, 7, 8], 2);
        assert_eq!(result, vec![2, 4]); // Evens [2,4,6,8], drop last 2 -> [2,4]
    }

    #[test]
    fn test_drop_last_strings() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = drop_last(&id, vec!["a", "b", "c", "d"], 2);
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_drop_last_large_data() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let data: Vec<i32> = (1..=1000).collect();
        let result = drop_last(&id, data, 10);
        assert_eq!(result.len(), 990);
        assert_eq!(result[0], 1);
        assert_eq!(result[989], 990);
    }

    #[test]
    fn test_drop_last_composed_operations() {
        // Map then drop last
        let pipeline = Map::new(|x: i32| x * x);
        let result = drop_last(&pipeline, vec![1, 2, 3, 4, 5], 2);
        assert_eq!(result, vec![1, 4, 9]); // [1,4,9,16,25] drop last 2
    }

    // Combined tests - take_last and drop_last interaction

    #[test]
    fn test_take_last_drop_last_equivalence() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        // take_last 3 should equal dropping first 7
        let taken = take_last(&id, data.clone(), 3);
        let remaining_after_drop_first: Vec<i32> = data.iter().skip(7).cloned().collect();
        assert_eq!(taken, remaining_after_drop_first);
    }

    #[test]
    fn test_drop_last_take_last_coverage() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        // drop_last 3 + take_last 3 should cover all elements
        let dropped = drop_last(&id, data.clone(), 3);
        let taken = take_last(&id, data, 3);

        assert_eq!(dropped.len() + taken.len(), 10);
        assert_eq!(dropped, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(taken, vec![8, 9, 10]);
    }

    // ============================================================================
    // Phase 4: Aggregation & Statistical Operations Tests
    // ============================================================================

    #[test]
    fn test_product_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = product(&id, vec![2, 3, 4]);
        assert_eq!(result, 24);
    }

    #[test]
    fn test_product_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result: i32 = product(&id, Vec::<i32>::new());
        assert_eq!(result, 1); // Multiplicative identity
    }

    #[test]
    fn test_product_with_map() {
        let double = Map::new(|x: i32| x * 2);
        let result = product(&double, vec![1, 2, 3]);
        assert_eq!(result, 48); // 2 * 4 * 6
    }

    #[test]
    fn test_product_with_zero() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = product(&id, vec![1, 2, 0, 3, 4]);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_mean_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mean(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(result, Some(3.0));
    }

    #[test]
    fn test_mean_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mean(&id, Vec::<f64>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_mean_integers() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mean(&id, vec![1, 2, 3, 4, 5]);
        assert_eq!(result, Some(3.0));
    }

    #[test]
    fn test_mean_with_map() {
        let double = Map::new(|x: i32| (x * 2) as f64);
        let result = mean(&double, vec![1, 2, 3, 4, 5]);
        assert_eq!(result, Some(6.0)); // Mean of [2, 4, 6, 8, 10]
    }

    #[test]
    fn test_median_odd_count() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = median(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(result, Some(3.0));
    }

    #[test]
    fn test_median_even_count() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = median(&id, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(result, Some(2.5));
    }

    #[test]
    fn test_median_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = median(&id, Vec::<f64>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_median_single() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = median(&id, vec![42.0]);
        assert_eq!(result, Some(42.0));
    }

    #[test]
    fn test_median_unsorted() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = median(&id, vec![5.0, 1.0, 3.0, 2.0, 4.0]);
        assert_eq!(result, Some(3.0));
    }

    #[test]
    fn test_min_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = min(&id, vec![3, 1, 4, 1, 5]);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_min_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = min(&id, Vec::<i32>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_min_with_map() {
        let double = Map::new(|x: i32| x * 2);
        let result = min(&double, vec![3, 1, 4, 1, 5]);
        assert_eq!(result, Some(2)); // Min of [6, 2, 8, 2, 10]
    }

    #[test]
    fn test_max_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = max(&id, vec![3, 1, 4, 1, 5]);
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_max_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = max(&id, Vec::<i32>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_max_with_map() {
        let double = Map::new(|x: i32| x * 2);
        let result = max(&double, vec![3, 1, 4, 1, 5]);
        assert_eq!(result, Some(10)); // Max of [6, 2, 8, 2, 10]
    }

    #[test]
    fn test_min_by_basic() {
        use crate::transducer::Identity;
        #[derive(Debug, PartialEq, Clone)]
        struct Product {
            name: &'static str,
            price: i32,
        }

        let id = Identity::new();
        let products = vec![
            Product {
                name: "Apple",
                price: 100,
            },
            Product {
                name: "Banana",
                price: 50,
            },
            Product {
                name: "Cherry",
                price: 150,
            },
        ];

        let cheapest = min_by(&id, products, |p| p.price);
        assert_eq!(cheapest.unwrap().name, "Banana");
    }

    #[test]
    fn test_min_by_empty() {
        use crate::transducer::Identity;
        #[derive(Debug, PartialEq)]
        struct Item {
            value: i32,
        }

        let id = Identity::new();
        let result = min_by(&id, Vec::<Item>::new(), |item| item.value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_max_by_basic() {
        use crate::transducer::Identity;
        #[derive(Debug, PartialEq, Clone)]
        struct Product {
            name: &'static str,
            price: i32,
        }

        let id = Identity::new();
        let products = vec![
            Product {
                name: "Apple",
                price: 100,
            },
            Product {
                name: "Banana",
                price: 50,
            },
            Product {
                name: "Cherry",
                price: 150,
            },
        ];

        let most_expensive = max_by(&id, products, |p| p.price);
        assert_eq!(most_expensive.unwrap().name, "Cherry");
    }

    #[test]
    fn test_max_by_empty() {
        use crate::transducer::Identity;
        #[derive(Debug, PartialEq)]
        struct Item {
            value: i32,
        }

        let id = Identity::new();
        let result = max_by(&id, Vec::<Item>::new(), |item| item.value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_variance_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = variance(&id, vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        // Variance is approximately 4.571
        assert!((result.unwrap() - 4.571).abs() < 0.01);
    }

    #[test]
    fn test_variance_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = variance(&id, Vec::<f64>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_variance_single_element() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = variance(&id, vec![42.0]);
        assert_eq!(result, None); // Need at least 2 elements for sample variance
    }

    #[test]
    fn test_variance_two_elements() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = variance(&id, vec![1.0, 3.0]);
        // Variance of [1, 3] is 2.0
        assert!((result.unwrap() - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_std_dev_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = std_dev(&id, vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        // Std dev is approximately 2.138
        assert!((result.unwrap() - 2.138).abs() < 0.01);
    }

    #[test]
    fn test_std_dev_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = std_dev(&id, Vec::<f64>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_std_dev_constant() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = std_dev(&id, vec![5.0, 5.0, 5.0, 5.0]);
        // Std dev of constant values is 0
        assert!((result.unwrap() - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_quantile_median() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = quantile(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0], 0.5);
        assert_eq!(result, Some(3.0));
    }

    #[test]
    fn test_quantile_min() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = quantile(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0], 0.0);
        assert_eq!(result, Some(1.0));
    }

    #[test]
    fn test_quantile_max() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = quantile(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0], 1.0);
        assert_eq!(result, Some(5.0));
    }

    #[test]
    fn test_quantile_p95() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = quantile(&id, vec![1.0, 2.0, 3.0, 4.0, 5.0], 0.95);
        assert_eq!(result, Some(4.8));
    }

    #[test]
    fn test_quantile_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = quantile(&id, Vec::<f64>::new(), 0.5);
        assert_eq!(result, None);
    }

    #[test]
    fn test_quantile_invalid_p() {
        use crate::transducer::Identity;
        let id = Identity::new();
        assert_eq!(quantile(&id, vec![1.0, 2.0, 3.0], -0.1), None);
        assert_eq!(quantile(&id, vec![1.0, 2.0, 3.0], 1.5), None);
    }

    #[test]
    fn test_quantile_single_element() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = quantile(&id, vec![42.0], 0.5);
        assert_eq!(result, Some(42.0));
    }

    #[test]
    fn test_mode_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mode(&id, vec![1, 2, 2, 3, 3, 3, 4]);
        assert_eq!(result, Some(3));
    }

    #[test]
    fn test_mode_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mode(&id, Vec::<i32>::new());
        assert_eq!(result, None);
    }

    #[test]
    fn test_mode_all_unique() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mode(&id, vec![1, 2, 3, 4, 5]);
        // All have frequency 1, should return one of them
        assert!(result.is_some());
        assert!([1, 2, 3, 4, 5].contains(&result.unwrap()));
    }

    #[test]
    fn test_mode_with_map() {
        let mod_3 = Map::new(|x: i32| x % 3);
        // Use data where 0 clearly appears most frequently
        let result = mode(&mod_3, vec![3, 6, 9, 12, 1, 2]);
        // 0 appears 4 times (3, 6, 9, 12), while 1 and 2 appear once each
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_mode_strings() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = mode(&id, vec!["a", "b", "a", "c", "a", "b"]);
        assert_eq!(result, Some("a"));
    }

    // Integration tests - combining Phase 4 operations

    #[test]
    fn test_statistical_operations_pipeline() {
        // Test that Phase 4 operations work well with transducers
        use crate::transducer::Identity;
        let id = Identity::new();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(mean(&id, data.clone()), Some(5.5));
        assert_eq!(median(&id, data.clone()), Some(5.5));
        assert_eq!(min(&id, data.clone()), Some(1));
        assert_eq!(max(&id, data.clone()), Some(10));
    }

    #[test]
    fn test_phase4_with_filter() {
        // Filter even numbers, then compute statistics
        let pipeline = Filter::new(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(count(&pipeline, data.clone()), 5);
        assert_eq!(sum(&pipeline, data.clone()), 30); // 2+4+6+8+10
        assert_eq!(product(&pipeline, data.clone()), 3840); // 2*4*6*8*10
        assert_eq!(mean(&pipeline, data.clone()), Some(6.0));
        assert_eq!(min(&pipeline, data.clone()), Some(2));
        assert_eq!(max(&pipeline, data.clone()), Some(10));
    }

    // ============================================================================
    // Phase 5: Collection Utilities & Advanced Helpers Tests
    // ============================================================================

    // Phase 5a: Sorting & Reversal Tests

    #[test]
    fn test_sort_by_basic() {
        use crate::transducer::Identity;
        #[derive(Debug, Clone, PartialEq)]
        struct Person {
            name: &'static str,
            age: i32,
        }

        let id = Identity::new();
        let people = vec![
            Person {
                name: "Alice",
                age: 30,
            },
            Person {
                name: "Bob",
                age: 25,
            },
            Person {
                name: "Charlie",
                age: 35,
            },
        ];

        let sorted = sort_by(&id, people, |p| p.age);
        assert_eq!(sorted[0].name, "Bob");
        assert_eq!(sorted[1].name, "Alice");
        assert_eq!(sorted[2].name, "Charlie");
    }

    #[test]
    fn test_sort_by_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result: Vec<i32> = sort_by(&id, Vec::<i32>::new(), |x| *x);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_sort_by_with_map() {
        let pipeline = Map::new(|x: i32| x * x);
        let result = sort_by(&pipeline, vec![3, 1, 4, 2], |x| *x);
        assert_eq!(result, vec![1, 4, 9, 16]);
    }

    #[test]
    fn test_sort_with_descending() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let numbers = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let sorted = sort_with(&id, numbers, |a, b| b.cmp(a));
        assert_eq!(sorted, vec![9, 6, 5, 4, 3, 2, 1, 1]);
    }

    #[test]
    fn test_sort_with_ascending() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let numbers = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let sorted = sort_with(&id, numbers, |a, b| a.cmp(b));
        assert_eq!(sorted, vec![1, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn test_reverse_basic() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = reverse(&id, vec![1, 2, 3, 4, 5]);
        assert_eq!(result, vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_reverse_empty() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result: Vec<i32> = reverse(&id, Vec::<i32>::new());
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_reverse_single() {
        use crate::transducer::Identity;
        let id = Identity::new();
        let result = reverse(&id, vec![42]);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_reverse_with_map() {
        let double = Map::new(|x: i32| x * 2);
        let result = reverse(&double, vec![1, 2, 3, 4, 5]);
        assert_eq!(result, vec![10, 8, 6, 4, 2]);
    }

    // Phase 5b: Generators & Sequences Tests

    #[test]
    fn test_range_basic() {
        let result = range(0, 10, 1);
        assert_eq!(result, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_range_step_2() {
        let evens = range(0, 20, 2);
        assert_eq!(evens, vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18]);
    }

    #[test]
    fn test_range_descending() {
        let desc = range(10, 0, -1);
        assert_eq!(desc, vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_range_empty() {
        let result = range(0, 0, 1);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_range_negative_step() {
        let result = range(5, 0, -2);
        assert_eq!(result, vec![5, 3, 1]);
    }

    #[test]
    #[should_panic(expected = "Step cannot be zero")]
    fn test_range_zero_step() {
        range(0, 10, 0);
    }

    #[test]
    fn test_repeat_basic() {
        let zeros = repeat(0, 5);
        assert_eq!(zeros, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_repeat_strings() {
        let words = repeat("hello", 3);
        assert_eq!(words, vec!["hello", "hello", "hello"]);
    }

    #[test]
    fn test_repeat_zero() {
        let result: Vec<i32> = repeat(42, 0);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_cycle_basic() {
        let pattern = cycle(vec![1, 2, 3], 3);
        assert_eq!(pattern, vec![1, 2, 3, 1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn test_cycle_strings() {
        let repeated = cycle(vec!["a", "b"], 2);
        assert_eq!(repeated, vec!["a", "b", "a", "b"]);
    }

    #[test]
    fn test_cycle_zero() {
        let result: Vec<i32> = cycle(vec![1, 2, 3], 0);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_cycle_empty_vec() {
        let result: Vec<i32> = cycle(Vec::new(), 5);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_unfold_powers_of_2() {
        let powers = unfold(
            1,
            |x| {
                let next = x * 2;
                if next <= 1000 {
                    Some(next)
                } else {
                    None
                }
            },
            20,
        );
        assert_eq!(powers, vec![2, 4, 8, 16, 32, 64, 128, 256, 512]);
    }

    #[test]
    fn test_unfold_countdown() {
        let countdown = unfold(5, |x| if *x > 0 { Some(x - 1) } else { None }, 10);
        assert_eq!(countdown, vec![4, 3, 2, 1, 0]);
    }

    #[test]
    fn test_unfold_limit() {
        // Even if function doesn't return None, limit should stop it
        let result = unfold(1, |x| Some(x + 1), 5);
        assert_eq!(result, vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_unfold_early_stop() {
        // Function returns None immediately
        let result: Vec<i32> = unfold(1, |_x| None, 10);
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_unfold_fibonacci() {
        let fibonacci = unfold(
            (0, 1),
            |(a, b)| {
                let next = (*b, a + b);
                if next.0 <= 100 {
                    Some(next)
                } else {
                    None
                }
            },
            20,
        );
        let fib_numbers: Vec<i32> = fibonacci.iter().map(|(a, _)| *a).collect();
        assert_eq!(fib_numbers, vec![1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]);
    }

    // Integration tests

    #[test]
    fn test_phase5_integration() {
        use crate::transducer::Identity;
        let id = Identity::new();

        // Generate range, filter evens, sort descending, reverse
        let numbers = range(1, 11, 1);
        let pipeline = Filter::new(|x: &i32| x % 2 == 0);
        let evens = to_vec(&pipeline, numbers);
        let sorted = sort_with(&id, evens, |a, b| b.cmp(a));
        let reversed = reverse(&id, sorted);

        assert_eq!(reversed, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_phase5_cycle_with_transducer() {
        // Cycle pattern, then transform with map
        let pattern = cycle(vec![1, 2, 3], 2);
        let double = Map::new(|x: i32| x * 2);
        let result = to_vec(&double, pattern);
        assert_eq!(result, vec![2, 4, 6, 2, 4, 6]);
    }
}
