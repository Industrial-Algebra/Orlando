//! Logic functions for predicate composition and conditional transformations.
//!
//! This module provides utilities for combining predicates and creating conditional
//! transformations, inspired by Ramda's logic functions.

use crate::describe::{Describable, StageSpec};
use crate::step::Step;
use crate::transducer::Transducer;
use std::marker::PhantomData;
use std::rc::Rc;

// ========================================
// Type Aliases
// ========================================

/// Type alias for a boxed predicate function
pub type BoxedPredicate<T> = Box<dyn Fn(&T) -> bool>;

/// Type alias for a vector of boxed predicates
pub type PredicateVec<T> = Vec<BoxedPredicate<T>>;

// ========================================
// Predicate Combinators
// ========================================

/// Combines two predicates with AND logic.
///
/// Returns a new predicate that is true only when both input predicates are true.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::both;
///
/// let is_positive = |x: &i32| *x > 0;
/// let is_even = |x: &i32| x % 2 == 0;
/// let is_positive_even = both(is_positive, is_even);
///
/// assert!(is_positive_even(&4));
/// assert!(!is_positive_even(&3));
/// assert!(!is_positive_even(&-2));
/// ```
pub fn both<T, P1, P2>(pred1: P1, pred2: P2) -> impl Fn(&T) -> bool
where
    P1: Fn(&T) -> bool,
    P2: Fn(&T) -> bool,
{
    move |x| pred1(x) && pred2(x)
}

/// Combines two predicates with OR logic.
///
/// Returns a new predicate that is true when either input predicate is true.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::either;
///
/// let is_small = |x: &i32| *x < 10;
/// let is_large = |x: &i32| *x > 100;
/// let is_extreme = either(is_small, is_large);
///
/// assert!(is_extreme(&5));
/// assert!(is_extreme(&200));
/// assert!(!is_extreme(&50));
/// ```
pub fn either<T, P1, P2>(pred1: P1, pred2: P2) -> impl Fn(&T) -> bool
where
    P1: Fn(&T) -> bool,
    P2: Fn(&T) -> bool,
{
    move |x| pred1(x) || pred2(x)
}

/// Negates a predicate.
///
/// Returns a new predicate that returns true when the input predicate returns false,
/// and vice versa.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::complement;
///
/// let is_even = |x: &i32| x % 2 == 0;
/// let is_odd = complement(is_even);
///
/// assert!(is_odd(&3));
/// assert!(!is_odd(&4));
/// ```
pub fn complement<T, P>(pred: P) -> impl Fn(&T) -> bool
where
    P: Fn(&T) -> bool,
{
    move |x| !pred(x)
}

/// Combines multiple predicates with AND logic.
///
/// Returns a new predicate that is true only when ALL input predicates are true.
/// Short-circuits on the first false predicate.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::{all_pass, PredicateVec};
///
/// let predicates: PredicateVec<i32> = vec![
///     Box::new(|x: &i32| *x > 0),
///     Box::new(|x: &i32| *x < 100),
///     Box::new(|x: &i32| x % 2 == 0),
/// ];
///
/// let is_valid = all_pass(predicates);
///
/// assert!(is_valid(&50));  // positive, < 100, even
/// assert!(!is_valid(&3));  // positive, < 100, but odd
/// assert!(!is_valid(&150)); // positive, even, but >= 100
/// ```
pub fn all_pass<T>(predicates: PredicateVec<T>) -> impl Fn(&T) -> bool {
    move |x| predicates.iter().all(|pred| pred(x))
}

/// Combines multiple predicates with OR logic.
///
/// Returns a new predicate that is true when ANY input predicate is true.
/// Short-circuits on the first true predicate.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::{any_pass, PredicateVec};
///
/// let predicates: PredicateVec<i32> = vec![
///     Box::new(|x: &i32| *x == 0),
///     Box::new(|x: &i32| x % 10 == 0),
///     Box::new(|x: &i32| *x > 1000),
/// ];
///
/// let is_special = any_pass(predicates);
///
/// assert!(is_special(&0));    // equals 0
/// assert!(is_special(&50));   // divisible by 10
/// assert!(is_special(&2000)); // > 1000
/// assert!(!is_special(&7));   // none of the above
/// ```
pub fn any_pass<T>(predicates: PredicateVec<T>) -> impl Fn(&T) -> bool {
    move |x| predicates.iter().any(|pred| pred(x))
}

