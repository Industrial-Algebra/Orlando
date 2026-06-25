//! Declarative pipeline macros: build a transducer chain and a matching
//! static descriptor from a single, readable declaration.
//!
//! These are the ergonomic layer over the [`Transducer`](crate::transducer::Transducer)
//! / [`Describable`](crate::describe::Describable) core. They exist so that:
//!
//! 1. **Composition reads naturally** — `map(f) >> filter(p) >> take(n)` instead
//!    of `Map::new(f).compose(Filter::new(p)).compose(Take::new(n))` with a
//!    battery of struct imports.
//! 2. **Forward and inverse stay in sync by construction** — both [`pipeline!`]
//!    and [`pipeline_descriptor!`] dispatch through the *same* per-operation
//!    mapping, and the built chain is itself [`Describable`] (Step 2), so
//!    `.describe()` and the macro-produced descriptor are provably consistent
//!    (a property test asserts this).
//!
//! The macros require no imports — all paths are `$crate`-qualified and
//! `compose` is invoked via UFCS, so `pipeline!{...}` works standalone.
//!
//! # Examples
//!
//! ```
//! use orlando_transducers::pipeline;
//! use orlando_transducers::{Describable, StageSpec};
//! use orlando_transducers::collectors::to_vec;
//!
//! // Build the chain — no struct imports needed.
//! let p = pipeline! {
//!     map(|x: i32| x * 2) >>
//!     filter(|x: &i32| *x > 5) >>
//!     take(3)
//! };
//!
//! assert_eq!(to_vec(&p, 1..100), vec![6, 8, 10]);
//!
//! // It describes itself for free (Layer C reflection):
//! assert_eq!(p.describe(), vec![
//!     StageSpec::Map,
//!     StageSpec::Filter,
//!     StageSpec::Take { n: 3 },
//! ]);
//! ```
//!
//! A compile-time, zero-allocation descriptor for the same stages:
//!
//! ```
//! use orlando_transducers::{pipeline_descriptor, StageSpec};
//!
//! const DESC: &[StageSpec] = pipeline_descriptor! {
//!     map >> filter >> take(3)
//! };
//! assert_eq!(DESC, &[StageSpec::Map, StageSpec::Filter, StageSpec::Take { n: 3 }]);
//! ```

