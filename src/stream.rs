// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! # Stream: A push-based event stream
//!
//! `Stream<T>` represents a sequence of discrete events over time.
//! Events can be mapped, filtered, merged, and folded — using the same
//! transducer concepts as Orlando's pull-based pipelines, but push-based.
//!
//! This is Orlando's bridge toward Cliffy's `Event<T>`.
//!
//! ## Usage
//!
//! ```rust
//! use orlando_transducers::stream::Stream;
//! use std::cell::RefCell;
//! use std::rc::Rc;
//!
//! let clicks = Stream::new();
//! let log = Rc::new(RefCell::new(Vec::new()));
//! let log_clone = log.clone();
//!
//! let _sub = clicks.subscribe(move |v: &i32| {
//!     log_clone.borrow_mut().push(*v);
//! });
//!
//! clicks.emit(1);
//! clicks.emit(2);
//! clicks.emit(3);
//!
//! assert_eq!(*log.borrow(), vec![1, 2, 3]);
//! ```

use std::cell::RefCell;
use std::rc::Rc;

type StreamListener<T> = Box<dyn Fn(&T)>;

/// A push-based event stream.
///
/// Events are emitted via `emit()` and propagated to subscribers and
/// derived streams (map, filter, merge, fold).
pub struct Stream<T: 'static> {
    inner: Rc<RefCell<StreamInner<T>>>,
}

struct StreamInner<T> {
    listeners: Vec<StreamListener<T>>,
}

/// A subscription handle for stream events.
pub struct StreamSubscription {
    #[allow(dead_code)]
    id: usize,
}

