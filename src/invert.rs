//! The invertible transducer groupoid (Layer A of the inverse-transducer design).
//!
//! A transducer is *invertible* when its effect can be undone. The invertible
//! transducers form a **groupoid** under composition:
//!
//! - [`Identity`] inverts to itself.
//! - [`IsoMap`]`<to, from>` inverts to `IsoMap<from, to>` (swap directions).
//! - [`Compose`](crate::transducer::Compose)`<a, b>` inverts to
//!   `Compose<b⁻¹, a⁻¹>` — i.e. reverse the order and invert each part.
//!
//! This mirrors the existing [`Iso`](crate::optics::Iso) optic lifted to the
//! streaming layer: an `IsoMap` *is* a streaming isomorphism.
//!
//! ## What's *not* invertible
//!
//! Lossy transducers (`Filter`, `Take`, `Drop`, `Unique`, `FlatMap`, `Chunk` as
//! many-to-one, …) drop information and have no groupoid inverse. They do not
//! implement [`Invertible`]; attempting `invert()` on a pipeline containing them
//! is a compile error. (Provenance for those ops is Layer B, a future step.)
//!
//! ## Examples
//!
//! ```
//! use orlando_transducers::invert::{Invertible, IsoMap};
//! use orlando_transducers::collectors::to_vec;
//! use orlando_transducers::transducer::Transducer;
//!
//! // Celsius ⇄ Fahrenheit, as a streaming isomorphism.
//! let to_f = IsoMap::new(|c: f64| c * 9.0 / 5.0 + 32.0, |f: f64| (f - 32.0) * 5.0 / 9.0);
//!
//! let celsius = vec![0.0, 100.0, 25.0];
//! let fahrenheit = to_vec(&to_f, celsius.clone());
//! assert_eq!(fahrenheit, vec![32.0, 212.0, 77.0]);
//!
//! // Invert and recover the original input.
//! let to_c = to_f.invert();
//! let recovered = to_vec(&to_c, fahrenheit);
//! assert_eq!(recovered, celsius);
//! ```

use crate::describe::{Describable, StageSpec};
use crate::step::Step;
use crate::transducer::Transducer;
use std::marker::PhantomData;
use std::rc::Rc;

/// A transducer whose effect can be reversed.
///
/// Implementations form the **groupoid** of invertible transducers (Layer A).
/// The [`invert`](Self::invert) method produces a transducer going `Out -> In`
/// that undoes the forward `In -> Out` effect, and the composition law holds:
///
/// ```text
/// (t₁ ∘ t₂).invert()  ==  t₂.invert() ∘ t₁.invert()
/// t.invert().invert()  ==  t          (up to representation)
/// ```
///
/// The trait requires [`Describable`] so an inverted pipeline remains
/// introspectable. It is *not* object-safe (it has an associated type), which
/// is intentional: inversion is a type-level, statically-dispatched operation,
/// distinct from the object-safe reflection layer ([`Describable`]).
pub trait Invertible<In, Out>: Describable {
    /// The inverse transducer type, mapping `Out -> In`.
    type Inverse: Invertible<Out, In>;

    /// Produce the inverse transducer.
    fn invert(&self) -> Self::Inverse;
}

/// An invertible map: a streaming isomorphism pairing a `to` and `from` function.
///
/// Forward (as a [`Transducer`]) applies `to`; [`invert`](Invertible::invert)
/// produces the map applying `from`. This is the transducer analogue of the
/// [`Iso`](crate::optics::Iso) optic, lifted to streaming data.
///
/// Use this (rather than [`Map`](crate::transforms::Map)) when you can supply
/// an inverse and want the pipeline to be reversible.
///
/// # Examples
///
/// ```
/// use orlando_transducers::invert::IsoMap;
/// use orlando_transducers::invert::Invertible;
///
/// let double = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
/// // invert() gives the halving transducer
/// let _halve = double.invert();
/// ```
pub struct IsoMap<F, G, In, Out> {
    to: Rc<F>,
    from: Rc<G>,
    _phantom: PhantomData<(In, Out)>,
}