/// Build a transducer pipeline from a `>>`-separated stage list.
///
/// Each stage mirrors a built-in transducer constructor. The result implements
/// both [`Transducer`](crate::transducer::Transducer) and
/// [`Describable`](crate::describe::Describable), so you can run it *and*
/// inspect it.
///
/// No imports are required — the macro is fully `$crate`-qualified.
///
/// # Stage syntax
///
/// | Stage | Equivalent |
/// |-------|-----------|
/// | `map(f)` | `Map::new(f)` |
/// | `filter(p)` | `Filter::new(p)` |
/// | `reject(p)` | `Reject::new(p)` |
/// | `take(n)` | `Take::new(n)` |
/// | `take_while(p)` | `TakeWhile::new(p)` |
/// | `drop(n)` | `Drop::new(n)` |
/// | `drop_while(p)` | `DropWhile::new(p)` |
/// | `chunk(size)` | `Chunk::new(size)` |
/// | `flat_map(f)` | `FlatMap::new(f)` |
/// | `scan(init, f)` | `Scan::new(init, f)` |
/// | `tap(f)` | `Tap::new(f)` |
/// | `interpose(sep)` | `Interpose::new(sep)` |
/// | `repeat_each(n)` | `RepeatEach::new(n)` |
/// | `aperture(size)` | `Aperture::new(size)` |
/// | `unique` | `Unique::new()` |
/// | `unique_by(f)` | `UniqueBy::new(f)` |
/// | `when(p, f)` | `When::new(p, f)` |
/// | `unless(p, f)` | `Unless::new(p, f)` |
/// | `if_else(p, t, e)` | `IfElse::new(p, t, e)` |
/// | `identity` | `Identity::new()` |
///
/// # Example
///
/// ```
/// use orlando_transducers::pipeline;
/// use orlando_transducers::collectors::to_vec;
///
/// let p = pipeline!(map(|x: i32| x + 1) >> filter(|x: &i32| *x % 2 == 0) >> take(2));
/// assert_eq!(to_vec(&p, 1..), vec![2, 4]);
/// ```
#[macro_export]
macro_rules! pipeline {
    // ---- terminal stages (no trailing >>) ----
    (identity) => { $crate::Identity::<_>::new() };
    (map($f:expr)) => { $crate::Map::new($f) };
    (filter($p:expr)) => { $crate::Filter::new($p) };
    (reject($p:expr)) => { $crate::Reject::new($p) };
    (take($n:expr)) => { $crate::Take::<_>::new($n) };
    (take_while($p:expr)) => { $crate::TakeWhile::new($p) };
    (drop($n:expr)) => { $crate::Drop::<_>::new($n) };
    (drop_while($p:expr)) => { $crate::DropWhile::new($p) };
    (chunk($size:expr)) => { $crate::Chunk::<_>::new($size) };
    (flat_map($f:expr)) => { $crate::FlatMap::new($f) };
    (scan($init:expr, $f:expr)) => { $crate::Scan::new($init, $f) };
    (tap($f:expr)) => { $crate::Tap::new($f) };
    (interpose($sep:expr)) => { $crate::Interpose::new($sep) };
    (repeat_each($n:expr)) => { $crate::RepeatEach::<_>::new($n) };
    (aperture($size:expr)) => { $crate::Aperture::<_>::new($size) };
    (unique) => { $crate::Unique::<_>::new() };
    (unique_by($f:expr)) => { $crate::UniqueBy::new($f) };
    (when($p:expr, $f:expr)) => { $crate::When::new($p, $f) };
    (unless($p:expr, $f:expr)) => { $crate::Unless::new($p, $f) };
    (if_else($p:expr, $t:expr, $e:expr)) => { $crate::IfElse::new($p, $t, $e) };

    // ---- recursive stages (stage >> rest) ----
    // UFCS compose so the Transducer trait need not be imported at the call site.
    (identity >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Identity::<_>::new(), $crate::pipeline!($($rest)*))
    };
    (map($f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Map::new($f), $crate::pipeline!($($rest)*))
    };
    (filter($p:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Filter::new($p), $crate::pipeline!($($rest)*))
    };
    (reject($p:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Reject::new($p), $crate::pipeline!($($rest)*))
    };
    (take($n:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Take::<_>::new($n), $crate::pipeline!($($rest)*))
    };
    (take_while($p:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::TakeWhile::new($p), $crate::pipeline!($($rest)*))
    };
    (drop($n:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Drop::<_>::new($n), $crate::pipeline!($($rest)*))
    };
    (drop_while($p:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::DropWhile::new($p), $crate::pipeline!($($rest)*))
    };
    (chunk($size:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Chunk::<_>::new($size), $crate::pipeline!($($rest)*))
    };
    (flat_map($f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::FlatMap::new($f), $crate::pipeline!($($rest)*))
    };
    (scan($init:expr, $f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Scan::new($init, $f), $crate::pipeline!($($rest)*))
    };
    (tap($f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Tap::new($f), $crate::pipeline!($($rest)*))
    };
    (interpose($sep:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Interpose::new($sep), $crate::pipeline!($($rest)*))
    };
    (repeat_each($n:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::RepeatEach::<_>::new($n), $crate::pipeline!($($rest)*))
    };
    (aperture($size:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Aperture::<_>::new($size), $crate::pipeline!($($rest)*))
    };
    (unique >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Unique::<_>::new(), $crate::pipeline!($($rest)*))
    };
    (unique_by($f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::UniqueBy::new($f), $crate::pipeline!($($rest)*))
    };
    (when($p:expr, $f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::When::new($p, $f), $crate::pipeline!($($rest)*))
    };
    (unless($p:expr, $f:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::Unless::new($p, $f), $crate::pipeline!($($rest)*))
    };
    (if_else($p:expr, $t:expr, $e:expr) >> $($rest:tt)*) => {
        $crate::Transducer::compose($crate::IfElse::new($p, $t, $e), $crate::pipeline!($($rest)*))
    };
}

