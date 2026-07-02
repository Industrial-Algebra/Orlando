// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! # Optics: Functional Lenses for Immutable Data Access
//!
//! Optics provide composable, type-safe access to nested data structures.
//! This module implements a hierarchy of optics based on category theory:
//!
//! - **Lens**: Total focus on a part of a structure (always exists)
//! - **Optional**: Partial focus (may not exist)
//! - **Traversal**: Multiple foci (collection elements)
//! - **Prism**: Focus on a variant of a sum type
//! - **Iso**: Lossless bidirectional conversion
//!
//! ## Lens Laws
//!
//! All lenses must satisfy three fundamental laws:
//!
//! 1. **GetPut**: `set(s, get(s)) = s` - Setting what you got changes nothing
//! 2. **PutGet**: `get(set(s, a)) = a` - Getting what you set returns that value
//! 3. **PutPut**: `set(set(s, a1), a2) = set(s, a2)` - Setting twice = setting once
//!
//! These laws ensure lenses are well-behaved and composable.
//!
//! ## Usage
//!
//! ```rust
//! use orlando_transducers::optics::Lens;
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct User {
//!     name: String,
//!     age: u32,
//! }
//!
//! // Create a lens that focuses on a specific field
//! let name_lens = Lens::new(
//!     |user: &User| user.name.clone(),
//!     |user: &User, name: String| User { name, age: user.age }
//! );
//!
//! let user = User {
//!     name: "Alice".to_string(),
//!     age: 30,
//! };
//!
//! // Get the focused value
//! let name = name_lens.get(&user);
//! assert_eq!(name, "Alice");
//!
//! // Set the focused value
//! let updated_user = name_lens.set(&user, "Bob".to_string());
//! assert_eq!(updated_user.name, "Bob");
//!
//! // Transform the focused value
//! let upper_user = name_lens.over(&user, |s| s.to_uppercase());
//! assert_eq!(upper_user.name, "ALICE");
//! ```

use std::marker::PhantomData;
use std::rc::Rc;

use karpal_profunctor::{Choice, Profunctor, Strong, Traversing};

// Type aliases to satisfy clippy type_complexity lint.
// Using Rc (not Box) so closures can be shared into profunctor transform closures.
type GetterFn<S, A> = Rc<dyn Fn(&S) -> A>;
type SetterFn<S, A> = Rc<dyn Fn(&S, A) -> S>;
type OptionalGetter<S, A> = Rc<dyn Fn(&S) -> Option<A>>;
type ReviewerFn<S, A> = Rc<dyn Fn(A) -> S>;
type FoldGetter<S, A> = Rc<dyn Fn(&S) -> Vec<A>>;
type TraversalSetter<S, A> = Rc<dyn Fn(&S, &dyn Fn(A) -> A) -> S>;

/// A Lens focuses on a part A of a structure S, allowing both reading and updating.
///
/// A Lens is defined by two functions:
/// - `get: &S -> A` - Extract the focused value
/// - `set: (&S, A) -> S` - Update the focused value immutably
///
/// Lenses compose: given `Lens<S, A>` and `Lens<A, B>`, we can create `Lens<S, B>`.
/// A composed lens from chaining two lenses. In Orlando, this is identical to `Lens`
/// since both use closure-based storage (unlike Karpal where `Lens` uses fn ptrs).
pub type ComposedLens<S, A> = Lens<S, A>;

pub struct Lens<S, A>
where
    S: Clone,
    A: Clone,
{
    get: GetterFn<S, A>,
    set: SetterFn<S, A>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> Lens<S, A>
where
    S: Clone,
    A: Clone,
{
    /// Create a new lens from getter and setter functions.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use orlando_transducers::optics::Lens;
    /// #[derive(Clone)]
    /// struct User { name: String }
    ///
    /// let name_lens = Lens::new(
    ///     |user: &User| user.name.clone(),
    ///     |user: &User, name: String| User { name }
    /// );
    /// ```
    pub fn new<G, S2>(get_fn: G, set_fn: S2) -> Self
    where
        G: Fn(&S) -> A + 'static,
        S2: Fn(&S, A) -> S + 'static,
    {
        Lens {
            get: Rc::new(get_fn),
            set: Rc::new(set_fn),
            _phantom: PhantomData,
        }
    }

    /// Extract the focused value from the structure.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use orlando_transducers::optics::Lens;
    /// # #[derive(Clone)]
    /// # struct User { name: String }
    /// # let lens = Lens::new(
    /// #     |user: &User| user.name.clone(),
    /// #     |user: &User, name: String| User { name }
    /// # );
    /// # let user = User { name: "Alice".to_string() };
    /// let name = lens.get(&user);
    /// assert_eq!(name, "Alice");
    /// ```
    pub fn get(&self, source: &S) -> A {
        (self.get)(source)
    }

    /// Update the focused value immutably, returning a new structure.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use orlando_transducers::optics::Lens;
    /// # #[derive(Clone, PartialEq, Debug)]
    /// # struct User { name: String }
    /// # let lens = Lens::new(
    /// #     |user: &User| user.name.clone(),
    /// #     |user: &User, name: String| User { name }
    /// # );
    /// # let user = User { name: "Alice".to_string() };
    /// let updated = lens.set(&user, "Bob".to_string());
    /// assert_eq!(updated.name, "Bob");
    /// assert_eq!(user.name, "Alice"); // Original unchanged
    /// ```
    pub fn set(&self, source: &S, value: A) -> S {
        (self.set)(source, value)
    }

    /// Transform the focused value using a function.
    ///
    /// This is equivalent to: `set(source, f(get(source)))`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use orlando_transducers::optics::Lens;
    /// # #[derive(Clone)]
    /// # struct User { name: String }
    /// # let lens = Lens::new(
    /// #     |user: &User| user.name.clone(),
    /// #     |user: &User, name: String| User { name }
    /// # );
    /// # let user = User { name: "Alice".to_string() };
    /// let updated = lens.over(&user, |name| name.to_uppercase());
    /// assert_eq!(updated.name, "ALICE");
    /// ```
    pub fn over<F>(&self, source: &S, f: F) -> S
    where
        F: Fn(A) -> A,
    {
        let current = self.get(source);
        let updated = f(current);
        self.set(source, updated)
    }

    /// Apply this lens as a profunctor transformer.
    ///
    /// A lens is a `Strong` profunctor transformer: given `P<A, A>`, produce `P<S, S>`.
    /// This is the profunctor optics encoding from Karpal.
    pub fn transform<P: Strong>(&self, pab: P::P<A, A>) -> P::P<S, S>
    where
        S: 'static,
        A: 'static,
    {
        let get = self.get.clone();
        let set = self.set.clone();
        P::dimap(
            move |s: S| {
                let a = get(&s);
                (a, s)
            },
            move |(a, s): (A, S)| set(&s, a),
            P::first::<A, A, S>(pab),
        )
    }

    /// Compose with another lens, focusing deeper. Alias for `compose()`.
    pub fn then<B>(self, inner: Lens<A, B>) -> Lens<S, B>
    where
        B: Clone + 'static,
        A: 'static,
        S: 'static,
    {
        self.compose(inner)
    }

    /// Convert this Lens into a Traversal (a Lens is a single-focus Traversal).
    pub fn to_traversal(&self) -> Traversal<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let get1 = self.get.clone();
        let get2 = self.get.clone();
        let set = self.set.clone();
        Traversal::new(
            move |s: &S| vec![get1(s)],
            move |s: &S, f: &dyn Fn(A) -> A| set(s, f(get2(s))),
        )
    }

    /// Convert this Lens into a read-only Fold.
    pub fn to_fold(&self) -> Fold<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let get = self.get.clone();
        Fold::new(move |s: &S| vec![get(s)])
    }

    /// Compose two lenses to focus deeper into nested structures.
    ///
    /// Given `Lens<S, A>` and `Lens<A, B>`, produces `Lens<S, B>`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use orlando_transducers::optics::Lens;
    /// # #[derive(Clone)]
    /// # struct User { address: Address }
    /// # #[derive(Clone)]
    /// # struct Address { city: String }
    /// # let address_lens = Lens::new(
    /// #     |user: &User| user.address.clone(),
    /// #     |user: &User, address: Address| User { address }
    /// # );
    /// # let city_lens = Lens::new(
    /// #     |addr: &Address| addr.city.clone(),
    /// #     |addr: &Address, city: String| Address { city }
    /// # );
    /// let user_city_lens = address_lens.compose(city_lens);
    /// # let user = User { address: Address { city: "NYC".to_string() } };
    /// let city = user_city_lens.get(&user);
    /// assert_eq!(city, "NYC");
    /// ```
    pub fn compose<B>(self, other: Lens<A, B>) -> Lens<S, B>
    where
        B: Clone + 'static,
        A: 'static,
        S: 'static,
    {
        let self_get = self.get;
        let self_set = self.set;
        let other_get = other.get;
        let other_set = other.set;

        let self_get_2 = self_get.clone();

        Lens::new(
            move |source: &S| {
                let intermediate = self_get(source);
                other_get(&intermediate)
            },
            move |source: &S, value: B| {
                let intermediate = self_get_2(source);
                let updated_intermediate = other_set(&intermediate, value);
                self_set(source, updated_intermediate)
            },
        )
    }
}

