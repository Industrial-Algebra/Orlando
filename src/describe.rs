//! Pipeline reflection: describe composed transducers as serializable stage lists.
//!
//! Today Orlando's `Compose<T1, T2, …>` is purely type-level — once built, a
//! pipeline carries no runtime description of its own stages. This module adds the
//! **inverse capability**: any transducer implementing [`Describable`] can report
//! its stages as a `Vec<StageSpec>`, enabling introspection, serialization,
//! visualization, and (in later steps) inversion of the bijective subset.
//!
//! ## Design
//!
//! - [`StageSpec`] is **structural metadata only** — it records *what kind* of
//!   operation a stage is and its captured parameters, never the closure body
//!   (closures are not serializable). This is enough to inspect, serialize,
//!   compare, and eventually invert a pipeline.
//! - [`Describable`] is **opt-in and non-generic** — it imposes no type
//!   parameters and requires no `'static` bounds, so it composes cleanly across
//!   the built-in transducer hierarchy without burdening the core
//!   [`Transducer`](crate::transducer::Transducer) trait. It is object-safe and
//!   can be used through `Box<dyn Describable>` in type-erased contexts.
//!
//! ## Examples
//!
//! ```
//! use orlando_transducers::transforms::{Map, Filter, Take};
//! use orlando_transducers::transducer::Transducer;
//! use orlando_transducers::describe::{Describable, StageSpec};
//!
//! let pipeline = Map::new(|x: i32| x * 2)
//!     .compose(Filter::new(|x: &i32| *x > 5))
//!     .compose(Take::new(3));
//!
//! let stages = pipeline.describe();
//! assert_eq!(stages, vec![
//!     StageSpec::Map,
//!     StageSpec::Filter,
//!     StageSpec::Take { n: 3 },
//! ]);
//! ```

/// A serializable description of a single pipeline stage.
///
/// One variant per built-in transducer. Variants that capture parameters
/// (e.g. [`Take`](Self::Take), [`Chunk`](Self::Chunk)) record them as named
/// fields; the rest are unit variants since their behavior is fully determined
/// by the closure supplied at runtime (which is not serializable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageSpec {
    /// The identity transducer — passes values through unchanged.
    Identity,
    /// Transform each value with a function (`Map`).
    Map,
    /// Keep only values matching a predicate (`Filter`).
    Filter,
    /// Inverse of [`Filter`](Self::Filter) — drop values matching a predicate.
    Reject,
    /// Take the first `n` elements, then stop (early termination).
    Take { n: usize },
    /// Take elements while a predicate holds, then stop.
    TakeWhile,
    /// Skip the first `n` elements.
    Drop { n: usize },
    /// Skip elements while a predicate holds.
    DropWhile,
    /// Group consecutive elements into chunks of `size`.
    Chunk { size: usize },
    /// Drop consecutive duplicate values.
    Unique,
    /// Drop duplicates by a key function.
    UniqueBy,
    /// Running accumulation emitting every intermediate state.
    Scan,
    /// Map each element to a collection and flatten (`FlatMap`).
    FlatMap,
    /// Side effect without transformation (`Tap`).
    Tap,
    /// Insert a separator between elements (`Interpose`).
    Interpose,
    /// Repeat each element `n` times (`RepeatEach`).
    RepeatEach { n: usize },
    /// Sliding window of `size` elements (`Aperture`).
    Aperture { size: usize },
    /// Conditional transform applied only when a predicate is true (`When`).
    When,
    /// Conditional transform applied only when a predicate is false (`Unless`).
    Unless,
    /// Branch between two transforms on a predicate (`IfElse`).
    IfElse,
}