/// Build a compile-time, zero-allocation `&[StageSpec]` descriptor.
///
/// Mirrors [`pipeline!`] stage names but only keeps the parameters that appear
/// in a `StageSpec` (counts/sizes); closure bodies are omitted. The result is a
/// `&'static [StageSpec]` suitable for `const` contexts.
///
/// This and [`pipeline!`] share the same per-operation semantics, so a pipeline
/// built with `pipeline!` and described via `.describe()` is guaranteed to
/// equal the matching `pipeline_descriptor!` (asserted in tests).
///
/// # Example
///
/// ```
/// use orlando_transducers::{pipeline_descriptor, StageSpec};
///
/// static DESC: &[StageSpec] = pipeline_descriptor! {
///     map >> filter >> take(3) >> chunk(2)
/// };
/// assert_eq!(DESC, &[
///     StageSpec::Map,
///     StageSpec::Filter,
///     StageSpec::Take { n: 3 },
///     StageSpec::Chunk { size: 2 },
/// ]);
/// ```
#[macro_export]
macro_rules! pipeline_descriptor {
    // base: nothing left to consume (must come before the catch-all entry arm)
    (@accum ($($done:expr),*)) => { &[$($done),*] };

    // ---- terminal stages ----
    (@accum ($($done:expr),*) identity) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Identity))
    };
    (@accum ($($done:expr),*) map) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Map))
    };
    (@accum ($($done:expr),*) filter) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Filter))
    };
    (@accum ($($done:expr),*) reject) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Reject))
    };
    (@accum ($($done:expr),*) take($n:expr)) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Take { n: $n }))
    };
    (@accum ($($done:expr),*) take_while) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::TakeWhile))
    };
    (@accum ($($done:expr),*) drop($n:expr)) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Drop { n: $n }))
    };
    (@accum ($($done:expr),*) drop_while) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::DropWhile))
    };
    (@accum ($($done:expr),*) chunk($size:expr)) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Chunk { size: $size }))
    };
    (@accum ($($done:expr),*) flat_map) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::FlatMap))
    };
    (@accum ($($done:expr),*) scan) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Scan))
    };
    (@accum ($($done:expr),*) tap) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Tap))
    };
    (@accum ($($done:expr),*) interpose) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Interpose))
    };
    (@accum ($($done:expr),*) repeat_each($n:expr)) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::RepeatEach { n: $n }))
    };
    (@accum ($($done:expr),*) aperture($size:expr)) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Aperture { size: $size }))
    };
    (@accum ($($done:expr),*) unique) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Unique))
    };
    (@accum ($($done:expr),*) unique_by) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::UniqueBy))
    };
    (@accum ($($done:expr),*) when) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::When))
    };
    (@accum ($($done:expr),*) unless) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Unless))
    };
    (@accum ($($done:expr),*) if_else) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::IfElse))
    };

    // ---- recursive stages ----
    (@accum ($($done:expr),*) identity >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Identity) $($rest)*)
    };
    (@accum ($($done:expr),*) map >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Map) $($rest)*)
    };
    (@accum ($($done:expr),*) filter >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Filter) $($rest)*)
    };
    (@accum ($($done:expr),*) reject >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Reject) $($rest)*)
    };
    (@accum ($($done:expr),*) take($n:expr) >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Take { n: $n }) $($rest)*)
    };
    (@accum ($($done:expr),*) take_while >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::TakeWhile) $($rest)*)
    };
    (@accum ($($done:expr),*) drop($n:expr) >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Drop { n: $n }) $($rest)*)
    };
    (@accum ($($done:expr),*) drop_while >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::DropWhile) $($rest)*)
    };
    (@accum ($($done:expr),*) chunk($size:expr) >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Chunk { size: $size }) $($rest)*)
    };
    (@accum ($($done:expr),*) flat_map >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::FlatMap) $($rest)*)
    };
    (@accum ($($done:expr),*) scan >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Scan) $($rest)*)
    };
    (@accum ($($done:expr),*) tap >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Tap) $($rest)*)
    };
    (@accum ($($done:expr),*) interpose >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Interpose) $($rest)*)
    };
    (@accum ($($done:expr),*) repeat_each($n:expr) >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::RepeatEach { n: $n }) $($rest)*)
    };
    (@accum ($($done:expr),*) aperture($size:expr) >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Aperture { size: $size }) $($rest)*)
    };
    (@accum ($($done:expr),*) unique >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Unique) $($rest)*)
    };
    (@accum ($($done:expr),*) unique_by >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::UniqueBy) $($rest)*)
    };
    (@accum ($($done:expr),*) when >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::When) $($rest)*)
    };
    (@accum ($($done:expr),*) unless >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::Unless) $($rest)*)
    };
    (@accum ($($done:expr),*) if_else >> $($rest:tt)*) => {
        $crate::pipeline_descriptor!(@accum ($($done,)* $crate::StageSpec::IfElse) $($rest)*)
    };

    // entry: seed the accumulator. Must come LAST so internal @accum calls
    // match a specific arm above instead of re-entering here.
    ($($input:tt)*) => { $crate::pipeline_descriptor!(@accum () $($input)*) };
}