impl<F, G, In, Out> IsoMap<F, G, In, Out>
where
    F: Fn(In) -> Out,
    G: Fn(Out) -> In,
{
    /// Create an invertible map from a function and its inverse.
    ///
    /// **Caller responsibility:** `to` and `from` must be true inverses
    /// (`from(to(x)) == x` and `to(from(y)) == y`). This is not checked at
    /// runtime; violating it breaks the groupoid laws. Property tests cover the
    /// built-in examples.
    pub fn new(to: F, from: G) -> Self {
        IsoMap {
            to: Rc::new(to),
            from: Rc::new(from),
            _phantom: PhantomData,
        }
    }
}

impl<F, G, In, Out> Transducer<In, Out> for IsoMap<F, G, In, Out>
where
    F: Fn(In) -> Out + 'static,
    G: Fn(Out) -> In + 'static,
    In: 'static,
    Out: 'static,
{
    #[inline(always)]
    fn apply<Acc, R>(&self, reducer: R) -> Box<dyn Fn(Acc, In) -> Step<Acc>>
    where
        R: Fn(Acc, Out) -> Step<Acc> + 'static,
        Acc: 'static,
    {
        let to = Rc::clone(&self.to);
        Box::new(move |acc, val| reducer(acc, to(val)))
    }
}

impl<F, G, In, Out> Describable for IsoMap<F, G, In, Out> {
    fn describe_into(&self, out: &mut Vec<StageSpec>) {
        out.push(StageSpec::IsoMap);
    }
}

impl<F, G, In, Out> Invertible<In, Out> for IsoMap<F, G, In, Out>
where
    F: Fn(In) -> Out + 'static,
    G: Fn(Out) -> In + 'static,
    In: 'static,
    Out: 'static,
{
    // Swapping to/from and the type parameters yields the inverse groupoid element.
    type Inverse = IsoMap<G, F, Out, In>;

    fn invert(&self) -> Self::Inverse {
        IsoMap {
            to: Rc::clone(&self.from),
            from: Rc::clone(&self.to),
            _phantom: PhantomData,
        }
    }
}

