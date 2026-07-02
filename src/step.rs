// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! The Step monad for early termination in transducer chains.
//!
//! # Category Theory
//!
//! The Step type represents a monad that encodes the possibility of early termination
//! during a reduction operation. This is essential for transducers like `take` and
//! `take_while` that need to signal when processing should stop.
//!
//! Mathematically, Step<T> is isomorphic to Either T T, where:
//! - Continue(T) represents "continue processing with this value"
//! - Stop(T) represents "stop processing with this final value"
//!
//! The monad laws hold:
//! - Left identity: `cont(x).and_then(f) == f(x)`
//! - Right identity: `m.and_then(cont) == m`
//! - Associativity: `m.and_then(f).and_then(g) == m.and_then(|x| f(x).and_then(g))`

use std::fmt;

/// A Step represents a value in a reduction that may signal early termination.
///
/// # Examples
///
/// ```
/// use orlando_transducers::step::{cont, stop, is_stopped};
///
/// let continuing = cont(42);
/// assert!(!is_stopped(&continuing));
///
/// let stopped = stop(42);
/// assert!(is_stopped(&stopped));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step<T> {
    /// Continue processing with this value
    Continue(T),
    /// Stop processing with this final value
    Stop(T),
}

impl<T> Step<T> {
    /// Returns true if this Step is Continue
    #[inline(always)]
    pub fn is_continue(&self) -> bool {
        matches!(self, Step::Continue(_))
    }

    /// Returns true if this Step is Stop
    #[inline(always)]
    pub fn is_stop(&self) -> bool {
        matches!(self, Step::Stop(_))
    }

    /// Unwrap the inner value regardless of whether this is Continue or Stop
    #[inline(always)]
    pub fn unwrap(self) -> T {
        match self {
            Step::Continue(v) | Step::Stop(v) => v,
        }
    }

    /// Map a function over the inner value, preserving Continue/Stop status
    #[inline(always)]
    pub fn map<U, F>(self, f: F) -> Step<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Step::Continue(v) => Step::Continue(f(v)),
            Step::Stop(v) => Step::Stop(f(v)),
        }
    }

    /// Monadic bind operation
    #[inline(always)]
    pub fn and_then<U, F>(self, f: F) -> Step<U>
    where
        F: FnOnce(T) -> Step<U>,
        U: From<T>,
    {
        match self {
            Step::Continue(v) => f(v),
            Step::Stop(v) => Step::Stop(v.into()),
        }
    }

    /// Convert Continue to Some, Stop to None (for early termination detection)
    #[inline(always)]
    pub fn continue_value(self) -> Option<T> {
        match self {
            Step::Continue(v) => Some(v),
            Step::Stop(_) => None,
        }
    }
}

/// Helper function to create a Continue step
#[inline(always)]
pub fn cont<T>(value: T) -> Step<T> {
    Step::Continue(value)
}

/// Helper function to create a Stop step
#[inline(always)]
pub fn stop<T>(value: T) -> Step<T> {
    Step::Stop(value)
}

/// Check if a Step is stopped
#[inline(always)]
pub fn is_stopped<T>(step: &Step<T>) -> bool {
    step.is_stop()
}

/// Unwrap a Step value
#[inline(always)]
pub fn unwrap_step<T>(step: Step<T>) -> T {
    step.unwrap()
}

impl<T: fmt::Display> fmt::Display for Step<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Step::Continue(v) => write!(f, "Continue({})", v),
            Step::Stop(v) => write!(f, "Stop({})", v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continue() {
        let step = cont(42);
        assert!(step.is_continue());
        assert!(!step.is_stop());
        assert_eq!(step.unwrap(), 42);
    }

    #[test]
    fn test_stop() {
        let step = stop(42);
        assert!(step.is_stop());
        assert!(!step.is_continue());
        assert_eq!(step.unwrap(), 42);
    }

    #[test]
    fn test_map() {
        let step = cont(42);
        let mapped = step.map(|x| x * 2);
        assert_eq!(mapped, cont(84));

        let stopped = stop(42).map(|x| x * 2);
        assert_eq!(stopped, stop(84));
    }

    #[test]
    fn test_monad_laws() {
        // Left identity: cont(x).and_then(f) == f(x)
        let f = |x| cont(x * 2);
        assert_eq!(cont(42).and_then(f), f(42));

        // Right identity: m.and_then(cont) == m
        let m = cont(42);
        assert_eq!(m.and_then(cont), m);

        // Associativity
        let f = |x| cont(x * 2);
        let g = |x| cont(x + 10);
        let m = cont(42);
        assert_eq!(m.and_then(f).and_then(g), m.and_then(|x| f(x).and_then(g)));
    }
}
