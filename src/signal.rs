// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! # Signal: A time-varying reactive value
//!
//! `Signal<T>` represents a value that changes over time. When the value changes,
//! all subscribers and derived signals are automatically updated.
//!
//! This is Orlando's bridge toward Cliffy's `Behavior<T>` — providing the pipeline
//! semantics while Cliffy will later provide the geometric state backing.
//!
//! ## Usage
//!
//! ```rust
//! use orlando_transducers::signal::Signal;
//!
//! let count = Signal::new(0);
//! let doubled = count.map(|n| n * 2);
//!
//! assert_eq!(*count.get(), 0);
//! assert_eq!(*doubled.get(), 0);
//!
//! count.set(5);
//! assert_eq!(*count.get(), 5);
//! assert_eq!(*doubled.get(), 10);
//! ```

use std::cell::RefCell;
use std::rc::Rc;

type Listener<T> = Box<dyn Fn(&T)>;
type Transform<A, B> = Rc<dyn Fn(&A) -> B>;

/// A reactive value that notifies subscribers when it changes.
///
/// Signals form a directed acyclic graph: derived signals automatically
/// update when their sources change.
pub struct Signal<T: Clone + 'static> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

struct SignalInner<T: Clone> {
    value: T,
    listeners: Vec<Listener<T>>,
}

impl<T: Clone + 'static> Signal<T> {
    /// Create a new signal with an initial value.
    pub fn new(value: T) -> Self {
        Signal {
            inner: Rc::new(RefCell::new(SignalInner {
                value,
                listeners: Vec::new(),
            })),
        }
    }

    /// Get the current value.
    pub fn get(&self) -> std::cell::Ref<'_, T> {
        std::cell::Ref::map(self.inner.borrow(), |inner| &inner.value)
    }

    /// Set a new value, notifying all subscribers.
    pub fn set(&self, value: T) {
        let mut inner = self.inner.borrow_mut();
        inner.value = value;
        let value_ref = &inner.value;
        for listener in &inner.listeners {
            listener(value_ref);
        }
    }

    /// Update the value using a function, notifying all subscribers.
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let new_value = {
            let inner = self.inner.borrow();
            f(&inner.value)
        };
        self.set(new_value);
    }

    /// Subscribe to value changes. The callback is called with each new value.
    ///
    /// Returns a `Subscription` that can be used to unsubscribe.
    pub fn subscribe(&self, f: impl Fn(&T) + 'static) -> Subscription {
        let id = {
            let mut inner = self.inner.borrow_mut();
            let id = inner.listeners.len();
            inner.listeners.push(Box::new(f));
            id
        };
        Subscription { id }
    }

    /// Create a derived signal by mapping over this signal's value.
    ///
    /// The derived signal automatically updates when this signal changes.
    ///
    /// ```rust
    /// use orlando_transducers::signal::Signal;
    ///
    /// let count = Signal::new(3);
    /// let label = count.map(|n| format!("Count: {}", n));
    ///
    /// assert_eq!(*label.get(), "Count: 3");
    /// count.set(10);
    /// assert_eq!(*label.get(), "Count: 10");
    /// ```
    pub fn map<B: Clone + 'static>(&self, f: impl Fn(&T) -> B + 'static) -> Signal<B> {
        let initial = f(&self.inner.borrow().value);
        let derived = Signal::new(initial);
        let derived_inner = derived.inner.clone();
        let transform: Transform<T, B> = Rc::new(f);

        self.subscribe(move |value| {
            let new_value = transform(value);
            let mut inner = derived_inner.borrow_mut();
            inner.value = new_value;
            let value_ref = &inner.value;
            for listener in &inner.listeners {
                listener(value_ref);
            }
        });

        derived
    }

    /// Combine two signals into a new signal using a combining function.
    ///
    /// ```rust
    /// use orlando_transducers::signal::Signal;
    ///
    /// let a = Signal::new(3);
    /// let b = Signal::new(4);
    /// let sum = a.combine(&b, |x, y| x + y);
    ///
    /// assert_eq!(*sum.get(), 7);
    /// a.set(10);
    /// assert_eq!(*sum.get(), 14);
    /// ```
    pub fn combine<U: Clone + 'static, R: Clone + 'static>(
        &self,
        other: &Signal<U>,
        f: impl Fn(&T, &U) -> R + 'static,
    ) -> Signal<R> {
        let initial = {
            let a = self.inner.borrow();
            let b = other.inner.borrow();
            f(&a.value, &b.value)
        };
        let derived = Signal::new(initial);

        // Subscribe to changes from self
        let derived_inner_a = derived.inner.clone();
        let other_inner_a = other.inner.clone();
        let f_a = Rc::new(f);
        let f_b = f_a.clone();

        self.subscribe(move |a_value| {
            let b_value = other_inner_a.borrow();
            let new_value = f_a(a_value, &b_value.value);
            let mut inner = derived_inner_a.borrow_mut();
            inner.value = new_value;
            let value_ref = &inner.value;
            for listener in &inner.listeners {
                listener(value_ref);
            }
        });

        // Subscribe to changes from other
        let derived_inner_b = derived.inner.clone();
        let self_inner_b = self.inner.clone();

        other.subscribe(move |b_value| {
            let a_value = self_inner_b.borrow();
            let new_value = f_b(&a_value.value, b_value);
            let mut inner = derived_inner_b.borrow_mut();
            inner.value = new_value;
            let value_ref = &inner.value;
            for listener in &inner.listeners {
                listener(value_ref);
            }
        });

        derived
    }

    /// Fold over the signal's changes, accumulating a value.
    ///
    /// Returns a new signal that holds the accumulated value.
    ///
    /// ```rust
    /// use orlando_transducers::signal::Signal;
    ///
    /// let value = Signal::new(0);
    /// let history = value.fold(Vec::new(), |mut acc, v| {
    ///     acc.push(*v);
    ///     acc
    /// });
    ///
    /// value.set(1);
    /// value.set(2);
    /// assert_eq!(*history.get(), vec![0, 1, 2]);
    /// ```
    pub fn fold<B: Clone + 'static>(
        &self,
        initial: B,
        f: impl Fn(B, &T) -> B + 'static,
    ) -> Signal<B> {
        let init = {
            let inner = self.inner.borrow();
            f(initial, &inner.value)
        };
        let derived = Signal::new(init);
        let derived_inner = derived.inner.clone();
        let f = Rc::new(f);

        self.subscribe(move |value| {
            let new_acc = {
                let inner = derived_inner.borrow();
                f(inner.value.clone(), value)
            };
            let mut inner = derived_inner.borrow_mut();
            inner.value = new_acc;
            let value_ref = &inner.value;
            for listener in &inner.listeners {
                listener(value_ref);
            }
        });

        derived
    }
}

