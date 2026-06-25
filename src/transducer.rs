//! Core transducer trait and composition.
//!
//! # Category Theory
//!
//! Transducers are natural transformations between fold functors. Given a reducing
//! function `R: (Acc, Out) -> Acc`, a transducer transforms it into a new reducing
//! function `(Acc, In) -> Acc`.
//!
//! Formally, a transducer `T: In ~> Out` is a polymorphic function:
//!
//! ```text
//! ∀Acc. ((Acc, Out) -> Acc) -> ((Acc, In) -> Acc)
//! ```
//!
//! Transducers form a category where:
//! - Objects are types
//! - Morphisms are transducers
//! - Composition is transducer composition
//! - Identity is the identity transducer
//!
//! The category laws hold:
//! - Left identity: `id().compose(t) == t`
//! - Right identity: `t.compose(id()) == t`
//! - Associativity: `(t1.compose(t2)).compose(t3) == t1.compose(t2.compose(t3))`

use crate::describe::{Describable, StageSpec};
use crate::step::Step;
use std::marker::PhantomData;

/// A transducer transforms reducing functions.
///
/// # Type Parameters
///
/// - `In`: The input element type this transducer consumes
/// - `Out`: The output element type this transducer produces
///
/// # Examples
///
/// ```
/// use orlando_transducers::transducer::Transducer;
/// use orlando_transducers::step::{Step, cont};
///
/// // Identity transducer - passes values through unchanged
/// let id = orlando_transducers::transducer::Identity::<i32>::new();
/// ```
pub trait Transducer<In, Out>: Sized {
    /// Apply this transducer to a reducing function.
    ///
    /// This transforms a reducer that consumes `Out` into one that consumes `In`.
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, In) -> Step<Acc>>
    where
        R: Fn(Acc, Out) -> Step<Acc> + 'static,
        Acc: 'static,
        In: 'static,
        Out: 'static;

    /// Compose this transducer with another.
    ///
    /// Creates a new transducer that applies `self` first, then `other`.
    fn compose<Out2, T>(self, other: T) -> Compose<Self, T, In, Out, Out2>
    where
        T: Transducer<Out, Out2>,
        Self: 'static,
        Out: 'static,
        Out2: 'static,
        T: 'static,
    {
        Compose {
            first: self,
            second: other,
            _phantom: PhantomData,
        }
    }
}

/// The identity transducer - passes values through unchanged.
///
/// # Category Theory
///
/// This is the identity morphism in the transducer category.
pub struct Identity<T> {
    _phantom: PhantomData<T>,
}

impl<T> Identity<T> {
    pub fn new() -> Self {
        Identity {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for Identity<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Transducer<T, T> for Identity<T> {
    #[inline(always)]
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, T) -> Step<Acc>>
    where
        R: Fn(Acc, T) -> Step<Acc> + 'static,
        Acc: 'static,
        T: 'static,
    {
        Box::new(reducer)
    }
}

/// Composition of two transducers.
///
/// # Category Theory
///
/// This represents categorical composition in the transducer category.
/// If `T1: A ~> B` and `T2: B ~> C`, then `Compose<T1, T2>: A ~> C`.
pub struct Compose<T1, T2, In, Mid, Out> {
    first: T1,
    second: T2,
    _phantom: PhantomData<(In, Mid, Out)>,
}

impl<T1, T2, In, Mid, Out> Transducer<In, Out> for Compose<T1, T2, In, Mid, Out>
where
    T1: Transducer<In, Mid>,
    T2: Transducer<Mid, Out>,
    In: 'static,
    Mid: 'static,
    Out: 'static,
{
    #[inline(always)]
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, In) -> Step<Acc>>
    where
        R: Fn(Acc, Out) -> Step<Acc> + 'static,
        Acc: 'static,
    {
        // Compose right-to-left: first apply second, then first
        let r2 = self.second.apply(reducer);
        self.first.apply(r2)
    }
}

// ===========================================================================
// Reflection: Identity and Compose describe themselves as StageSpec lists.
// ===========================================================================

impl<T> Describable for Identity<T> {
    fn describe_into(&self, out: &mut Vec<StageSpec>) {
        out.push(StageSpec::Identity);
    }
}

impl<T1, T2, In, Mid, Out> Describable for Compose<T1, T2, In, Mid, Out>
where
    T1: Describable,
    T2: Describable,
{
    fn describe_into(&self, out: &mut Vec<StageSpec>) {
        // Data flows `first` -> `second` (see `apply`), so describe in that order.
        self.first.describe_into(out);
        self.second.describe_into(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::cont;

    #[test]
    fn test_identity() {
        let id = Identity::<i32>::new();
        let reducer = |acc: Vec<i32>, x: i32| {
            let mut v = acc;
            v.push(x);
            cont(v)
        };

        let transformed = id.apply(reducer);
        let result = transformed(vec![], 42);
        assert_eq!(result.unwrap(), vec![42]);
    }

    #[test]
    fn test_identity_laws() {
        // Identity should pass values through unchanged
        let id = Identity::<i32>::new();
        let reducer = |acc: i32, x: i32| cont(acc + x);

        let transformed = id.apply(reducer);
        assert_eq!(transformed(0, 5).unwrap(), 5);
        assert_eq!(transformed(10, 5).unwrap(), 15);
    }
}