/// An Optional focuses on a part A of a structure S that may not exist.
///
/// Like Lens, but the focused value may be None (e.g., nullable fields in JavaScript).
pub struct Optional<S, A>
where
    S: Clone,
    A: Clone,
{
    get: OptionalGetter<S, A>,
    set: SetterFn<S, A>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> Optional<S, A>
where
    S: Clone,
    A: Clone,
{
    /// Create a new optional from getter and setter functions.
    pub fn new<G, S2>(get_fn: G, set_fn: S2) -> Self
    where
        G: Fn(&S) -> Option<A> + 'static,
        S2: Fn(&S, A) -> S + 'static,
    {
        Optional {
            get: Rc::new(get_fn),
            set: Rc::new(set_fn),
            _phantom: PhantomData,
        }
    }

    /// Extract the focused value, which may not exist.
    pub fn get(&self, source: &S) -> Option<A> {
        (self.get)(source)
    }

    /// Extract the focused value with a default if it doesn't exist.
    pub fn get_or(&self, source: &S, default: A) -> A {
        self.get(source).unwrap_or(default)
    }

    /// Update the focused value immutably if it exists.
    pub fn set(&self, source: &S, value: A) -> S {
        (self.set)(source, value)
    }

    /// Transform the focused value using a function, only if it exists.
    ///
    /// If the value doesn't exist, returns the source unchanged.
    pub fn over<F>(&self, source: &S, f: F) -> S
    where
        F: Fn(A) -> A,
    {
        match self.get(source) {
            Some(current) => {
                let updated = f(current);
                self.set(source, updated)
            }
            None => source.clone(),
        }
    }

    /// Convert this Optional into a read-only Fold.
    pub fn to_fold(&self) -> Fold<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let get = self.get.clone();
        Fold::new(move |s: &S| get(s).into_iter().collect())
    }
}

/// A Prism focuses on a variant A of a sum type S.
///
/// A Prism is defined by two functions:
/// - `preview: &S -> Option<A>` - Try to extract the focused variant
/// - `review: A -> S` - Construct the sum type from the variant
///
/// ## Prism Laws
///
/// 1. **Preview-Review**: `preview(review(a)) = Some(a)` — round-trip through review then preview
/// 2. **Review-Preview**: If `preview(s) = Some(a)`, then `review(a)` reconstructs `s`
///    (modulo other fields — strictly: `preview(s).map(review) = preview(s).map(|_| s)` for simple prisms)
///
/// ## Usage
///
/// ```rust
/// use orlando_transducers::optics::Prism;
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum Shape {
///     Circle(f64),
///     Rectangle(f64, f64),
/// }
///
/// let circle_prism = Prism::new(
///     |s: &Shape| match s {
///         Shape::Circle(r) => Some(*r),
///         _ => None,
///     },
///     |r: f64| Shape::Circle(r),
/// );
///
/// let shape = Shape::Circle(5.0);
/// assert_eq!(circle_prism.preview(&shape), Some(5.0));
/// assert_eq!(circle_prism.review(3.0), Shape::Circle(3.0));
///
/// let rect = Shape::Rectangle(2.0, 3.0);
/// assert_eq!(circle_prism.preview(&rect), None);
/// ```
pub struct Prism<S, A>
where
    S: Clone,
    A: Clone,
{
    preview_fn: OptionalGetter<S, A>,
    review_fn: ReviewerFn<S, A>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> Prism<S, A>
where
    S: Clone,
    A: Clone,
{
    /// Create a new prism from preview and review functions.
    pub fn new<P, R>(preview_fn: P, review_fn: R) -> Self
    where
        P: Fn(&S) -> Option<A> + 'static,
        R: Fn(A) -> S + 'static,
    {
        Prism {
            preview_fn: Rc::new(preview_fn),
            review_fn: Rc::new(review_fn),
            _phantom: PhantomData,
        }
    }

    /// Try to extract the focused variant from the sum type.
    pub fn preview(&self, source: &S) -> Option<A> {
        (self.preview_fn)(source)
    }

    /// Construct the sum type from the focused variant.
    pub fn review(&self, value: A) -> S {
        (self.review_fn)(value)
    }

    /// Transform the focused variant using a function, if it matches.
    ///
    /// If the value doesn't match, returns the source unchanged.
    pub fn over<F>(&self, source: &S, f: F) -> S
    where
        F: Fn(A) -> A,
    {
        match self.preview(source) {
            Some(a) => self.review(f(a)),
            None => source.clone(),
        }
    }

    /// Apply this prism as a profunctor transformer.
    ///
    /// A prism is a `Choice` profunctor transformer: given `P<A, A>`, produce `P<S, S>`.
    pub fn transform<P: Choice>(&self, pab: P::P<A, A>) -> P::P<S, S>
    where
        S: 'static,
        A: 'static,
    {
        let preview = self.preview_fn.clone();
        let review = self.review_fn.clone();
        P::dimap(
            move |s: S| match preview(&s) {
                Some(a) => Ok(a),
                None => Err(s),
            },
            move |r: Result<A, S>| match r {
                Ok(a) => review(a),
                Err(s) => s,
            },
            P::left::<A, A, S>(pab),
        )
    }

    /// Convert this Prism into a Traversal (a Prism is a zero-or-one focus Traversal).
    pub fn to_traversal(&self) -> Traversal<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let preview = self.preview_fn.clone();
        let preview2 = self.preview_fn.clone();
        let review = self.review_fn.clone();
        Traversal::new(
            move |s: &S| preview(s).into_iter().collect(),
            move |s: &S, f: &dyn Fn(A) -> A| match preview2(s) {
                Some(a) => review(f(a)),
                None => s.clone(),
            },
        )
    }

    /// Convert this Prism into a read-only Fold.
    pub fn to_fold(&self) -> Fold<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let preview = self.preview_fn.clone();
        Fold::new(move |s: &S| preview(s).into_iter().collect())
    }
}

