// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Profunctor types re-exported from Karpal.
//!
//! Profunctors are the foundation of the profunctor optics encoding.
//! Each optic type constrains which profunctors it can transform:
//!
//! - **Iso** requires `Profunctor` (the weakest constraint — just `dimap`)
//! - **Lens** requires `Strong` (supports first/second on product types)
//! - **Prism** requires `Choice` (supports left/right on sum types)
//! - **Traversal** requires `Traversing` (supports `wander` over multiple foci)
//!
//! Concrete profunctor types:
//!
//! - [`FnP`] — The function-arrow profunctor (`P<A, B> = Box<dyn Fn(A) -> B>`).
//!   Used for `over`/`set` operations via `transform`.
//! - [`ForgetF`] — The read-only profunctor (`P<A, B> = Box<dyn Fn(A) -> R>`).
//!   Used for `get`/`view`/`fold_map` via `transform`.
//! - [`TaggedF`] — The write-only profunctor (`P<A, B> = B`).
//!   Used for `review`/`build` via `transform`.

pub use karpal_core::Monoid;
pub use karpal_profunctor::{Choice, FnP, ForgetF, Profunctor, Strong, TaggedF, Traversing};