// ========================================
// Conditional Transducers
// ========================================

/// Conditional transformation - applies transform only when predicate is true.
///
/// When the predicate is false, the value passes through unchanged.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::When;
/// use orlando_transducers::collectors::to_vec;
///
/// let double_if_positive = When::new(|x: &i32| *x > 0, |x: i32| x * 2);
/// let result = to_vec(&double_if_positive, vec![-1, 2, -3, 4]);
/// assert_eq!(result, vec![-1, 4, -3, 8]);
/// ```
pub struct When<P, F, T> {
    predicate: Rc<P>,
    transform: Rc<F>,
    _phantom: PhantomData<T>,
}

impl<P, F, T> When<P, F, T>
where
    P: Fn(&T) -> bool,
    F: Fn(T) -> T,
{
    pub fn new(predicate: P, transform: F) -> Self {
        When {
            predicate: Rc::new(predicate),
            transform: Rc::new(transform),
            _phantom: PhantomData,
        }
    }
}

impl<P, F, T> Transducer<T, T> for When<P, F, T>
where
    P: Fn(&T) -> bool + 'static,
    F: Fn(T) -> T + 'static,
    T: Clone + 'static,
{
    #[inline(always)]
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, T) -> Step<Acc>>
    where
        R: Fn(Acc, T) -> Step<Acc> + 'static,
        Acc: 'static,
    {
        let predicate = Rc::clone(&self.predicate);
        let transform = Rc::clone(&self.transform);

        Box::new(move |acc, val| {
            if predicate(&val) {
                reducer(acc, transform(val))
            } else {
                reducer(acc, val)
            }
        })
    }
}

/// Conditional transformation - applies transform only when predicate is false.
///
/// When the predicate is true, the value passes through unchanged.
/// This is the inverse of `When`.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::Unless;
/// use orlando_transducers::collectors::to_vec;
///
/// let zero_if_negative = Unless::new(|x: &i32| *x > 0, |_| 0);
/// let result = to_vec(&zero_if_negative, vec![-1, 2, -3, 4]);
/// assert_eq!(result, vec![0, 2, 0, 4]);
/// ```
pub struct Unless<P, F, T> {
    predicate: Rc<P>,
    transform: Rc<F>,
    _phantom: PhantomData<T>,
}

impl<P, F, T> Unless<P, F, T>
where
    P: Fn(&T) -> bool,
    F: Fn(T) -> T,
{
    pub fn new(predicate: P, transform: F) -> Self {
        Unless {
            predicate: Rc::new(predicate),
            transform: Rc::new(transform),
            _phantom: PhantomData,
        }
    }
}

impl<P, F, T> Transducer<T, T> for Unless<P, F, T>
where
    P: Fn(&T) -> bool + 'static,
    F: Fn(T) -> T + 'static,
    T: Clone + 'static,
{
    #[inline(always)]
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, T) -> Step<Acc>>
    where
        R: Fn(Acc, T) -> Step<Acc> + 'static,
        Acc: 'static,
    {
        let predicate = Rc::clone(&self.predicate);
        let transform = Rc::clone(&self.transform);

        Box::new(move |acc, val| {
            if !predicate(&val) {
                reducer(acc, transform(val))
            } else {
                reducer(acc, val)
            }
        })
    }
}

/// Branch on condition - applies different transforms based on predicate.
///
/// If the predicate is true, applies `on_true` transform.
/// If the predicate is false, applies `on_false` transform.
///
/// # Examples
///
/// ```
/// use orlando_transducers::logic::IfElse;
/// use orlando_transducers::collectors::to_vec;
///
/// let abs_with_sign = IfElse::new(
///     |x: &i32| *x >= 0,
///     |x: i32| x * 2,      // double if positive
///     |x: i32| x / 2       // halve if negative
/// );
/// let result = to_vec(&abs_with_sign, vec![-4, 3, -6, 5]);
/// assert_eq!(result, vec![-2, 6, -3, 10]);
/// ```
pub struct IfElse<P, F1, F2, T> {
    predicate: Rc<P>,
    on_true: Rc<F1>,
    on_false: Rc<F2>,
    _phantom: PhantomData<T>,
}

impl<P, F1, F2, T> IfElse<P, F1, F2, T>
where
    P: Fn(&T) -> bool,
    F1: Fn(T) -> T,
    F2: Fn(T) -> T,
{
    pub fn new(predicate: P, on_true: F1, on_false: F2) -> Self {
        IfElse {
            predicate: Rc::new(predicate),
            on_true: Rc::new(on_true),
            on_false: Rc::new(on_false),
            _phantom: PhantomData,
        }
    }
}