/// An Iso represents a lossless bidirectional conversion between types S and A.
///
/// An Iso is defined by two functions:
/// - `to: &S -> A` - Convert from S to A
/// - `from: A -> S` - Convert from A to S
///
/// ## Iso Laws
///
/// 1. **Round-trip forward**: `from(to(s)) = s` — converting and back is identity
/// 2. **Round-trip backward**: `to(from(a)) = a` — converting and back is identity
///
/// ## Usage
///
/// ```rust
/// use orlando_transducers::optics::Iso;
///
/// // Celsius ↔ Fahrenheit
/// let celsius_fahrenheit = Iso::new(
///     |c: &f64| *c * 9.0 / 5.0 + 32.0,
///     |f: f64| (f - 32.0) * 5.0 / 9.0,
/// );
///
/// let c = 100.0;
/// let f = celsius_fahrenheit.to(&c);
/// assert!((f - 212.0).abs() < 1e-10);
///
/// let back = celsius_fahrenheit.from(f);
/// assert!((back - c).abs() < 1e-10);
/// ```
pub struct Iso<S, A>
where
    S: Clone,
    A: Clone,
{
    to_fn: GetterFn<S, A>,
    from_fn: ReviewerFn<S, A>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> Iso<S, A>
where
    S: Clone,
    A: Clone,
{
    /// Create a new isomorphism from to and from functions.
    pub fn new<T, F>(to_fn: T, from_fn: F) -> Self
    where
        T: Fn(&S) -> A + 'static,
        F: Fn(A) -> S + 'static,
    {
        Iso {
            to_fn: Rc::new(to_fn),
            from_fn: Rc::new(from_fn),
            _phantom: PhantomData,
        }
    }

    /// Convert from S to A.
    pub fn to(&self, source: &S) -> A {
        (self.to_fn)(source)
    }

    /// Convert from A to S.
    pub fn from(&self, value: A) -> S {
        (self.from_fn)(value)
    }

    /// Transform via the isomorphism: convert to A, apply f, convert back to S.
    pub fn over<F>(&self, source: &S, f: F) -> S
    where
        F: Fn(A) -> A,
    {
        let a = self.to(source);
        self.from(f(a))
    }

    /// Apply this iso as a profunctor transformer.
    ///
    /// An iso only requires `Profunctor` (the weakest constraint — just `dimap`).
    pub fn transform<P: Profunctor>(&self, pab: P::P<A, A>) -> P::P<S, S>
    where
        S: 'static,
        A: 'static,
    {
        let to = self.to_fn.clone();
        let from = self.from_fn.clone();
        P::dimap(move |s: S| to(&s), move |a: A| from(a), pab)
    }

    /// Reverse the isomorphism: produce `Iso<A, S>`.
    pub fn reverse(self) -> Iso<A, S>
    where
        S: 'static,
        A: 'static,
    {
        let original_to = self.to_fn;
        let original_from = self.from_fn;

        Iso {
            to_fn: Rc::new(move |a: &A| (original_from)(a.clone())),
            from_fn: Rc::new(move |s: S| (original_to)(&s)),
            _phantom: PhantomData,
        }
    }

    /// Convert this Iso into a Lens (every Iso is a valid Lens).
    pub fn as_lens(self) -> Lens<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let to_fn = self.to_fn;
        let from_fn = self.from_fn;

        Lens::new(move |s: &S| to_fn(s), move |_s: &S, a: A| from_fn(a))
    }

    /// Convert this Iso into a Prism (every Iso is a valid Prism).
    pub fn as_prism(self) -> Prism<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let to_fn = self.to_fn;
        let from_fn = self.from_fn;
        Prism::new(move |s: &S| Some(to_fn(s)), move |a: A| from_fn(a))
    }

    /// Convert this Iso into a Traversal (every Iso is a single-focus Traversal).
    pub fn to_traversal(&self) -> Traversal<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let to1 = self.to_fn.clone();
        let to2 = self.to_fn.clone();
        let from = self.from_fn.clone();
        Traversal::new(
            move |s: &S| vec![to1(s)],
            move |s: &S, f: &dyn Fn(A) -> A| from(f(to2(s))),
        )
    }

    /// Convert this Iso into a read-only Fold.
    pub fn to_fold(&self) -> Fold<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let to = self.to_fn.clone();
        Fold::new(move |s: &S| vec![to(s)])
    }
}

