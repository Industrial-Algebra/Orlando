// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! # Iterator Extension for Transducers
//!
//! Provides an extension trait that lets you apply transducer pipelines
//! directly to Rust iterators using the `.transduce()` method.
//!
//! ## Usage
//!
//! ```rust
//! use orlando_transducers::transforms::{Map, Filter, Take};
//! use orlando_transducers::transducer::Transducer;
//! use orlando_transducers::iter_ext::TransduceExt;
//!
//! let pipeline = Map::new(|x: i32| x * 2)
//!     .compose(Filter::new(|x: &i32| *x > 5))
//!     .compose(Take::new(3));
//!
//! let result: Vec<i32> = (1i32..100).transduce(&pipeline).collect();
//! assert_eq!(result, vec![6, 8, 10]);
//! ```

use crate::step::Step;
use crate::transducer::Transducer;

/// Extension trait that adds `.transduce()` to any iterator.
pub trait TransduceExt: Iterator + Sized {
    /// Apply a transducer pipeline to this iterator, returning a new iterator
    /// that yields transformed elements.
    ///
    /// The transducer is applied lazily — elements are only processed as
    /// they are consumed from the returned iterator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use orlando_transducers::transforms::{Map, Filter};
    /// use orlando_transducers::transducer::Transducer;
    /// use orlando_transducers::iter_ext::TransduceExt;
    ///
    /// let result: Vec<_> = vec![1, 2, 3, 4, 5]
    ///     .into_iter()
    ///     .transduce(&Map::new(|x: i32| x * 10))
    ///     .collect();
    /// assert_eq!(result, vec![10, 20, 30, 40, 50]);
    /// ```
    fn transduce<Out, T>(self, transducer: &T) -> TransducedIterator<Self::Item, Out>
    where
        T: Transducer<Self::Item, Out>,
        Self::Item: 'static,
        Out: 'static;
}

impl<I: Iterator + 'static> TransduceExt for I {
    fn transduce<Out, T>(self, transducer: &T) -> TransducedIterator<Self::Item, Out>
    where
        T: Transducer<Self::Item, Out>,
        Self::Item: 'static,
        Out: 'static,
    {
        // We buffer output elements since a single input can produce
        // zero or more outputs (e.g., flatMap produces many, filter produces zero or one).
        let reducer = |mut acc: Vec<Out>, x: Out| {
            acc.push(x);
            crate::step::cont(acc)
        };
        let step_fn = transducer.apply(reducer);

        TransducedIterator {
            source: Box::new(self),
            step_fn,
            buffer: Vec::new(),
            buffer_pos: 0,
            stopped: false,
        }
    }
}

/// An iterator adapter that applies a transducer pipeline to source elements.
pub struct TransducedIterator<In: 'static, Out: 'static> {
    source: Box<dyn Iterator<Item = In>>,
    step_fn: StepFn<In, Out>,
    buffer: Vec<Out>,
    buffer_pos: usize,
    stopped: bool,
}

impl<In, Out> Iterator for TransducedIterator<In, Out> {
    type Item = Out;

    fn next(&mut self) -> Option<Out> {
        loop {
            // If we have buffered items, return the next one
            if self.buffer_pos < self.buffer.len() {
                let item = self.buffer.remove(self.buffer_pos);
                return Some(item);
            }

            // If we've been stopped (e.g., by Take), we're done
            if self.stopped {
                return None;
            }

            // Get the next source element
            let input = self.source.next()?;

            // Apply the transducer step
            self.buffer.clear();
            self.buffer_pos = 0;

            match (self.step_fn)(Vec::new(), input) {
                Step::Continue(items) => {
                    self.buffer = items;
                }
                Step::Stop(items) => {
                    self.buffer = items;
                    self.stopped = true;
                }
            }
        }
    }
}

/// A builder for creating transducer pipelines with a fluent API.
///
/// This provides a Rust-native alternative to the WASM Pipeline,
/// allowing pipeline construction without JavaScript interop.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::iter_ext::PipelineBuilder;
///
/// let result: Vec<i32> = PipelineBuilder::new()
///     .map(|x: i32| x * 2)
///     .run(vec![1, 2, 3]);
///
/// assert_eq!(result, vec![2, 4, 6]);
/// ```
pub struct PipelineBuilder<In: 'static, Out: 'static> {
    transducer: Box<dyn ErasedTransducer<In, Out>>,
}

type Reducer<T> = Box<dyn Fn(Vec<T>, T) -> Step<Vec<T>>>;
type StepFn<In, Out> = Box<dyn Fn(Vec<Out>, In) -> Step<Vec<Out>>>;

/// Type-erased transducer for use in PipelineBuilder.
trait ErasedTransducer<In: 'static, Out: 'static> {
    fn apply_erased(&self, reducer: Reducer<Out>) -> StepFn<In, Out>;
}

/// Wrapper to make any Transducer into an ErasedTransducer.
struct TransducerWrapper<T> {
    inner: T,
}

