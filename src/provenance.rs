//! Provenance traces for lossy transducers (Layer B of the inverse-transducer design).
//!
//! Layer A ([`Invertible`](crate::invert::Invertible)) handles the *bijective*
//! subset — pipelines that can be truly reversed. But most real pipelines are
//! **lossy**: `Filter` drops elements, `Take` truncates, `Drop` skips,
//! `FlatMap` fans out. These have no groupoid inverse — information is destroyed.
//!
//! Layer B answers a different, still-useful question: **"which inputs produced
//! these outputs?"** Given the source (or a recorded trace), you can recover the
//! provenance of every output element:
//!
//! | Lossy op | Its Layer-B "inverse" (provenance) |
//! |----------|-------------------------------------|
//! | `Filter` | a boolean kept-mask over the input stream |
//! | `Take(n)` | "the prefix of length n" — first n inputs |
//! | `Drop(n)` | "the suffix after index n" |
//! | `FlatMap` | many outputs tagged to one input |
//! | `Chunk` | a chunk tagged to its completing input |
//!
//! Unlike inversion, provenance is a **post-hoc** decompose: it needs the
//! original source stream (or a recorded trace), because the lost information
//! lives there. This module records the trace as a side product of execution.
//!
//! ## Examples
//!
//! ```
//! use orlando_transducers::provenance::trace;
//! use orlando_transducers::transforms::{Map, Filter};
//! use orlando_transducers::transducer::Transducer;
//!
//! // Keep even numbers, then double them.
//! let pipeline = Filter::new(|x: &i32| x % 2 == 0)
//!     .compose(Map::new(|x: i32| x * 2));
//!
//! let data = vec![1, 2, 3, 4, 5, 6];
//! let (outputs, trace) = trace(&pipeline, data.clone());
//!
//! assert_eq!(outputs, vec![4, 8, 12]);
//! // outputs[0]=4 came from input[1]=2, outputs[1]=8 from input[3]=4, etc.
//! assert_eq!(trace.sources, vec![1, 3, 5]);
//! // The "inverse of Filter" — a kept-mask over the original input:
//! assert_eq!(trace.kept_mask(data.len()), vec![false, true, false, true, false, true]);
//! ```

use crate::step::Step;
use crate::transducer::Transducer;
use std::cell::RefCell;
use std::rc::Rc;

/// A recorded provenance tape: for each output element, the index of the
/// source input that produced it.
///
/// Produced by [`trace`]. `sources[oi]` is the input index that generated
/// output element `oi`. This is enough to answer the Layer-B "inverse"
/// questions for lossy transducers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// `sources[oi]` = index of the input element that produced output `oi`.
    ///
    /// For fan-out ops (`FlatMap`), multiple consecutive outputs may share the
    /// same source index. For many-to-one ops (`Chunk`), a single output is
    /// tagged with the index of the input that completed the group.
    pub sources: Vec<usize>,
}

impl Trace {
    /// Number of output elements recorded.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Whether no outputs were recorded.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Which input index produced output `oi`? Returns `None` if out of range.
    pub fn source_of(&self, output_index: usize) -> Option<usize> {
        self.sources.get(output_index).copied()
    }

    /// All output indices produced by `input_index`.
    ///
    /// For a 1-to-1 stage this is one element; for `FlatMap` it may be many;
    /// for an input that was filtered out, it is empty.
    pub fn outputs_of(&self, input_index: usize) -> Vec<usize> {
        self.sources
            .iter()
            .enumerate()
            .filter(|(_, &src)| src == input_index)
            .map(|(oi, _)| oi)
            .collect()
    }

    /// A boolean mask over the input stream: `true` where the input produced
    /// at least one output (i.e. it survived the pipeline).
    ///
    /// This is the **Layer-B inverse of `Filter`** (and of `Take`/`Drop`, which
    /// act as positional filters). `mask.len() == input_len`; inputs beyond the
    /// trace's observed range are reported `false`.
    pub fn kept_mask(&self, input_len: usize) -> Vec<bool> {
        let mut mask = vec![false; input_len];
        for &src in &self.sources {
            if src < input_len {
                mask[src] = true;
            }
        }
        mask
    }

    /// Indices of inputs that were **dropped** (produced no output) — the
    /// complement of [`kept_mask`](Self::kept_mask).
    pub fn dropped_indices(&self, input_len: usize) -> Vec<usize> {
        let mask = self.kept_mask(input_len);
        (0..input_len).filter(|i| !mask[*i]).collect()
    }
}