impl<T: 'static> Stream<T> {
    /// Create a new empty stream.
    pub fn new() -> Self {
        Stream {
            inner: Rc::new(RefCell::new(StreamInner {
                listeners: Vec::new(),
            })),
        }
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, value: T) {
        let inner = self.inner.borrow();
        for listener in &inner.listeners {
            listener(&value);
        }
    }

    /// Subscribe to events on this stream.
    pub fn subscribe(&self, f: impl Fn(&T) + 'static) -> StreamSubscription {
        let mut inner = self.inner.borrow_mut();
        let id = inner.listeners.len();
        inner.listeners.push(Box::new(f));
        StreamSubscription { id }
    }

    /// Create a new stream that transforms each event using a function.
    ///
    /// ```rust
    /// use orlando_transducers::stream::Stream;
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    ///
    /// let numbers = Stream::new();
    /// let doubled = numbers.map(|n: &i32| n * 2);
    ///
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let log_clone = log.clone();
    /// let _sub = doubled.subscribe(move |v: &i32| {
    ///     log_clone.borrow_mut().push(*v);
    /// });
    ///
    /// numbers.emit(1);
    /// numbers.emit(5);
    /// assert_eq!(*log.borrow(), vec![2, 10]);
    /// ```
    pub fn map<B: 'static>(&self, f: impl Fn(&T) -> B + 'static) -> Stream<B> {
        let derived = Stream::new();
        let derived_inner = derived.inner.clone();
        let transform = Rc::new(f);

        self.subscribe(move |value| {
            let mapped = transform(value);
            let inner = derived_inner.borrow();
            for listener in &inner.listeners {
                listener(&mapped);
            }
        });

        derived
    }

    /// Create a new stream that only passes events matching a predicate.
    ///
    /// ```rust
    /// use orlando_transducers::stream::Stream;
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    ///
    /// let numbers = Stream::new();
    /// let evens = numbers.filter(|n: &i32| n % 2 == 0);
    ///
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let log_clone = log.clone();
    /// let _sub = evens.subscribe(move |v: &i32| {
    ///     log_clone.borrow_mut().push(*v);
    /// });
    ///
    /// numbers.emit(1);
    /// numbers.emit(2);
    /// numbers.emit(3);
    /// numbers.emit(4);
    /// assert_eq!(*log.borrow(), vec![2, 4]);
    /// ```
    pub fn filter(&self, pred: impl Fn(&T) -> bool + 'static) -> Stream<T>
    where
        T: Clone,
    {
        let derived = Stream::new();
        let derived_inner = derived.inner.clone();
        let pred = Rc::new(pred);

        self.subscribe(move |value| {
            if pred(value) {
                let inner = derived_inner.borrow();
                let cloned = value.clone();
                for listener in &inner.listeners {
                    listener(&cloned);
                }
            }
        });

        derived
    }

    /// Fold (scan) over events, accumulating state into a Signal.
    ///
    /// Returns a `Signal` that holds the accumulated value.
    ///
    /// ```rust
    /// use orlando_transducers::stream::Stream;
    ///
    /// let clicks = Stream::new();
    /// let count = clicks.fold(0, |acc, _: &()| acc + 1);
    ///
    /// assert_eq!(*count.get(), 0);
    /// clicks.emit(());
    /// clicks.emit(());
    /// assert_eq!(*count.get(), 2);
    /// ```
    pub fn fold<B: Clone + 'static>(
        &self,
        initial: B,
        f: impl Fn(B, &T) -> B + 'static,
    ) -> crate::signal::Signal<B> {
        let signal = crate::signal::Signal::new(initial);
        let signal_clone = signal.clone();
        let f = Rc::new(f);

        self.subscribe(move |value| {
            let new_val = {
                let current = signal_clone.get();
                f(current.clone(), value)
            };
            signal_clone.set(new_val);
        });

        signal
    }

    /// Merge two streams into one. Events from either stream appear on the merged stream.
    ///
    /// ```rust
    /// use orlando_transducers::stream::Stream;
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    ///
    /// let a = Stream::new();
    /// let b = Stream::new();
    /// let merged = a.merge(&b);
    ///
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let log_clone = log.clone();
    /// let _sub = merged.subscribe(move |v: &i32| {
    ///     log_clone.borrow_mut().push(*v);
    /// });
    ///
    /// a.emit(1);
    /// b.emit(2);
    /// a.emit(3);
    /// assert_eq!(*log.borrow(), vec![1, 2, 3]);
    /// ```
    pub fn merge(&self, other: &Stream<T>) -> Stream<T>
    where
        T: Clone,
    {
        let merged = Stream::new();
        let merged_inner_a = merged.inner.clone();
        let merged_inner_b = merged.inner.clone();

        self.subscribe(move |value| {
            let inner = merged_inner_a.borrow();
            let cloned = value.clone();
            for listener in &inner.listeners {
                listener(&cloned);
            }
        });

        other.subscribe(move |value| {
            let inner = merged_inner_b.borrow();
            let cloned = value.clone();
            for listener in &inner.listeners {
                listener(&cloned);
            }
        });

        merged
    }

    /// Take only the first `n` events, then stop.
    ///
    /// ```rust
    /// use orlando_transducers::stream::Stream;
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    ///
    /// let numbers = Stream::new();
    /// let first_three = numbers.take(3);
    ///
    /// let log = Rc::new(RefCell::new(Vec::new()));
    /// let log_clone = log.clone();
    /// let _sub = first_three.subscribe(move |v: &i32| {
    ///     log_clone.borrow_mut().push(*v);
    /// });
    ///
    /// for i in 0..10 {
    ///     numbers.emit(i);
    /// }
    /// assert_eq!(*log.borrow(), vec![0, 1, 2]);
    /// ```
    pub fn take(&self, n: usize) -> Stream<T>
    where
        T: Clone,
    {
        let derived = Stream::new();
        let derived_inner = derived.inner.clone();
        let count = Rc::new(RefCell::new(0usize));

        self.subscribe(move |value| {
            let mut c = count.borrow_mut();
            if *c < n {
                *c += 1;
                let inner = derived_inner.borrow();
                let cloned = value.clone();
                for listener in &inner.listeners {
                    listener(&cloned);
                }
            }
        });

        derived
    }
}