impl<In: 'static, Out: 'static, T: Transducer<In, Out>> ErasedTransducer<In, Out>
    for TransducerWrapper<T>
{
    #[allow(clippy::redundant_closure)]
    fn apply_erased(&self, reducer: Reducer<Out>) -> StepFn<In, Out> {
        // Cannot pass `reducer` directly — `apply` needs `impl Fn` but `reducer` is `Box<dyn Fn>`.
        // The closure captures the Box and calls through it.
        self.inner.apply(move |acc, x| reducer(acc, x))
    }
}

impl PipelineBuilder<(), ()> {
    /// Create a new empty pipeline builder.
    pub fn new() -> PipelineBuilder<(), ()> {
        PipelineBuilder {
            transducer: Box::new(TransducerWrapper {
                inner: crate::transducer::Identity::<()>::new(),
            }),
        }
    }
}

impl Default for PipelineBuilder<(), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineBuilder<(), ()> {
    /// Start a pipeline with a map operation.
    pub fn map<A: 'static, B: 'static>(
        self,
        f: impl Fn(A) -> B + 'static,
    ) -> PipelineBuilder<A, B> {
        PipelineBuilder {
            transducer: Box::new(TransducerWrapper {
                inner: crate::transforms::Map::new(f),
            }),
        }
    }

    /// Start a pipeline with a filter operation.
    pub fn filter<A: Clone + 'static>(
        self,
        pred: impl Fn(&A) -> bool + 'static,
    ) -> PipelineBuilder<A, A> {
        PipelineBuilder {
            transducer: Box::new(TransducerWrapper {
                inner: crate::transforms::Filter::new(pred),
            }),
        }
    }

    /// Start a pipeline with a take operation.
    pub fn take<A: 'static>(self, n: usize) -> PipelineBuilder<A, A> {
        PipelineBuilder {
            transducer: Box::new(TransducerWrapper {
                inner: crate::transforms::Take::<A>::new(n),
            }),
        }
    }
}

impl<In: 'static, Out: 'static> PipelineBuilder<In, Out> {
    /// Execute the pipeline on a data source, collecting results into a Vec.
    pub fn run<Iter: IntoIterator<Item = In>>(self, source: Iter) -> Vec<Out> {
        let reducer = Box::new(|mut acc: Vec<Out>, x: Out| {
            acc.push(x);
            crate::step::cont(acc)
        });

        let step_fn = self.transducer.apply_erased(reducer);
        let mut result = Vec::new();

        for item in source {
            match step_fn(result, item) {
                Step::Continue(new_result) => result = new_result,
                Step::Stop(final_result) => {
                    result = final_result;
                    break;
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transducer::Transducer;

    #[test]
    fn test_transduce_map() {
        let pipeline = crate::transforms::Map::new(|x: i32| x * 2);
        let result: Vec<_> = vec![1, 2, 3].into_iter().transduce(&pipeline).collect();
        assert_eq!(result, vec![2, 4, 6]);
    }

    #[test]
    fn test_transduce_filter() {
        let pipeline = crate::transforms::Filter::new(|x: &i32| *x > 3);
        let result: Vec<_> = (1..=5).transduce(&pipeline).collect();
        assert_eq!(result, vec![4, 5]);
    }

    #[test]
    fn test_transduce_take() {
        let pipeline = crate::transforms::Take::<i32>::new(3);
        let result: Vec<_> = (1..=1_000_000).transduce(&pipeline).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_transduce_composed() {
        let pipeline = crate::transforms::Map::new(|x: i32| x * 2)
            .compose(crate::transforms::Filter::new(|x: &i32| *x > 5))
            .compose(crate::transforms::Take::new(3));

        let result: Vec<_> = (1..100).transduce(&pipeline).collect();
        assert_eq!(result, vec![6, 8, 10]);
    }

    #[test]
    fn test_transduce_with_for_loop() {
        let pipeline = crate::transforms::Map::new(|x: i32| x * x);
        let mut sum = 0;
        for val in (1..=4).transduce(&pipeline) {
            sum += val;
        }
        assert_eq!(sum, 1 + 4 + 9 + 16);
    }

    #[test]
    fn test_transduce_chaining_with_std_iterators() {
        let pipeline = crate::transforms::Map::new(|x: i32| x * 2);
        let result: Vec<_> = (1..=5)
            .transduce(&pipeline)
            .filter(|x| x % 3 == 0)
            .collect();
        assert_eq!(result, vec![6]);
    }

    #[test]
    fn test_transduce_empty_iterator() {
        let pipeline = crate::transforms::Map::new(|x: i32| x * 2);
        let result: Vec<i32> = std::iter::empty().transduce(&pipeline).collect();
        assert_eq!(result, Vec::<i32>::new());
    }

    #[test]
    fn test_pipeline_builder_map_filter_take() {
        let result = PipelineBuilder::new()
            .map(|x: i32| x * 2)
            .run(vec![1, 2, 3, 4, 5]);
        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_pipeline_builder_filter() {
        let result = PipelineBuilder::new().filter(|x: &i32| *x > 3).run(1..=5);
        assert_eq!(result, vec![4, 5]);
    }

    #[test]
    fn test_pipeline_builder_take() {
        let result: Vec<i32> = PipelineBuilder::new().take(3).run(1..=1000);
        assert_eq!(result, vec![1, 2, 3]);
    }
}