/// Run a transducer over a source, returning both the outputs and a
/// [`Trace`] recording which input index produced each output.
///
/// This is the Layer-B execution path. It works for *any* transducer — lossy
/// or invertible — but is most useful for lossy pipelines where
/// [`Invertible::invert`](crate::invert::Invertible::invert) does not apply.
///
/// # How provenance is captured
///
/// The current input index is threaded through a shared cell that is advanced
/// once per source element *before* the transducer processes it. The reducing
/// function reads that cell to tag each emitted output with its source index.
/// As a result:
///
/// - `Map`/`Filter`: output tagged with the input that produced it.
/// - `FlatMap`: every fanned-out output shares the originating input's index.
/// - `Chunk`/`Aperture`: the emitted group is tagged with the index of the
///   input that completed it.
/// - `Take`/`Drop`: standard prefix/suffix indexing.
///
/// # Example
///
/// ```
/// use orlando_transducers::provenance::trace;
/// use orlando_transducers::transforms::FlatMap;
/// use orlando_transducers::transducer::Transducer;
///
/// // Each input fans out to two outputs.
/// let pipeline = FlatMap::new(|x: i32| vec![x, x + 10]);
/// let data = vec![1, 2];
/// let (outputs, trace) = trace(&pipeline, data);
///
/// assert_eq!(outputs, vec![1, 11, 2, 12]);
/// // outputs 0,1 came from input 0; outputs 2,3 from input 1.
/// assert_eq!(trace.sources, vec![0, 0, 1, 1]);
/// assert_eq!(trace.outputs_of(0), vec![0, 1]);
/// ```
pub fn trace<T, In, Out, Iter>(transducer: &T, source: Iter) -> (Vec<Out>, Trace)
where
    T: Transducer<In, Out>,
    In: 'static,
    Out: 'static,
    Iter: IntoIterator<Item = In>,
{
    // Shared cell holding the index of the input currently being processed.
    // Advanced by the driver loop below; read by the reducer to tag outputs.
    let current = Rc::new(RefCell::new(0usize));
    let current_for_reducer = Rc::clone(&current);

    let reducer = move |mut acc: Vec<(Out, usize)>, out: Out| -> Step<Vec<(Out, usize)>> {
        let idx = *current_for_reducer.borrow();
        acc.push((out, idx));
        crate::step::cont(acc)
    };

    let step_fn = transducer.apply(reducer);

    let mut acc: Vec<(Out, usize)> = Vec::new();
    for (i, in_val) in source.into_iter().enumerate() {
        // Tag all outputs emitted while processing this input with its index.
        *current.borrow_mut() = i;
        match step_fn(acc, in_val) {
            Step::Continue(new_acc) => acc = new_acc,
            Step::Stop(final_acc) => {
                acc = final_acc;
                break;
            }
        }
    }

    let len = acc.len();
    let mut outputs = Vec::with_capacity(len);
    let mut sources = Vec::with_capacity(len);
    for (out, idx) in acc {
        outputs.push(out);
        sources.push(idx);
    }

    (outputs, Trace { sources })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transducer::Transducer;
    use crate::transforms::{Aperture, Chunk, Drop, Filter, FlatMap, Map, Take};

    // ---- identity / map: 1-to-1 provenance ----

    #[test]
    fn trace_map_preserves_indices() {
        let p = Map::new(|x: i32| x * 10);
        let (out, t) = trace(&p, vec![1, 2, 3]);
        assert_eq!(out, vec![10, 20, 30]);
        assert_eq!(t.sources, vec![0, 1, 2]);
    }

    // ---- filter: the kept-mask is the inverse ----

    #[test]
    fn trace_filter_kept_mask() {
        let p = Filter::new(|x: &i32| x % 2 == 0);
        let data = vec![1, 2, 3, 4, 5, 6];
        let (out, t) = trace(&p, data.clone());

        assert_eq!(out, vec![2, 4, 6]);
        // Each kept output carries its original input index.
        assert_eq!(t.sources, vec![1, 3, 5]);
        // The Layer-B inverse of Filter: which inputs survived.
        assert_eq!(
            t.kept_mask(data.len()),
            vec![false, true, false, true, false, true]
        );
        assert_eq!(t.dropped_indices(data.len()), vec![0, 2, 4]);
    }

    #[test]
    fn trace_filter_outputs_of_dropped_is_empty() {
        let p = Filter::new(|x: &i32| *x > 2);
        let (_out, t) = trace(&p, vec![1, 2, 3, 4]);
        // input 0 (value 1) was dropped -> no outputs.
        assert!(t.outputs_of(0).is_empty());
        // input 2 (value 3) survived -> output index 0.
        assert_eq!(t.outputs_of(2), vec![0]);
    }

    // ---- take: prefix provenance ----

    #[test]
    fn trace_take_prefix() {
        let p = Take::<i32>::new(3);
        let (out, t) = trace(&p, 0..1_000_000);
        assert_eq!(out, vec![0, 1, 2]);
        assert_eq!(t.sources, vec![0, 1, 2]);
        // Inverse-of-Take: first 3 inputs kept.
        assert_eq!(
            t.kept_mask(10),
            vec![true, true, true, false, false, false, false, false, false, false]
        );
    }

    // ---- drop: suffix provenance ----

    #[test]
    fn trace_drop_suffix() {
        let p = Drop::<i32>::new(2);
        let data = vec![10, 20, 30, 40];
        let (out, t) = trace(&p, data.clone());
        assert_eq!(out, vec![30, 40]);
        assert_eq!(t.sources, vec![2, 3]);
        assert_eq!(t.kept_mask(data.len()), vec![false, false, true, true]);
    }

    // ---- flatmap: fan-out, many outputs per input ----

    #[test]
    fn trace_flatmap_shares_source_index() {
        let p = FlatMap::new(|x: i32| vec![x, x + 10, x + 20]);
        let data = vec![1, 2];
        let (out, t) = trace(&p, data);

        assert_eq!(out, vec![1, 11, 21, 2, 12, 22]);
        // First three outputs all came from input 0; next three from input 1.
        assert_eq!(t.sources, vec![0, 0, 0, 1, 1, 1]);
        assert_eq!(t.outputs_of(0), vec![0, 1, 2]);
        assert_eq!(t.outputs_of(1), vec![3, 4, 5]);
    }

    // ---- chunk: many-to-one, tagged at completion ----

    #[test]
    fn trace_chunk_completion_index() {
        let p = Chunk::<i32>::new(2);
        let data = vec![1, 2, 3, 4];
        let (out, t) = trace(&p, data);

        assert_eq!(out, vec![vec![1, 2], vec![3, 4]]);
        // Chunk [1,2] completes at input index 1; [3,4] at index 3.
        assert_eq!(t.sources, vec![1, 3]);
    }

    // ---- aperture: sliding window ----

    #[test]
    fn trace_aperture_indices() {
        let p = Aperture::<i32>::new(2);
        let (_out, t) = trace(&p, vec![1, 2, 3, 4]);
        // Windows [1,2],[2,3],[3,4] complete at inputs 1,2,3.
        assert_eq!(t.sources, vec![1, 2, 3]);
    }

    // ---- composed lossy pipeline ----

    #[test]
    fn trace_composed_lossy_pipeline() {
        // Filter evens, then double, then take first 2.
        let p = Filter::new(|x: &i32| x % 2 == 0)
            .compose(Map::new(|x: i32| x * 2))
            .compose(Take::new(2));

        let data = vec![1, 2, 3, 4, 5, 6, 8];
        let (out, t) = trace(&p, data.clone());

        assert_eq!(out, vec![4, 8]);
        // outputs came from inputs 1 and 3 (the first two evens).
        assert_eq!(t.sources, vec![1, 3]);
        // Only those two inputs survived to an output.
        assert_eq!(
            t.kept_mask(data.len()),
            vec![false, true, false, true, false, false, false]
        );
    }

    // ---- source_of round-trip ----

    #[test]
    fn trace_source_of() {
        let p = Map::new(|x: i32| x);
        let (_out, t) = trace(&p, vec![10, 20, 30]);
        assert_eq!(t.source_of(0), Some(0));
        assert_eq!(t.source_of(2), Some(2));
        assert_eq!(t.source_of(5), None);
    }

    // ---- empty / degenerate ----

    #[test]
    fn trace_empty_source() {
        let p = Map::new(|x: i32| x * 2);
        let (out, t) = trace(&p, Vec::<i32>::new());
        assert!(out.is_empty());
        assert!(t.is_empty());
        assert_eq!(t.kept_mask(0), Vec::<bool>::new());
    }

    #[test]
    fn trace_filter_all_dropped() {
        let p = Filter::new(|_: &i32| false);
        let (_out, t) = trace(&p, vec![1, 2, 3]);
        assert!(t.is_empty());
        assert_eq!(t.kept_mask(3), vec![false, false, false]);
        assert_eq!(t.dropped_indices(3), vec![0, 1, 2]);
    }

    // ---- invertible pipeline also traces (provenance is orthogonal) ----

    #[test]
    fn trace_works_on_invertible_pipeline() {
        use crate::invert::IsoMap;
        let p = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
        let (out, t) = trace(&p, vec![1, 2, 3]);
        assert_eq!(out, vec![2, 4, 6]);
        assert_eq!(t.sources, vec![0, 1, 2]);
    }
}