impl<P, F1, F2, T> Transducer<T, T> for IfElse<P, F1, F2, T>
where
    P: Fn(&T) -> bool + 'static,
    F1: Fn(T) -> T + 'static,
    F2: Fn(T) -> T + 'static,
    T: Clone + 'static,
{
    #[inline(always)]
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, T) -> Step<Acc>>
    where
        R: Fn(Acc, T) -> Step<Acc> + 'static,
        Acc: 'static,
    {
        let predicate = Rc::clone(&self.predicate);
        let on_true = Rc::clone(&self.on_true);
        let on_false = Rc::clone(&self.on_false);

        Box::new(move |acc, val| {
            if predicate(&val) {
                reducer(acc, on_true(val))
            } else {
                reducer(acc, on_false(val))
            }
        })
    }
}

// ===========================================================================
// Reflection: conditional transducers describe themselves as StageSpecs.
// ===========================================================================

impl<P, F, T> Describable for When<P, F, T> {
    fn describe_into(&self, out: &mut Vec<StageSpec>) {
        out.push(StageSpec::When);
    }
}

impl<P, F, T> Describable for Unless<P, F, T> {
    fn describe_into(&self, out: &mut Vec<StageSpec>) {
        out.push(StageSpec::Unless);
    }
}

impl<P, F1, F2, T> Describable for IfElse<P, F1, F2, T> {
    fn describe_into(&self, out: &mut Vec<StageSpec>) {
        out.push(StageSpec::IfElse);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collectors::to_vec;

    #[test]
    fn test_both() {
        let is_positive = |x: &i32| *x > 0;
        let is_even = |x: &i32| x % 2 == 0;
        let is_positive_even = both(is_positive, is_even);

        assert!(is_positive_even(&4));
        assert!(!is_positive_even(&3));
        assert!(!is_positive_even(&-2));
    }

    #[test]
    fn test_either() {
        let is_small = |x: &i32| *x < 10;
        let is_large = |x: &i32| *x > 100;
        let is_extreme = either(is_small, is_large);

        assert!(is_extreme(&5));
        assert!(is_extreme(&200));
        assert!(!is_extreme(&50));
    }

    #[test]
    fn test_complement() {
        let is_even = |x: &i32| x % 2 == 0;
        let is_odd = complement(is_even);

        assert!(is_odd(&3));
        assert!(!is_odd(&4));
    }

    #[test]
    fn test_all_pass() {
        let predicates: PredicateVec<i32> = vec![
            Box::new(|x: &i32| *x > 0),
            Box::new(|x: &i32| *x < 100),
            Box::new(|x: &i32| x % 2 == 0),
        ];

        let is_valid = all_pass(predicates);

        assert!(is_valid(&50));
        assert!(!is_valid(&3));
        assert!(!is_valid(&150));
    }

    #[test]
    fn test_any_pass() {
        let predicates: PredicateVec<i32> = vec![
            Box::new(|x: &i32| *x == 0),
            Box::new(|x: &i32| x % 10 == 0),
            Box::new(|x: &i32| *x > 1000),
        ];

        let is_special = any_pass(predicates);

        assert!(is_special(&0));
        assert!(is_special(&50));
        assert!(is_special(&2000));
        assert!(!is_special(&7));
    }

    #[test]
    fn test_when() {
        let double_if_positive = When::new(|x: &i32| *x > 0, |x: i32| x * 2);
        let result = to_vec(&double_if_positive, vec![-1, 2, -3, 4]);
        assert_eq!(result, vec![-1, 4, -3, 8]);
    }

    #[test]
    fn test_unless() {
        let zero_if_negative = Unless::new(|x: &i32| *x > 0, |_| 0);
        let result = to_vec(&zero_if_negative, vec![-1, 2, -3, 4]);
        assert_eq!(result, vec![0, 2, 0, 4]);
    }

    #[test]
    fn test_if_else() {
        let abs_with_sign = IfElse::new(|x: &i32| *x >= 0, |x: i32| x * 2, |x: i32| x / 2);
        let result = to_vec(&abs_with_sign, vec![-4, 3, -6, 5]);
        assert_eq!(result, vec![-2, 6, -3, 10]);
    }
}