impl StageSpec {
    /// Returns a stable, human-readable name for the stage.
    ///
    /// These names mirror the JavaScript `Pipeline` method names and are
    /// intended for serialization/UI display. They are stable across versions.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Map => "map",
            Self::Filter => "filter",
            Self::Reject => "reject",
            Self::Take { .. } => "take",
            Self::TakeWhile => "takeWhile",
            Self::Drop { .. } => "drop",
            Self::DropWhile => "dropWhile",
            Self::Chunk { .. } => "chunk",
            Self::Unique => "unique",
            Self::UniqueBy => "uniqueBy",
            Self::Scan => "scan",
            Self::FlatMap => "flatMap",
            Self::Tap => "tap",
            Self::Interpose => "interpose",
            Self::RepeatEach { .. } => "repeatEach",
            Self::Aperture { .. } => "aperture",
            Self::When => "when",
            Self::Unless => "unless",
            Self::IfElse => "ifElse",
        }
    }

    /// Whether this stage belongs to the **bijective (reversible) subset** —
    /// the groupoid of transducers that admit a clean inverse.
    ///
    /// This is the foundation of Layer A in the inverse-transducer design
    /// (`docs/INVERSE_TRANSDUCER_DESIGN.md`). Only total one-to-one
    /// transformations are reversible; anything that drops, filters, takes,
    /// flattens, chunks, or branches is lossy and returns `false`.
    ///
    /// # Caveat: `Map` is optimistic
    ///
    /// `Map` returns `true` here, but a `Map` is only *actually* reversible when
    /// its function is invertible. `StageSpec` cannot inspect the closure, so
    /// this classification is structural — callers constructing a true inverse
    /// (Step 4) must supply and verify the inverse function explicitly.
    pub fn is_reversible(&self) -> bool {
        matches!(self, Self::Identity | Self::Map)
    }

    /// Convenience inverse of [`is_reversible`](Self::is_reversible).
    pub fn is_lossy(&self) -> bool {
        !self.is_reversible()
    }
}

/// Something that can describe itself as a sequence of [`StageSpec`]s.
///
/// Implemented by all built-in transducers and by
/// [`Compose`](crate::transducer::Compose). Composition preserves data-flow
/// order: `a.compose(b).describe()` yields `a`'s stages followed by `b`'s.
///
/// The trait is non-generic and object-safe, so it can be stored as
/// `Box<dyn Describable>` in type-erased contexts (e.g. a reflected pipeline).
pub trait Describable {
    /// Append this item's stage(s) to `out`, in data-flow order.
    fn describe_into(&self, out: &mut Vec<StageSpec>);