/// Invert a transducer (free-function convenience for `t.invert()`).
pub fn invert<T, In, Out>(t: &T) -> T::Inverse
where
    T: Invertible<In, Out> + ?Sized,
{
    t.invert()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collectors::to_vec;
    use crate::transducer::{Identity, Transducer};

    // ---- IsoMap: forward ----

    #[test]
    fn iso_map_forward_applies_to() {
        let m = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
        assert_eq!(to_vec(&m, vec![1, 2, 3]), vec![2, 4, 6]);
    }

    #[test]
    fn iso_map_describes_as_isomap() {
        let m = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
        assert_eq!(m.describe(), vec![StageSpec::IsoMap]);
    }

    // ---- invert(): the groupoid inverse ----

    #[test]
    fn iso_map_invert_recovers_input() {
        let to_f = IsoMap::new(|c: f64| c * 9.0 / 5.0 + 32.0, |f: f64| (f - 32.0) * 5.0 / 9.0);
        let celsius = vec![0.0, 100.0, 25.0, -40.0];
        let fahrenheit = to_vec(&to_f, celsius.clone());

        let to_c = to_f.invert();
        let recovered = to_vec(&to_c, fahrenheit);
        assert_eq!(recovered, celsius);
    }

    #[test]
    fn iso_map_invert_describes_as_isomap() {
        let m = IsoMap::new(|x: i32| x + 1, |y: i32| y - 1);
        assert_eq!(m.invert().describe(), vec![StageSpec::IsoMap]);
    }

    #[test]
    fn iso_map_invert_actually_uses_from() {
        // Distinct functions so we can confirm invert() uses `from`, not `to`.
        // to = +10, from = -10
        let m = IsoMap::new(|x: i32| x + 10, |y: i32| y - 10);
        assert_eq!(to_vec(&m.invert(), vec![11, 22]), vec![1, 12]);
    }

    // ---- involution: invert().invert() recovers forward ----

    #[test]
    fn invert_is_involution() {
        let m = IsoMap::new(|x: i32| x * 3, |y: i32| y / 3);
        let roundtrip = m.invert().invert();
        assert_eq!(to_vec(&roundtrip, vec![1, 2, 3]), vec![3, 6, 9]);
    }

    // ---- Identity ----

    #[test]
    fn identity_inverts_to_identity() {
        let id = Identity::<i32>::new();
        let inv = id.invert();
        assert_eq!(to_vec(&inv, vec![1, 2, 3]), vec![1, 2, 3]);
        assert_eq!(inv.describe(), vec![StageSpec::Identity]);
    }

    // ---- composition: (a ∘ b)⁻¹ == b⁻¹ ∘ a⁻¹ ----

    #[test]
    fn composed_invert_reverses_order() {
        // a: x*2   b: x+10
        // forward: x -> (x*2) -> (x*2+10)
        // inverse:  y -> (y-10) -> ((y-10)/2)
        let a = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
        let b = IsoMap::new(|x: i32| x + 10, |y: i32| y - 10);
        let forward = a.compose(b);

        let input = vec![1, 2, 3, 4];
        let output = to_vec(&forward, input.clone());
        assert_eq!(output, vec![12, 14, 16, 18]);

        let inverse = forward.invert();
        let recovered = to_vec(&inverse, output);
        assert_eq!(recovered, input);
    }

    #[test]
    fn composed_invert_describes_reversed() {
        let a = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
        let b = IsoMap::new(|x: i32| x + 10, |y: i32| y - 10);
        let forward = a.compose(b);

        // Forward describes [IsoMap, IsoMap]; the inverse is also two IsoMaps
        // (reversal is structural, but each part still describes as IsoMap).
        assert_eq!(forward.invert().describe(), vec![StageSpec::IsoMap, StageSpec::IsoMap]);
    }

    #[test]
    fn composed_involution() {
        let a = IsoMap::new(|x: i32| x + 1, |y: i32| y - 1);
        let b = IsoMap::new(|x: i32| x * 5, |y: i32| y / 5);
        let forward = a.compose(b);

        let roundtrip = forward.invert().invert();
        assert_eq!(to_vec(&roundtrip, vec![1, 2, 3]), to_vec(&forward, vec![1, 2, 3]));
    }

    // ---- three-stage composition ----

    #[test]
    fn three_stage_pipeline_round_trips() {
        let p = IsoMap::new(|x: i32| x + 1, |y: i32| y - 1)
            .compose(IsoMap::new(|x: i32| x * 2, |y: i32| y / 2))
            .compose(IsoMap::new(|x: i32| x - 3, |y: i32| y + 3));

        let input: Vec<i32> = (1..=10).collect();
        let output = to_vec(&p, input.clone());
        let recovered = to_vec(&p.invert(), output);
        assert_eq!(recovered, input);
    }

    // ---- free function ----

    #[test]
    fn free_invert_function() {
        let m = IsoMap::new(|x: i32| x + 5, |y: i32| y - 5);
        let inv = invert(&m);
        assert_eq!(to_vec(&inv, vec![6, 11]), vec![1, 6]);
    }

    // ---- mixing IsoMap with Identity ----

    #[test]
    fn identity_in_compose_is_invertible() {
        let p = Identity::<i32>::new().compose(IsoMap::new(|x: i32| x * 2, |y: i32| y / 2));
        let input = vec![3, 5, 7];
        let output = to_vec(&p, input.clone());
        assert_eq!(to_vec(&p.invert(), output), input);
    }
}