/// A Fold extracts zero or more values of type A from a structure S (read-only).
///
/// A Fold is the read-only counterpart of a Traversal. It cannot modify the structure.
///
/// ## Usage
///
/// ```rust
/// use orlando_transducers::optics::Fold;
///
/// let even_fold = Fold::new(
///     |v: &Vec<i32>| v.iter().filter(|x| *x % 2 == 0).cloned().collect(),
/// );
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// assert_eq!(even_fold.fold_of(&data), vec![2, 4, 6]);
/// ```
pub struct Fold<S, A>
where
    S: Clone,
    A: Clone,
{
    fold_fn: FoldGetter<S, A>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> Fold<S, A>
where
    S: Clone,
    A: Clone,
{
    /// Create a new fold from an extraction function.
    pub fn new<F>(fold_fn: F) -> Self
    where
        F: Fn(&S) -> Vec<A> + 'static,
    {
        Fold {
            fold_fn: Rc::new(fold_fn),
            _phantom: PhantomData,
        }
    }

    /// Extract all focused values from the structure.
    pub fn fold_of(&self, source: &S) -> Vec<A> {
        (self.fold_fn)(source)
    }

    /// Check if the fold finds any values.
    pub fn is_empty(&self, source: &S) -> bool {
        self.fold_of(source).is_empty()
    }

    /// Count the number of focused values.
    pub fn length(&self, source: &S) -> usize {
        self.fold_of(source).len()
    }

    /// Find the first focused value, if any.
    pub fn first(&self, source: &S) -> Option<A> {
        self.fold_of(source).into_iter().next()
    }

    /// Apply a function to each focused value and combine the results using `Monoid`.
    pub fn fold_map<R: karpal_core::Monoid>(&self, source: &S, f: impl Fn(A) -> R) -> R {
        self.fold_of(source)
            .into_iter()
            .map(&f)
            .fold(R::empty(), R::combine)
    }

    /// Check if any focused value satisfies the predicate.
    pub fn any(&self, source: &S, f: impl Fn(&A) -> bool) -> bool {
        self.fold_of(source).iter().any(f)
    }

    /// Check if all focused values satisfy the predicate.
    pub fn all(&self, source: &S, f: impl Fn(&A) -> bool) -> bool {
        self.fold_of(source).iter().all(f)
    }

    /// Find the first focused value satisfying a predicate.
    pub fn find(&self, source: &S, f: impl Fn(&A) -> bool) -> Option<A> {
        self.fold_of(source).into_iter().find(|a| f(a))
    }

    /// Compose with another Fold, focusing deeper.
    pub fn then<B>(self, inner: Fold<A, B>) -> Fold<S, B>
    where
        S: 'static,
        A: 'static,
        B: Clone + 'static,
    {
        let outer = self.fold_fn;
        let inner = inner.fold_fn;
        Fold::new(move |s: &S| outer(s).into_iter().flat_map(|a| inner(&a)).collect())
    }
}

/// A Traversal focuses on zero or more values of type A within a structure S,
/// supporting both reading and writing.
///
/// ## Traversal Laws
///
/// 1. **Identity**: `over_all(s, id) = s` — traversing with identity is no-op
/// 2. **Composition**: `over_all(over_all(s, f), g) = over_all(s, g ∘ f)`
///
/// ## Usage
///
/// ```rust
/// use orlando_transducers::optics::Traversal;
///
/// let each = Traversal::new(
///     |v: &Vec<i32>| v.clone(),
///     |v: &Vec<i32>, f: &dyn Fn(i32) -> i32| v.iter().map(|x| f(*x)).collect(),
/// );
///
/// let data = vec![1, 2, 3];
/// assert_eq!(each.get_all(&data), vec![1, 2, 3]);
/// assert_eq!(each.over_all(&data, |x| x * 2), vec![2, 4, 6]);
/// ```
pub struct Traversal<S, A>
where
    S: Clone,
    A: Clone,
{
    get_all_fn: FoldGetter<S, A>,
    over_all_fn: TraversalSetter<S, A>,
    _phantom: PhantomData<(S, A)>,
}

impl<S, A> Traversal<S, A>
where
    S: Clone,
    A: Clone,
{
    /// Create a new traversal from get_all and over_all functions.
    pub fn new<G, O>(get_all_fn: G, over_all_fn: O) -> Self
    where
        G: Fn(&S) -> Vec<A> + 'static,
        O: Fn(&S, &dyn Fn(A) -> A) -> S + 'static,
    {
        Traversal {
            get_all_fn: Rc::new(get_all_fn),
            over_all_fn: Rc::new(over_all_fn),
            _phantom: PhantomData,
        }
    }

    /// Extract all focused values from the structure.
    pub fn get_all(&self, source: &S) -> Vec<A> {
        (self.get_all_fn)(source)
    }

    /// Transform all focused values using a function.
    pub fn over_all<F>(&self, source: &S, f: F) -> S
    where
        F: Fn(A) -> A,
    {
        (self.over_all_fn)(source, &f)
    }

    /// Set all focused values to a single value.
    pub fn set_all(&self, source: &S, value: A) -> S
    where
        A: 'static,
    {
        let v = value;
        (self.over_all_fn)(source, &move |_| v.clone())
    }

    /// Apply this traversal as a profunctor transformer.
    ///
    /// A traversal is a `Traversing` profunctor transformer: given `P<A, A>`, produce `P<S, S>`.
    pub fn transform<P: Traversing>(&self, pab: P::P<A, A>) -> P::P<S, S>
    where
        S: 'static,
        A: 'static,
    {
        let get_all = self.get_all_fn.clone();
        let over_all = self.over_all_fn.clone();
        P::wander(
            move |s: &S| get_all(s),
            move |s: S, f: &dyn Fn(A) -> A| over_all(&s, f),
            pab,
        )
    }

    /// Convert this Traversal into a read-only Fold.
    pub fn as_fold(self) -> Fold<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let get_all = self.get_all_fn;
        Fold::new(move |s: &S| get_all(s))
    }

    /// Convert this Traversal into a read-only Fold (non-consuming).
    pub fn to_fold(&self) -> Fold<S, A>
    where
        S: 'static,
        A: 'static,
    {
        let get_all = self.get_all_fn.clone();
        Fold::new(move |s: &S| get_all(s))
    }

    /// Compose with another Traversal, focusing deeper.
    pub fn then<B>(self, inner: Traversal<A, B>) -> Traversal<S, B>
    where
        S: 'static,
        A: 'static,
        B: Clone + 'static,
    {
        let outer_get = self.get_all_fn;
        let outer_over = self.over_all_fn;
        let inner_get = inner.get_all_fn;
        let inner_over = inner.over_all_fn;

        Traversal::new(
            move |s: &S| {
                outer_get(s)
                    .into_iter()
                    .flat_map(|a| inner_get(&a))
                    .collect()
            },
            move |s: &S, f: &dyn Fn(B) -> B| {
                let inner_ov = inner_over.clone();
                outer_over(s, &move |a: A| inner_ov(&a, f))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    use proptest::prelude::*;

    #[derive(Clone, Debug, PartialEq)]
    struct User {
        name: String,
        age: u32,
        address: Option<Address>,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Address {
        city: String,
        zip: String,
    }

    #[test]
    fn test_lens_get() {
        let name_lens = Lens::new(
            |user: &User| user.name.clone(),
            |user: &User, name: String| User {
                name,
                age: user.age,
                address: user.address.clone(),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        assert_eq!(name_lens.get(&user), "Alice");
    }

    #[test]
    fn test_lens_set() {
        let name_lens = Lens::new(
            |user: &User| user.name.clone(),
            |user: &User, name: String| User {
                name,
                age: user.age,
                address: user.address.clone(),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let updated = name_lens.set(&user, "Bob".to_string());
        assert_eq!(updated.name, "Bob");
        assert_eq!(updated.age, 30);
        assert_eq!(user.name, "Alice"); // Original unchanged
    }

    #[test]
    fn test_lens_over() {
        let name_lens = Lens::new(
            |user: &User| user.name.clone(),
            |user: &User, name: String| User {
                name,
                age: user.age,
                address: user.address.clone(),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let updated = name_lens.over(&user, |name| name.to_uppercase());
        assert_eq!(updated.name, "ALICE");
    }

    #[test]
    fn test_lens_law_get_put() {
        // Law 1: set(s, get(s)) = s
        let name_lens = Lens::new(
            |user: &User| user.name.clone(),
            |user: &User, name: String| User {
                name,
                age: user.age,
                address: user.address.clone(),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let result = name_lens.set(&user, name_lens.get(&user));
        assert_eq!(result, user);
    }

    #[test]
    fn test_lens_law_put_get() {
        // Law 2: get(set(s, a)) = a
        let name_lens = Lens::new(
            |user: &User| user.name.clone(),
            |user: &User, name: String| User {
                name,
                age: user.age,
                address: user.address.clone(),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let new_name = "Bob".to_string();
        let updated = name_lens.set(&user, new_name.clone());
        assert_eq!(name_lens.get(&updated), new_name);
    }

    #[test]
    fn test_lens_law_put_put() {
        // Law 3: set(set(s, a1), a2) = set(s, a2)
        let name_lens = Lens::new(
            |user: &User| user.name.clone(),
            |user: &User, name: String| User {
                name,
                age: user.age,
                address: user.address.clone(),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let name1 = "Bob".to_string();
        let name2 = "Charlie".to_string();

        let result1 = name_lens.set(&name_lens.set(&user, name1), name2.clone());
        let result2 = name_lens.set(&user, name2);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_lens_composition() {
        let address_lens = Lens::new(
            |user: &User| user.address.clone().unwrap(),
            |user: &User, address: Address| User {
                name: user.name.clone(),
                age: user.age,
                address: Some(address),
            },
        );

        let city_lens = Lens::new(
            |addr: &Address| addr.city.clone(),
            |addr: &Address, city: String| Address {
                city,
                zip: addr.zip.clone(),
            },
        );

        let user_city_lens = address_lens.compose(city_lens);

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: Some(Address {
                city: "NYC".to_string(),
                zip: "10001".to_string(),
            }),
        };

        let city = user_city_lens.get(&user);
        assert_eq!(city, "NYC");

        let updated = user_city_lens.set(&user, "Boston".to_string());
        assert_eq!(updated.address.unwrap().city, "Boston");
    }

    #[test]
    fn test_optional_get_some() {
        let address_lens = Optional::new(
            |user: &User| user.address.clone(),
            |user: &User, address: Address| User {
                name: user.name.clone(),
                age: user.age,
                address: Some(address),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: Some(Address {
                city: "NYC".to_string(),
                zip: "10001".to_string(),
            }),
        };

        assert_eq!(
            address_lens.get(&user),
            Some(Address {
                city: "NYC".to_string(),
                zip: "10001".to_string()
            })
        );
    }

    #[test]
    fn test_optional_get_none() {
        let address_lens = Optional::new(
            |user: &User| user.address.clone(),
            |user: &User, address: Address| User {
                name: user.name.clone(),
                age: user.age,
                address: Some(address),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        assert_eq!(address_lens.get(&user), None);
    }

    #[test]
    fn test_optional_get_or() {
        let address_lens = Optional::new(
            |user: &User| user.address.clone(),
            |user: &User, address: Address| User {
                name: user.name.clone(),
                age: user.age,
                address: Some(address),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let default = Address {
            city: "Unknown".to_string(),
            zip: "00000".to_string(),
        };

        assert_eq!(address_lens.get_or(&user, default.clone()), default);
    }

    #[test]
    fn test_optional_over_some() {
        let address_lens = Optional::new(
            |user: &User| user.address.clone(),
            |user: &User, address: Address| User {
                name: user.name.clone(),
                age: user.age,
                address: Some(address),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: Some(Address {
                city: "NYC".to_string(),
                zip: "10001".to_string(),
            }),
        };

        let updated = address_lens.over(&user, |addr| Address {
            city: addr.city.to_uppercase(),
            zip: addr.zip,
        });

        assert_eq!(updated.address.unwrap().city, "NYC");
    }

    #[test]
    fn test_optional_over_none() {
        let address_lens = Optional::new(
            |user: &User| user.address.clone(),
            |user: &User, address: Address| User {
                name: user.name.clone(),
                age: user.age,
                address: Some(address),
            },
        );

        let user = User {
            name: "Alice".to_string(),
            age: 30,
            address: None,
        };

        let updated = address_lens.over(&user, |addr| Address {
            city: addr.city.to_uppercase(),
            zip: addr.zip,
        });

        assert_eq!(updated, user); // Unchanged when None
    }

    // Property-based tests for lens laws
    #[cfg(not(target_arch = "wasm32"))]
    mod lens_laws_properties {
        use super::*;

        // Strategy to generate arbitrary User instances
        fn arbitrary_user() -> impl Strategy<Value = User> {
            (any::<String>(), 1u32..100u32, any::<Option<String>>()).prop_map(
                |(name, age, city_opt)| User {
                    name,
                    age,
                    address: city_opt.map(|city| Address {
                        city,
                        zip: "12345".to_string(),
                    }),
                },
            )
        }

        proptest! {
            /// Law 1: GetPut - Setting what you got changes nothing
            /// For all s: S, set(s, get(s)) = s
            #[test]
            fn prop_lens_law_get_put(user in arbitrary_user()) {
                let name_lens = Lens::new(
                    |user: &User| user.name.clone(),
                    |user: &User, name: String| User {
                        name,
                        age: user.age,
                        address: user.address.clone(),
                    },
                );

                let result = name_lens.set(&user, name_lens.get(&user));
                prop_assert_eq!(result, user);
            }

            /// Law 2: PutGet - Getting what you set returns that value
            /// For all s: S, a: A, get(set(s, a)) = a
            #[test]
            fn prop_lens_law_put_get(user in arbitrary_user(), new_name in any::<String>()) {
                let name_lens = Lens::new(
                    |user: &User| user.name.clone(),
                    |user: &User, name: String| User {
                        name,
                        age: user.age,
                        address: user.address.clone(),
                    },
                );

                let updated = name_lens.set(&user, new_name.clone());
                prop_assert_eq!(name_lens.get(&updated), new_name);
            }

            /// Law 3: PutPut - Setting twice equals setting once (last write wins)
            /// For all s: S, a1: A, a2: A, set(set(s, a1), a2) = set(s, a2)
            #[test]
            fn prop_lens_law_put_put(
                user in arbitrary_user(),
                name1 in any::<String>(),
                name2 in any::<String>()
            ) {
                let name_lens = Lens::new(
                    |user: &User| user.name.clone(),
                    |user: &User, name: String| User {
                        name,
                        age: user.age,
                        address: user.address.clone(),
                    },
                );

                let result1 = name_lens.set(&name_lens.set(&user, name1), name2.clone());
                let result2 = name_lens.set(&user, name2);

                prop_assert_eq!(result1, result2);
            }

            /// Test that `over` is equivalent to get + transform + set
            #[test]
            fn prop_lens_over_equivalence(user in arbitrary_user()) {
                let name_lens = Lens::new(
                    |user: &User| user.name.clone(),
                    |user: &User, name: String| User {
                        name,
                        age: user.age,
                        address: user.address.clone(),
                    },
                );

                let transform = |s: String| s.to_uppercase();

                // over should be equivalent to: set(s, f(get(s)))
                let result_over = name_lens.over(&user, transform);
                let result_manual = name_lens.set(&user, transform(name_lens.get(&user)));

                prop_assert_eq!(result_over, result_manual);
            }

            /// Test lens composition laws
            #[test]
            fn prop_lens_composition_get(user in arbitrary_user()) {
                // Skip if no address using prop_assume!
                prop_assume!(user.address.is_some());

                let address_lens = Lens::new(
                    |user: &User| user.address.clone().unwrap(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                let city_lens = Lens::new(
                    |addr: &Address| addr.city.clone(),
                    |addr: &Address, city: String| Address {
                        city,
                        zip: addr.zip.clone(),
                    },
                );

                let composed = address_lens.compose(city_lens);

                // Composed get should equal nested gets
                let composed_get = composed.get(&user);
                let manual_get = user.address.as_ref().unwrap().city.clone();

                prop_assert_eq!(composed_get, manual_get);
            }

            /// Test lens composition set
            #[test]
            fn prop_lens_composition_set(user in arbitrary_user(), new_city in any::<String>()) {
                // Skip if no address using prop_assume!
                prop_assume!(user.address.is_some());

                let address_lens = Lens::new(
                    |user: &User| user.address.clone().unwrap(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                let city_lens = Lens::new(
                    |addr: &Address| addr.city.clone(),
                    |addr: &Address, city: String| Address {
                        city,
                        zip: addr.zip.clone(),
                    },
                );

                let composed = address_lens.compose(city_lens);

                // Composed set should update nested value
                let updated = composed.set(&user, new_city.clone());
                prop_assert_eq!(updated.address.as_ref().unwrap().city.clone(), new_city);

                // Other fields should remain unchanged
                prop_assert_eq!(updated.name, user.name);
                prop_assert_eq!(updated.age, user.age);
                prop_assert_eq!(
                    updated.address.as_ref().unwrap().zip.clone(),
                    user.address.as_ref().unwrap().zip.clone()
                );
            }
        }
    }

    // Property-based tests for Optional laws
    #[cfg(not(target_arch = "wasm32"))]
    mod optional_laws_properties {
        use super::*;

        fn arbitrary_user() -> impl Strategy<Value = User> {
            (any::<String>(), 1u32..100u32, any::<Option<String>>()).prop_map(
                |(name, age, city_opt)| User {
                    name,
                    age,
                    address: city_opt.map(|city| Address {
                        city,
                        zip: "12345".to_string(),
                    }),
                },
            )
        }

        proptest! {
            /// Test Optional.get returns None when field is None
            #[test]
            fn prop_optional_get_none(name in any::<String>(), age in 1u32..100u32) {
                let user = User {
                    name,
                    age,
                    address: None,
                };

                let address_lens = Optional::new(
                    |user: &User| user.address.clone(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                prop_assert_eq!(address_lens.get(&user), None);
            }

            /// Test Optional.get returns Some when field exists
            #[test]
            fn prop_optional_get_some(user in arbitrary_user()) {
                prop_assume!(user.address.is_some());

                let address_lens = Optional::new(
                    |user: &User| user.address.clone(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                prop_assert_eq!(address_lens.get(&user), user.address);
            }

            /// Test Optional.over is no-op when value is None
            #[test]
            fn prop_optional_over_none(name in any::<String>(), age in 1u32..100u32) {
                let user = User {
                    name,
                    age,
                    address: None,
                };

                let address_lens = Optional::new(
                    |user: &User| user.address.clone(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                let updated = address_lens.over(&user, |addr| Address {
                    city: addr.city.to_uppercase(),
                    zip: addr.zip,
                });

                prop_assert_eq!(updated, user);
            }

            /// Test Optional.over transforms value when Some
            #[test]
            fn prop_optional_over_some(user in arbitrary_user()) {
                prop_assume!(user.address.is_some());

                let address_lens = Optional::new(
                    |user: &User| user.address.clone(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                let updated = address_lens.over(&user, |addr| Address {
                    city: addr.city.to_uppercase(),
                    zip: addr.zip.clone(),
                });

                prop_assert_eq!(
                    updated.address.as_ref().unwrap().city.clone(),
                    user.address.as_ref().unwrap().city.to_uppercase()
                );
            }

            /// Test Optional.get_or returns default when None
            #[test]
            fn prop_optional_get_or_none(name in any::<String>(), age in 1u32..100u32, default_city in any::<String>()) {
                let user = User {
                    name,
                    age,
                    address: None,
                };

                let default_addr = Address {
                    city: default_city,
                    zip: "00000".to_string(),
                };

                let address_lens = Optional::new(
                    |user: &User| user.address.clone(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                prop_assert_eq!(address_lens.get_or(&user, default_addr.clone()), default_addr);
            }

            /// Test Optional.get_or returns value when Some
            #[test]
            fn prop_optional_get_or_some(user in arbitrary_user(), default_city in any::<String>()) {
                prop_assume!(user.address.is_some());

                let default_addr = Address {
                    city: default_city,
                    zip: "00000".to_string(),
                };

                let address_lens = Optional::new(
                    |user: &User| user.address.clone(),
                    |user: &User, address: Address| User {
                        name: user.name.clone(),
                        age: user.age,
                        address: Some(address),
                    },
                );

                prop_assert_eq!(
                    address_lens.get_or(&user, default_addr),
                    user.address.clone().unwrap()
                );
            }
        }
    }

    // ===== Prism tests =====

    #[derive(Clone, Debug, PartialEq)]
    enum Shape {
        Circle(f64),
        Rectangle(f64, f64),
        Triangle(f64, f64, f64),
    }

    fn circle_prism() -> Prism<Shape, f64> {
        Prism::new(
            |s: &Shape| match s {
                Shape::Circle(r) => Some(*r),
                _ => None,
            },
            |r: f64| Shape::Circle(r),
        )
    }

    fn rectangle_prism() -> Prism<Shape, (f64, f64)> {
        Prism::new(
            |s: &Shape| match s {
                Shape::Rectangle(w, h) => Some((*w, *h)),
                _ => None,
            },
            |(w, h): (f64, f64)| Shape::Rectangle(w, h),
        )
    }

    #[test]
    fn test_prism_preview_match() {
        let prism = circle_prism();
        assert_eq!(prism.preview(&Shape::Circle(5.0)), Some(5.0));
    }

    #[test]
    fn test_prism_preview_no_match() {
        let prism = circle_prism();
        assert_eq!(prism.preview(&Shape::Rectangle(2.0, 3.0)), None);
    }

    #[test]
    fn test_prism_review() {
        let prism = circle_prism();
        assert_eq!(prism.review(5.0), Shape::Circle(5.0));
    }

    #[test]
    fn test_prism_over_match() {
        let prism = circle_prism();
        let shape = Shape::Circle(5.0);
        let updated = prism.over(&shape, |r| r * 2.0);
        assert_eq!(updated, Shape::Circle(10.0));
    }

    #[test]
    fn test_prism_over_no_match() {
        let prism = circle_prism();
        let shape = Shape::Rectangle(2.0, 3.0);
        let updated = prism.over(&shape, |r| r * 2.0);
        assert_eq!(updated, shape); // unchanged
    }

    #[test]
    fn test_prism_law_preview_review() {
        // Law: preview(review(a)) = Some(a)
        let prism = circle_prism();
        let radius = 42.0;
        assert_eq!(prism.preview(&prism.review(radius)), Some(radius));
    }

    #[test]
    fn test_prism_law_review_preview() {
        // Law: if preview(s) = Some(a), then review(a) = s
        let prism = circle_prism();
        let shape = Shape::Circle(7.5);
        if let Some(a) = prism.preview(&shape) {
            assert_eq!(prism.review(a), shape);
        }
    }

    #[test]
    fn test_prism_rectangle_variant() {
        let prism = rectangle_prism();
        assert_eq!(prism.preview(&Shape::Rectangle(2.0, 3.0)), Some((2.0, 3.0)));
        assert_eq!(prism.preview(&Shape::Circle(1.0)), None);
        assert_eq!(prism.review((4.0, 5.0)), Shape::Rectangle(4.0, 5.0));
    }

    // ===== Iso tests =====

    fn celsius_fahrenheit_iso() -> Iso<f64, f64> {
        Iso::new(
            |c: &f64| *c * 9.0 / 5.0 + 32.0,
            |f: f64| (f - 32.0) * 5.0 / 9.0,
        )
    }

    #[test]
    fn test_iso_to() {
        let iso = celsius_fahrenheit_iso();
        let f = iso.to(&100.0);
        assert!((f - 212.0).abs() < 1e-10);
    }

    #[test]
    fn test_iso_from() {
        let iso = celsius_fahrenheit_iso();
        let c = iso.from(32.0);
        assert!((c - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_iso_law_round_trip_forward() {
        // Law: from(to(s)) = s
        let iso = celsius_fahrenheit_iso();
        let c = 37.5;
        let back = iso.from(iso.to(&c));
        assert!((back - c).abs() < 1e-10);
    }

    #[test]
    fn test_iso_law_round_trip_backward() {
        // Law: to(from(a)) = a
        let iso = celsius_fahrenheit_iso();
        let f = 98.6;
        let back = iso.to(&iso.from(f));
        assert!((back - f).abs() < 1e-10);
    }

    #[test]
    fn test_iso_over() {
        let iso = celsius_fahrenheit_iso();
        // Double the Fahrenheit value, then convert back
        let result = iso.over(&100.0, |f| f * 2.0);
        // 100°C = 212°F, doubled = 424°F, back to °C = (424-32)*5/9 ≈ 217.78
        let expected = (424.0 - 32.0) * 5.0 / 9.0;
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_iso_reverse() {
        let iso = celsius_fahrenheit_iso();
        let reversed = iso.reverse();
        // reversed.to should convert F→C
        let c = reversed.to(&212.0);
        assert!((c - 100.0).abs() < 1e-10);
        // reversed.from should convert C→F
        let f = reversed.from(0.0);
        assert!((f - 32.0).abs() < 1e-10);
    }

    #[test]
    fn test_iso_as_lens() {
        let iso = celsius_fahrenheit_iso();
        let lens = iso.as_lens();
        let f = lens.get(&100.0);
        assert!((f - 212.0).abs() < 1e-10);
        let c = lens.set(&100.0, 32.0);
        assert!((c - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_iso_as_prism() {
        let iso = celsius_fahrenheit_iso();
        let prism = iso.as_prism();
        // Prism preview always succeeds for an Iso
        assert!(prism.preview(&100.0).is_some());
        let f = prism.preview(&100.0).unwrap();
        assert!((f - 212.0).abs() < 1e-10);
    }

    // Integer ↔ String iso for non-float testing
    fn string_len_iso() -> Iso<String, usize> {
        Iso::new(|s: &String| s.len(), |n: usize| "x".repeat(n))
    }

    #[test]
    fn test_iso_string_len() {
        let iso = string_len_iso();
        assert_eq!(iso.to(&"hello".to_string()), 5);
        assert_eq!(iso.from(3), "xxx".to_string());
    }

    // ===== Fold tests =====

    #[test]
    fn test_fold_basic() {
        let even_fold =
            Fold::new(|v: &Vec<i32>| v.iter().filter(|x| *x % 2 == 0).cloned().collect());
        let data = vec![1, 2, 3, 4, 5, 6];
        assert_eq!(even_fold.fold_of(&data), vec![2, 4, 6]);
    }

    #[test]
    fn test_fold_empty() {
        let even_fold =
            Fold::new(|v: &Vec<i32>| v.iter().filter(|x| *x % 2 == 0).cloned().collect());
        let data = vec![1, 3, 5];
        assert_eq!(even_fold.fold_of(&data), Vec::<i32>::new());
        assert!(even_fold.is_empty(&data));
    }

    #[test]
    fn test_fold_length() {
        let all_fold = Fold::new(|v: &Vec<i32>| v.clone());
        let data = vec![1, 2, 3];
        assert_eq!(all_fold.length(&data), 3);
    }

    #[test]
    fn test_fold_first() {
        let even_fold =
            Fold::new(|v: &Vec<i32>| v.iter().filter(|x| *x % 2 == 0).cloned().collect());
        assert_eq!(even_fold.first(&vec![1, 2, 3, 4]), Some(2));
        assert_eq!(even_fold.first(&vec![1, 3, 5]), None);
    }

    #[test]
    fn test_fold_on_struct() {
        // Fold that extracts all names from a team
        #[derive(Clone, Debug)]
        struct Team {
            members: Vec<String>,
        }

        let names_fold = Fold::new(|team: &Team| team.members.clone());
        let team = Team {
            members: vec!["Alice".to_string(), "Bob".to_string()],
        };
        assert_eq!(
            names_fold.fold_of(&team),
            vec!["Alice".to_string(), "Bob".to_string()]
        );
    }

    // ===== Traversal tests =====

    fn vec_traversal() -> Traversal<Vec<i32>, i32> {
        Traversal::new(
            |v: &Vec<i32>| v.clone(),
            |v: &Vec<i32>, f: &dyn Fn(i32) -> i32| v.iter().map(|x| f(*x)).collect(),
        )
    }

    #[test]
    fn test_traversal_get_all() {
        let trav = vec_traversal();
        assert_eq!(trav.get_all(&vec![1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_traversal_over_all() {
        let trav = vec_traversal();
        assert_eq!(trav.over_all(&vec![1, 2, 3], |x| x * 2), vec![2, 4, 6]);
    }

    #[test]
    fn test_traversal_set_all() {
        let trav = vec_traversal();
        assert_eq!(trav.set_all(&vec![1, 2, 3], 0), vec![0, 0, 0]);
    }

    #[test]
    fn test_traversal_law_identity() {
        // Law: over_all(s, id) = s
        let trav = vec_traversal();
        let data = vec![1, 2, 3];
        assert_eq!(trav.over_all(&data, |x| x), data);
    }

    #[test]
    fn test_traversal_law_composition() {
        // Law: over_all(over_all(s, f), g) = over_all(s, g ∘ f)
        let trav = vec_traversal();
        let data = vec![1, 2, 3];

        let f = |x: i32| x * 2;
        let g = |x: i32| x + 10;

        let step_by_step = trav.over_all(&trav.over_all(&data, f), g);
        let composed = trav.over_all(&data, |x| g(f(x)));

        assert_eq!(step_by_step, composed);
    }

    #[test]
    fn test_traversal_empty() {
        let trav = vec_traversal();
        let data: Vec<i32> = vec![];
        assert_eq!(trav.get_all(&data), Vec::<i32>::new());
        assert_eq!(trav.over_all(&data, |x| x * 2), Vec::<i32>::new());
    }

    #[test]
    fn test_traversal_as_fold() {
        let trav = vec_traversal();
        let fold = trav.as_fold();
        assert_eq!(fold.fold_of(&vec![1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_traversal_on_struct() {
        #[derive(Clone, Debug, PartialEq)]
        struct Team {
            members: Vec<String>,
            name: String,
        }

        let members_traversal = Traversal::new(
            |team: &Team| team.members.clone(),
            |team: &Team, f: &dyn Fn(String) -> String| Team {
                members: team.members.iter().map(|m| f(m.clone())).collect(),
                name: team.name.clone(),
            },
        );

        let team = Team {
            members: vec!["alice".to_string(), "bob".to_string()],
            name: "Dev".to_string(),
        };

        assert_eq!(
            members_traversal.get_all(&team),
            vec!["alice".to_string(), "bob".to_string()]
        );

        let updated = members_traversal.over_all(&team, |s| s.to_uppercase());
        assert_eq!(
            updated.members,
            vec!["ALICE".to_string(), "BOB".to_string()]
        );
        assert_eq!(updated.name, "Dev"); // other fields unchanged
    }

    // Property-based tests for Prism laws
    #[cfg(not(target_arch = "wasm32"))]
    mod prism_laws_properties {
        use super::*;

        fn arbitrary_shape() -> impl Strategy<Value = Shape> {
            prop_oneof![
                (0.1f64..1000.0).prop_map(Shape::Circle),
                (0.1f64..1000.0, 0.1f64..1000.0).prop_map(|(w, h)| Shape::Rectangle(w, h)),
                (0.1f64..1000.0, 0.1f64..1000.0, 0.1f64..1000.0)
                    .prop_map(|(a, b, c)| Shape::Triangle(a, b, c)),
            ]
        }

        proptest! {
            /// Prism law: preview(review(a)) = Some(a)
            #[test]
            fn prop_prism_preview_review(radius in 0.1f64..1000.0) {
                let prism = circle_prism();
                prop_assert_eq!(prism.preview(&prism.review(radius)), Some(radius));
            }

            /// Prism law: if preview(s) = Some(a), then review(a) = s
            #[test]
            fn prop_prism_review_preview(shape in arbitrary_shape()) {
                let prism = circle_prism();
                if let Some(a) = prism.preview(&shape) {
                    prop_assert_eq!(prism.review(a), shape);
                }
            }

            /// Prism over on non-matching variant is identity
            #[test]
            fn prop_prism_over_non_match(w in 0.1f64..1000.0, h in 0.1f64..1000.0) {
                let prism = circle_prism();
                let shape = Shape::Rectangle(w, h);
                prop_assert_eq!(prism.over(&shape, |r| r * 2.0), shape);
            }
        }
    }

    // Property-based tests for Iso laws
    #[cfg(not(target_arch = "wasm32"))]
    mod iso_laws_properties {
        use super::*;

        proptest! {
            /// Iso law: from(to(s)) = s (forward round-trip)
            #[test]
            fn prop_iso_round_trip_forward(c in -273.15f64..1000.0) {
                let iso = celsius_fahrenheit_iso();
                let back = iso.from(iso.to(&c));
                prop_assert!((back - c).abs() < 1e-8);
            }

            /// Iso law: to(from(a)) = a (backward round-trip)
            #[test]
            fn prop_iso_round_trip_backward(f in -459.67f64..2000.0) {
                let iso = celsius_fahrenheit_iso();
                let back = iso.to(&iso.from(f));
                prop_assert!((back - f).abs() < 1e-8);
            }

            /// Iso.over is equivalent to from(f(to(s)))
            #[test]
            fn prop_iso_over_equivalence(c in -273.15f64..1000.0) {
                let iso = celsius_fahrenheit_iso();
                let result_over = iso.over(&c, |f| f + 10.0);

                let iso2 = celsius_fahrenheit_iso();
                let result_manual = iso2.from(iso2.to(&c) + 10.0);

                prop_assert!((result_over - result_manual).abs() < 1e-8);
            }
        }
    }

    // Property-based tests for Traversal laws
    #[cfg(not(target_arch = "wasm32"))]
    mod traversal_laws_properties {
        use super::*;

        proptest! {
            /// Traversal law: over_all(s, id) = s
            #[test]
            fn prop_traversal_identity(data in proptest::collection::vec(any::<i32>(), 0..20)) {
                let trav = vec_traversal();
                prop_assert_eq!(trav.over_all(&data, |x| x), data);
            }

            /// Traversal law: over_all(over_all(s, f), g) = over_all(s, g ∘ f)
            #[test]
            fn prop_traversal_composition(data in proptest::collection::vec(-1000i32..1000, 0..20)) {
                let trav = vec_traversal();
                let f = |x: i32| x.saturating_mul(2);
                let g = |x: i32| x.saturating_add(10);

                let step_by_step = trav.over_all(&trav.over_all(&data, f), g);
                let composed = trav.over_all(&data, |x| g(f(x)));

                prop_assert_eq!(step_by_step, composed);
            }

            /// Traversal get_all length is preserved by over_all
            #[test]
            fn prop_traversal_preserves_length(data in proptest::collection::vec(-1000i32..1000, 0..20)) {
                let trav = vec_traversal();
                let updated = trav.over_all(&data, |x| x.saturating_mul(2));
                prop_assert_eq!(trav.get_all(&updated).len(), trav.get_all(&data).len());
            }
        }
    }

    // ===== Profunctor transform tests =====

    mod profunctor_tests {
        use super::*;
        use karpal_profunctor::FnP;

        fn name_lens() -> Lens<User, String> {
            Lens::new(
                |user: &User| user.name.clone(),
                |user: &User, name: String| User {
                    name,
                    age: user.age,
                    address: user.address.clone(),
                },
            )
        }

        fn age_lens() -> Lens<User, u32> {
            Lens::new(
                |user: &User| user.age,
                |user: &User, age: u32| User {
                    name: user.name.clone(),
                    age,
                    address: user.address.clone(),
                },
            )
        }

        fn test_user() -> User {
            User {
                name: "Alice".to_string(),
                age: 30,
                address: None,
            }
        }

        // -- Lens profunctor transform --

        #[test]
        fn test_lens_transform_fnp() {
            let lens = name_lens();
            let user = test_user();

            // FnP transform: given a function A -> A, produce a function S -> S
            let identity: Box<dyn Fn(String) -> String> = Box::new(|s| s);
            let transformed = lens.transform::<FnP>(identity);
            let result = transformed(user.clone());
            assert_eq!(result, user);
        }

        #[test]
        fn test_lens_transform_fnp_modification() {
            let lens = name_lens();
            let user = test_user();

            let uppercase: Box<dyn Fn(String) -> String> = Box::new(|s| s.to_uppercase());
            let transformed = lens.transform::<FnP>(uppercase);
            let result = transformed(user);
            assert_eq!(result.name, "ALICE");
            assert_eq!(result.age, 30);
        }

        #[test]
        fn test_lens_transform_matches_over() {
            let lens = name_lens();
            let user = test_user();

            let f = |s: String| s.to_uppercase();
            let via_over = lens.over(&user, &f);

            let f_boxed: Box<dyn Fn(String) -> String> = Box::new(f);
            let via_transform = (lens.transform::<FnP>(f_boxed))(user);

            assert_eq!(via_over, via_transform);
        }

        // -- Prism profunctor transform --

        #[test]
        fn test_prism_transform_fnp_match() {
            let prism = circle_prism();
            let shape = Shape::Circle(5.0);

            let double: Box<dyn Fn(f64) -> f64> = Box::new(|r| r * 2.0);
            let transformed = prism.transform::<FnP>(double);
            let result = transformed(shape);
            assert_eq!(result, Shape::Circle(10.0));
        }

        #[test]
        fn test_prism_transform_fnp_no_match() {
            let prism = circle_prism();
            let shape = Shape::Rectangle(2.0, 3.0);

            let double: Box<dyn Fn(f64) -> f64> = Box::new(|r| r * 2.0);
            let transformed = prism.transform::<FnP>(double);
            let result = transformed(shape.clone());
            assert_eq!(result, shape);
        }

        #[test]
        fn test_prism_transform_matches_over() {
            let prism = circle_prism();
            let shape = Shape::Circle(5.0);

            let f = |r: f64| r * 3.0;
            let via_over = prism.over(&shape, &f);

            let f_boxed: Box<dyn Fn(f64) -> f64> = Box::new(f);
            let via_transform = (prism.transform::<FnP>(f_boxed))(shape);

            assert_eq!(via_over, via_transform);
        }

        // -- Iso profunctor transform --

        #[test]
        fn test_iso_transform_fnp() {
            let iso = celsius_fahrenheit_iso();

            // Transform in Fahrenheit space: double the value
            let double: Box<dyn Fn(f64) -> f64> = Box::new(|f| f * 2.0);
            let transformed = iso.transform::<FnP>(double);

            // 100°C = 212°F, doubled = 424°F, back to °C
            let result = transformed(100.0);
            let expected = (424.0 - 32.0) * 5.0 / 9.0;
            assert!((result - expected).abs() < 1e-10);
        }

        #[test]
        fn test_iso_transform_matches_over() {
            let iso = celsius_fahrenheit_iso();
            let c = 37.5;

            let f = |x: f64| x + 10.0;
            let via_over = iso.over(&c, &f);

            let f_boxed: Box<dyn Fn(f64) -> f64> = Box::new(f);
            let via_transform = (iso.transform::<FnP>(f_boxed))(c);

            assert!((via_over - via_transform).abs() < 1e-10);
        }

        // -- Traversal profunctor transform --

        #[test]
        fn test_traversal_transform_fnp() {
            let trav = vec_traversal();
            let data = vec![1, 2, 3];

            let double: Box<dyn Fn(i32) -> i32> = Box::new(|x| x * 2);
            let transformed = trav.transform::<FnP>(double);
            let result = transformed(data);
            assert_eq!(result, vec![2, 4, 6]);
        }

        #[test]
        fn test_traversal_transform_matches_over_all() {
            let trav = vec_traversal();
            let data = vec![10, 20, 30];

            let f = |x: i32| x + 1;
            let via_over = trav.over_all(&data, &f);

            let f_boxed: Box<dyn Fn(i32) -> i32> = Box::new(f);
            let via_transform = (trav.transform::<FnP>(f_boxed))(data);

            assert_eq!(via_over, via_transform);
        }

        // -- Cross-type conversion tests --

        #[test]
        fn test_lens_to_traversal() {
            let lens = name_lens();
            let trav = lens.to_traversal();
            let user = test_user();

            assert_eq!(trav.get_all(&user), vec!["Alice".to_string()]);
            let updated = trav.over_all(&user, |s| s.to_uppercase());
            assert_eq!(updated.name, "ALICE");
        }

        #[test]
        fn test_lens_to_fold() {
            let lens = age_lens();
            let fold = lens.to_fold();
            let user = test_user();

            assert_eq!(fold.fold_of(&user), vec![30]);
            assert_eq!(fold.length(&user), 1);
        }

        #[test]
        fn test_prism_to_traversal() {
            let prism = circle_prism();
            let trav = prism.to_traversal();

            let circle = Shape::Circle(5.0);
            assert_eq!(trav.get_all(&circle), vec![5.0]);
            assert_eq!(trav.over_all(&circle, |r| r * 2.0), Shape::Circle(10.0));

            let rect = Shape::Rectangle(2.0, 3.0);
            assert!(trav.get_all(&rect).is_empty());
            assert_eq!(trav.over_all(&rect, |r| r * 2.0), rect);
        }

        #[test]
        fn test_prism_to_fold() {
            let prism = circle_prism();
            let fold = prism.to_fold();

            assert_eq!(fold.fold_of(&Shape::Circle(5.0)), vec![5.0]);
            assert!(fold.fold_of(&Shape::Rectangle(2.0, 3.0)).is_empty());
        }

        #[test]
        fn test_iso_to_traversal() {
            let iso = celsius_fahrenheit_iso();
            let trav = iso.to_traversal();
            let vals = trav.get_all(&100.0);
            assert_eq!(vals.len(), 1);
            assert!((vals[0] - 212.0).abs() < 1e-10);
        }

        #[test]
        fn test_iso_to_fold() {
            let iso = celsius_fahrenheit_iso();
            let fold = iso.to_fold();
            let vals = fold.fold_of(&100.0);
            assert!((vals[0] - 212.0).abs() < 1e-10);
        }

        #[test]
        fn test_optional_to_fold() {
            let opt = Optional::new(
                |user: &User| user.address.clone(),
                |user: &User, address: Address| User {
                    name: user.name.clone(),
                    age: user.age,
                    address: Some(address),
                },
            );

            let user_with = User {
                name: "A".into(),
                age: 1,
                address: Some(Address {
                    city: "NYC".into(),
                    zip: "10001".into(),
                }),
            };
            let user_without = User {
                name: "B".into(),
                age: 2,
                address: None,
            };

            let fold = opt.to_fold();
            assert_eq!(fold.length(&user_with), 1);
            assert_eq!(fold.length(&user_without), 0);
        }

        // -- Composition tests --

        #[test]
        fn test_lens_then() {
            let user = User {
                name: "Alice".to_string(),
                age: 30,
                address: Some(Address {
                    city: "NYC".to_string(),
                    zip: "10001".to_string(),
                }),
            };

            let via_then = Lens::new(
                |user: &User| user.address.clone().unwrap(),
                |user: &User, address: Address| User {
                    name: user.name.clone(),
                    age: user.age,
                    address: Some(address),
                },
            )
            .then(Lens::new(
                |addr: &Address| addr.city.clone(),
                |addr: &Address, city: String| Address {
                    city,
                    zip: addr.zip.clone(),
                },
            ));

            assert_eq!(via_then.get(&user), "NYC");
            let updated = via_then.set(&user, "Boston".to_string());
            assert_eq!(updated.address.unwrap().city, "Boston");
        }

        #[test]
        fn test_fold_then() {
            // Fold that gets all members from a team
            #[derive(Clone, Debug)]
            struct Team {
                members: Vec<String>,
            }

            let members_fold = Fold::new(|team: &Team| team.members.clone());
            let char_fold =
                Fold::new(|s: &String| s.chars().map(|c| c.to_string()).collect::<Vec<_>>());

            let composed = members_fold.then(char_fold);
            let team = Team {
                members: vec!["ab".to_string(), "cd".to_string()],
            };
            assert_eq!(
                composed.fold_of(&team),
                vec![
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string()
                ]
            );
        }

        #[test]
        fn test_traversal_then() {
            // Outer: traverse each inner vec
            let outer = Traversal::new(
                |v: &Vec<Vec<i32>>| v.clone(),
                |v: &Vec<Vec<i32>>, f: &dyn Fn(Vec<i32>) -> Vec<i32>| {
                    v.iter().map(|inner| f(inner.clone())).collect()
                },
            );

            // Inner: traverse each element in an inner vec
            let inner = vec_traversal();

            let composed = outer.then(inner);
            let data = vec![vec![1, 2], vec![3, 4]];

            assert_eq!(composed.get_all(&data), vec![1, 2, 3, 4]);
            let doubled = composed.over_all(&data, |x| x * 2);
            assert_eq!(doubled, vec![vec![2, 4], vec![6, 8]]);
        }

        // -- Fold aggregation tests --

        #[test]
        fn test_fold_any_all_find() {
            let even_fold =
                Fold::new(|v: &Vec<i32>| v.iter().filter(|x| *x % 2 == 0).cloned().collect());

            let data = vec![2, 4, 6, 8];
            assert!(even_fold.any(&data, |x| *x > 5));
            assert!(even_fold.all(&data, |x| *x % 2 == 0));
            assert_eq!(even_fold.find(&data, |x| *x > 5), Some(6));
        }

        #[test]
        fn test_fold_fold_map() {
            let all_fold = Fold::new(|v: &Vec<i32>| v.clone());
            let data = vec![1, 2, 3, 4];

            // Sum via fold_map (i32 Monoid is additive)
            let sum: i32 = all_fold.fold_map(&data, |x| x);
            assert_eq!(sum, 10);
        }

        // -- Traversal to_fold (non-consuming) --

        #[test]
        fn test_traversal_to_fold() {
            let trav = vec_traversal();
            let fold = trav.to_fold();
            assert_eq!(fold.fold_of(&vec![1, 2, 3]), vec![1, 2, 3]);
            // trav is still usable since to_fold borrows
            assert_eq!(trav.get_all(&vec![4, 5]), vec![4, 5]);
        }
    }
}