impl<T: 'static> Default for Stream<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Clone for Stream<T> {
    fn clone(&self) -> Self {
        Stream {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_stream_emit_subscribe() {
        let s = Stream::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();

        let _sub = s.subscribe(move |v: &i32| {
            log_clone.borrow_mut().push(*v);
        });

        s.emit(1);
        s.emit(2);
        s.emit(3);
        assert_eq!(*log.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn test_stream_map() {
        let s = Stream::new();
        let doubled = s.map(|n: &i32| n * 2);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();
        let _sub = doubled.subscribe(move |v: &i32| {
            log_clone.borrow_mut().push(*v);
        });

        s.emit(1);
        s.emit(5);
        s.emit(10);
        assert_eq!(*log.borrow(), vec![2, 10, 20]);
    }

    #[test]
    fn test_stream_filter() {
        let s = Stream::new();
        let evens = s.filter(|n: &i32| n % 2 == 0);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();
        let _sub = evens.subscribe(move |v: &i32| {
            log_clone.borrow_mut().push(*v);
        });

        for i in 1..=6 {
            s.emit(i);
        }
        assert_eq!(*log.borrow(), vec![2, 4, 6]);
    }

    #[test]
    fn test_stream_merge() {
        let a = Stream::new();
        let b = Stream::new();
        let merged = a.merge(&b);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();
        let _sub = merged.subscribe(move |v: &i32| {
            log_clone.borrow_mut().push(*v);
        });

        a.emit(1);
        b.emit(2);
        a.emit(3);
        b.emit(4);
        assert_eq!(*log.borrow(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_stream_take() {
        let s = Stream::new();
        let first_two = s.take(2);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();
        let _sub = first_two.subscribe(move |v: &i32| {
            log_clone.borrow_mut().push(*v);
        });

        s.emit(10);
        s.emit(20);
        s.emit(30);
        s.emit(40);
        assert_eq!(*log.borrow(), vec![10, 20]);
    }

    #[test]
    fn test_stream_fold_to_signal() {
        let clicks = Stream::new();
        let count = clicks.fold(0, |acc, _: &()| acc + 1);

        assert_eq!(*count.get(), 0);
        clicks.emit(());
        assert_eq!(*count.get(), 1);
        clicks.emit(());
        clicks.emit(());
        assert_eq!(*count.get(), 3);
    }

    #[test]
    fn test_stream_map_filter_chain() {
        let s = Stream::new();
        let processed = s.map(|n: &i32| n * 2).filter(|n: &i32| *n > 5);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();
        let _sub = processed.subscribe(move |v: &i32| {
            log_clone.borrow_mut().push(*v);
        });

        for i in 1..=5 {
            s.emit(i);
        }
        // 1*2=2, 2*2=4, 3*2=6, 4*2=8, 5*2=10
        // filter >5: [6, 8, 10]
        assert_eq!(*log.borrow(), vec![6, 8, 10]);
    }

    #[test]
    fn test_stream_fold_sum() {
        let numbers = Stream::new();
        let sum = numbers.fold(0, |acc, n: &i32| acc + n);

        assert_eq!(*sum.get(), 0);
        numbers.emit(10);
        assert_eq!(*sum.get(), 10);
        numbers.emit(20);
        assert_eq!(*sum.get(), 30);
        numbers.emit(5);
        assert_eq!(*sum.get(), 35);
    }

    #[test]
    fn test_stream_multiple_subscribers() {
        let s = Stream::new();
        let log1 = Rc::new(RefCell::new(Vec::new()));
        let log2 = Rc::new(RefCell::new(Vec::new()));
        let l1 = log1.clone();
        let l2 = log2.clone();

        let _sub1 = s.subscribe(move |v: &i32| l1.borrow_mut().push(*v));
        let _sub2 = s.subscribe(move |v: &i32| l2.borrow_mut().push(v * 10));

        s.emit(1);
        s.emit(2);

        assert_eq!(*log1.borrow(), vec![1, 2]);
        assert_eq!(*log2.borrow(), vec![10, 20]);
    }

    #[test]
    fn test_stream_default() {
        let s: Stream<i32> = Stream::default();
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        let _sub = s.subscribe(move |v: &i32| l.borrow_mut().push(*v));
        s.emit(42);
        assert_eq!(*log.borrow(), vec![42]);
    }
}