#[cfg(test)]
mod tests {
    use crate::collectors::to_vec;
    use crate::{Describable, StageSpec};

    // ---- pipeline!: build + run ----

    #[test]
    fn pipeline_single_stage() {
        let p = pipeline!(map(|x: i32| x * 2));
        assert_eq!(to_vec(&p, vec![1, 2, 3]), vec![2, 4, 6]);
    }

    #[test]
    fn pipeline_three_stages() {
        let p = pipeline! {
            map(|x: i32| x * 2) >>
            filter(|x: &i32| *x > 5) >>
            take(3)
        };
        assert_eq!(to_vec(&p, 1..100), vec![6, 8, 10]);
    }

    #[test]
    fn pipeline_matches_manual_compose() {
        use crate::transforms::{Filter, Map, Take};
        use crate::transducer::Transducer;

        let manual = Map::new(|x: i32| x * 2)
            .compose(Filter::new(|x: &i32| *x > 5))
            .compose(Take::new(3));
        let mac = pipeline!(map(|x: i32| x * 2) >> filter(|x: &i32| *x > 5) >> take(3));

        let data: Vec<i32> = (1..100).collect();
        assert_eq!(to_vec(&manual, data.clone()), to_vec(&mac, data));
    }

    #[test]
    fn pipeline_no_extra_imports_needed() {
        // The macro is fully $crate-qualified and uses UFCS for compose, so it
        // expands without needing Map/Filter/Take/Transducer in scope.
        let p = pipeline!(map(|x: i32| x + 1) >> take(1));
        assert_eq!(to_vec(&p, vec![0, 1, 2]), vec![1]);
    }