    /// Collect all stages describing this item, in data-flow order.
    fn describe(&self) -> Vec<StageSpec> {
        let mut out = Vec::new();
        self.describe_into(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collectors::to_vec;
    use crate::logic::{IfElse, Unless, When};
    use crate::transducer::{Identity, Transducer};
    use crate::transforms::{
        Aperture, Chunk, Drop, DropWhile, Filter, FlatMap, Interpose, Map, Reject, RepeatEach,
        Scan, Take, TakeWhile, Tap, Unique, UniqueBy,
    };

    #[test]
    fn test_atomic_stages() {
        assert_eq!(Map::new(|x: i32| x).describe(), vec![StageSpec::Map]);
        assert_eq!(
            Filter::new(|_: &i32| true).describe(),
            vec![StageSpec::Filter]
        );
        assert_eq!(
            Reject::new(|_: &i32| false).describe(),
            vec![StageSpec::Reject]
        );
        assert_eq!(
            Take::<i32>::new(3).describe(),
            vec![StageSpec::Take { n: 3 }]
        );
        assert_eq!(
            TakeWhile::new(|_: &i32| true).describe(),
            vec![StageSpec::TakeWhile]
        );
        assert_eq!(
            Drop::<i32>::new(2).describe(),
            vec![StageSpec::Drop { n: 2 }]
        );
        assert_eq!(
            DropWhile::new(|_: &i32| false).describe(),
            vec![StageSpec::DropWhile]
        );
        assert_eq!(
            Chunk::<i32>::new(4).describe(),
            vec![StageSpec::Chunk { size: 4 }]
        );
        assert_eq!(Unique::<i32>::new().describe(), vec![StageSpec::Unique]);
        assert_eq!(
            UniqueBy::new(|x: &i32| *x).describe(),
            vec![StageSpec::UniqueBy]
        );
        assert_eq!(
            Scan::new(0, |a: &i32, _: &i32| *a).describe(),
            vec![StageSpec::Scan]
        );
        assert_eq!(
            FlatMap::new(|x: i32| vec![x]).describe(),
            vec![StageSpec::FlatMap]
        );
        assert_eq!(Tap::new(|_: &i32| {}).describe(), vec![StageSpec::Tap]);
        assert_eq!(
            Interpose::new(0).describe(),
            vec![StageSpec::Interpose]
        );
        assert_eq!(
            RepeatEach::<i32>::new(2).describe(),
            vec![StageSpec::RepeatEach { n: 2 }]
        );
        assert_eq!(
            Aperture::<i32>::new(3).describe(),
            vec![StageSpec::Aperture { size: 3 }]
        );
    }

    #[test]
    fn test_identity_stage() {
        assert_eq!(
            Identity::<i32>::new().describe(),
            vec![StageSpec::Identity]
        );
    }

    #[test]
    fn test_conditional_stages() {
        assert_eq!(
            When::new(|_: &i32| true, |x: i32| x).describe(),
            vec![StageSpec::When]
        );
        assert_eq!(
            Unless::new(|_: &i32| false, |x: i32| x).describe(),
            vec![StageSpec::Unless]
        );
        assert_eq!(
            IfElse::new(|_: &i32| true, |x: i32| x, |x: i32| x).describe(),
            vec![StageSpec::IfElse]
        );
    }

    #[test]
    fn test_compose_preserves_data_flow_order() {
        // Map ∘ Filter ∘ Take  ->  data flows map -> filter -> take
        let pipeline = Map::new(|x: i32| x * 2)
            .compose(Filter::new(|x: &i32| *x > 5))
            .compose(Take::new(3));

        assert_eq!(
            pipeline.describe(),
            vec![
                StageSpec::Map,
                StageSpec::Filter,
                StageSpec::Take { n: 3 },
            ]
        );
    }

    #[test]
    fn test_deeply_nested_compose() {
        let pipeline = Map::new(|x: i32| x + 1)
            .compose(Filter::new(|_: &i32| true))
            .compose(Map::new(|x: i32| x * 2))
            .compose(Drop::<i32>::new(1))
            .compose(Take::new(2));

        assert_eq!(
            pipeline.describe(),
            vec![
                StageSpec::Map,
                StageSpec::Filter,
                StageSpec::Map,
                StageSpec::Drop { n: 1 },
                StageSpec::Take { n: 2 },
            ]
        );
    }

    #[test]
    fn test_compose_describe_matches_actual_execution() {
        // Guard against a describe/apply ordering mismatch: the described order
        // must be the order in which data actually passes through the stages.
        let pipeline = Map::new(|x: i32| x * 2)
            .compose(Filter::new(|x: &i32| *x > 5))
            .compose(Take::new(3));

        // If describe order were reversed, this would still pass trivially;
        // the real correctness check is that map*2 then >5 then take 3 yields [6,8,10].
        let result = to_vec(&pipeline, 1..100);
        assert_eq!(result, vec![6, 8, 10]);

        // And the described stages must line up with that interpretation.
        assert_eq!(
            pipeline.describe(),
            vec![StageSpec::Map, StageSpec::Filter, StageSpec::Take { n: 3 }]
        );
    }

    #[test]
    fn test_stage_names() {
        assert_eq!(StageSpec::Map.name(), "map");
        assert_eq!(StageSpec::Take { n: 5 }.name(), "take");
        assert_eq!(StageSpec::Chunk { size: 2 }.name(), "chunk");
        assert_eq!(StageSpec::FlatMap.name(), "flatMap");
        assert_eq!(StageSpec::Identity.name(), "identity");
    }

    #[test]
    fn test_reversibility_classification() {
        // Bijective subset (Layer A).
        assert!(StageSpec::Identity.is_reversible());
        assert!(StageSpec::Map.is_reversible());
        assert!(!StageSpec::Identity.is_lossy());

        // Everything else is lossy.
        assert!(!StageSpec::Filter.is_reversible());
        assert!(StageSpec::Filter.is_lossy());
        assert!(!StageSpec::Take { n: 3 }.is_reversible());
        assert!(StageSpec::Take { n: 3 }.is_lossy());
        assert!(!StageSpec::Drop { n: 1 }.is_reversible());
        assert!(!StageSpec::Chunk { size: 2 }.is_reversible());
        assert!(!StageSpec::FlatMap.is_reversible());
        assert!(!StageSpec::Unique.is_reversible());
        assert!(!StageSpec::IfElse.is_reversible());
    }

    #[test]
    fn test_describable_through_trait_object() {
        // Object safety: must be usable as Box<dyn Describable>.
        let pipelines: Vec<Box<dyn Describable>> = vec![
            Box::new(Map::new(|x: i32| x * 2)),
            Box::new(Filter::new(|_: &i32| true).compose(Take::<i32>::new(5))),
        ];

        assert_eq!(pipelines[0].describe(), vec![StageSpec::Map]);
        assert_eq!(
            pipelines[1].describe(),
            vec![StageSpec::Filter, StageSpec::Take { n: 5 }]
        );
    }
}