impl<T: Clone + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            inner: self.inner.clone(),
        }
    }
}

/// A subscription handle.
///
/// Currently subscriptions are permanent (live as long as the signal).
/// In a future version, dropping this handle will unsubscribe.
pub struct Subscription {
    #[allow(dead_code)]
    id: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn test_signal_basic() {
        let s = Signal::new(42);
        assert_eq!(*s.get(), 42);
        s.set(100);
        assert_eq!(*s.get(), 100);
    }

    #[test]
    fn test_signal_update() {
        let s = Signal::new(10);
        s.update(|n| n + 5);
        assert_eq!(*s.get(), 15);
    }

    #[test]
    fn test_signal_subscribe() {
        let s = Signal::new(0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let log_clone = log.clone();

        let _sub = s.subscribe(move |value| {
            log_clone.borrow_mut().push(*value);
        });

        s.set(1);
        s.set(2);
        s.set(3);

        assert_eq!(*log.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn test_signal_map() {
        let count = Signal::new(0);
        let doubled = count.map(|n| n * 2);

        assert_eq!(*doubled.get(), 0);
        count.set(5);
        assert_eq!(*doubled.get(), 10);
        count.set(7);
        assert_eq!(*doubled.get(), 14);
    }

    #[test]
    fn test_signal_map_chain() {
        let count = Signal::new(1);
        let doubled = count.map(|n| n * 2);
        let label = doubled.map(|n| format!("Value: {}", n));

        assert_eq!(*label.get(), "Value: 2");
        count.set(5);
        assert_eq!(*label.get(), "Value: 10");
    }

    #[test]
    fn test_signal_combine() {
        let a = Signal::new(3);
        let b = Signal::new(4);
        let sum = a.combine(&b, |x, y| x + y);

        assert_eq!(*sum.get(), 7);
        a.set(10);
        assert_eq!(*sum.get(), 14);
        b.set(1);
        assert_eq!(*sum.get(), 11);
    }

    #[test]
    fn test_signal_fold() {
        let value = Signal::new(0);
        let history = value.fold(Vec::new(), |mut acc, v| {
            acc.push(*v);
            acc
        });

        assert_eq!(*history.get(), vec![0]);
        value.set(1);
        assert_eq!(*history.get(), vec![0, 1]);
        value.set(2);
        assert_eq!(*history.get(), vec![0, 1, 2]);
    }

    #[test]
    fn test_signal_combine_string() {
        let first = Signal::new("Hello".to_string());
        let second = Signal::new("World".to_string());
        let greeting = first.combine(&second, |a, b| format!("{} {}!", a, b));

        assert_eq!(*greeting.get(), "Hello World!");
        first.set("Goodbye".to_string());
        assert_eq!(*greeting.get(), "Goodbye World!");
    }

    #[test]
    fn test_signal_multiple_subscribers() {
        let s = Signal::new(0);
        let log1 = Rc::new(RefCell::new(Vec::new()));
        let log2 = Rc::new(RefCell::new(Vec::new()));
        let log1_clone = log1.clone();
        let log2_clone = log2.clone();

        let _sub1 = s.subscribe(move |v| log1_clone.borrow_mut().push(*v));
        let _sub2 = s.subscribe(move |v| log2_clone.borrow_mut().push(v * 10));

        s.set(1);
        s.set(2);

        assert_eq!(*log1.borrow(), vec![1, 2]);
        assert_eq!(*log2.borrow(), vec![10, 20]);
    }
}