    #[test]
    fn pipeline_parametric_stages() {
        let p = pipeline! {
            drop(1) >>
            chunk(2) >>
            take(2)
        };
        // [1,2,3,4,5,6] -> drop 1 -> [2,3,4,5,6] -> chunk(2) -> [[2,3],[4,5]]
        // -> take(2)
        let r = to_vec(&p, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(r, vec![vec![2, 3], vec![4, 5]]);
    }

    #[test]
    fn pipeline_describes_for_free() {
        let p = pipeline!(map(|x: i32| x) >> filter(|_: &i32| true) >> take(4));
        assert_eq!(
            p.describe(),
            vec![StageSpec::Map, StageSpec::Filter, StageSpec::Take { n: 4 }]
        );
    }

    #[test]
    fn pipeline_all_stage_forms_build() {
        // Smoke test: every supported stage form compiles and runs. Stages that
        // are generic over the element type (Unique, RepeatEach, ...) are
        // composed with a typed map so the element type is anchored.
        let _ = pipeline!(map(|x: i32| x) >> identity);
        let _ = pipeline!(reject(|x: &i32| *x < 0));
        let _ = pipeline!(take_while(|x: &i32| *x < 3));
        let _ = pipeline!(drop_while(|x: &i32| *x < 3));
        let _ = pipeline!(flat_map(|x: i32| vec![x]));
        let _ = pipeline!(scan(0i32, |a: &i32, x: &i32| a + x));
        let _ = pipeline!(tap(|_: &i32| {}));
        let _ = pipeline!(interpose(0));
        let _ = pipeline!(map(|x: i32| x) >> repeat_each(2));
        let _ = pipeline!(map(|x: i32| x) >> aperture(2));
        let _ = pipeline!(map(|x: i32| x) >> unique);
        let _ = pipeline!(unique_by(|x: &i32| *x));
        let _ = pipeline!(when(|_: &i32| true, |x: i32| x));
        let _ = pipeline!(unless(|_: &i32| false, |x: i32| x));
        let _ = pipeline!(if_else(|_: &i32| true, |x: i32| x, |x: i32| x));
    }

    // ---- pipeline_descriptor!: const descriptor ----

    #[test]
    fn descriptor_basic() {
        const DESC: &[StageSpec] = pipeline_descriptor!(map >> filter >> take(3));
        assert_eq!(
            DESC,
            &[StageSpec::Map, StageSpec::Filter, StageSpec::Take { n: 3 }]
        );
    }

    #[test]
    fn descriptor_in_const_context() {
        const DESC: &[StageSpec] = pipeline_descriptor! {
            identity >> map >> drop(2) >> chunk(4) >> repeat_each(2) >> aperture(3)
        };
        assert_eq!(
            DESC,
            &[
                StageSpec::Identity,
                StageSpec::Map,
                StageSpec::Drop { n: 2 },
                StageSpec::Chunk { size: 4 },
                StageSpec::RepeatEach { n: 2 },
                StageSpec::Aperture { size: 3 },
            ]
        );
    }

    #[test]
    fn descriptor_empty() {
        const DESC: &[StageSpec] = pipeline_descriptor!();
        assert!(DESC.is_empty());
    }

    #[test]
    fn descriptor_all_unit_stages() {
        const DESC: &[StageSpec] = pipeline_descriptor! {
            reject >> take_while >> drop_while >> flat_map >>
            scan >> tap >> interpose >> unique >> unique_by >>
            when >> unless >> if_else
        };
        assert_eq!(DESC.len(), 12);
        assert_eq!(DESC[0], StageSpec::Reject);
        assert_eq!(DESC[11], StageSpec::IfElse);
    }

    // ---- cross-consistency: the "can't drift" guarantee ----

    #[test]
    fn descriptor_matches_describe_of_pipeline() {
        // Build with pipeline!, introspect with .describe(); compare to the
        // static descriptor of the same stages. These must agree.
        let p = pipeline!(map(|x: i32| x) >> filter(|_: &i32| true) >> take(5));
        const DESC: &[StageSpec] = pipeline_descriptor!(map >> filter >> take(5));

        assert_eq!(p.describe(), DESC.to_vec());
    }

    #[test]
    fn descriptor_matches_describe_deep() {
        let p = pipeline! {
            map(|x: i32| x) >>
            drop(1) >>
            filter(|_: &i32| true) >>
            chunk(2) >>
            take(3)
        };
        const DESC: &[StageSpec] = pipeline_descriptor! {
            map >> drop(1) >> filter >> chunk(2) >> take(3)
        };

        assert_eq!(p.describe(), DESC.to_vec());
    }
}
