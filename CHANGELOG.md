# Changelog

All notable changes to Orlando will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-06-27

### Added

#### Pipeline Reflection & Inversion (inverse-transducer design)

Orlando pipelines are now **inspectable, serializable, and — for the bijective
subset — reversible**. Three complementary layers:

**Layer C — Reflection (`describe`):**
- New module `src/describe.rs`: `StageSpec` enum (one variant per op, carrying
  params) and the `Describable` trait (non-generic, object-safe, opt-in).
- Implemented by all 15 transforms, `When`/`Unless`/`IfElse`, and
  `Identity`/`Compose` (preserves data-flow order).
- `StageSpec::name()` (stable JS-style names) and `is_reversible()` /
  `is_lossy()` groupoid classification.

**Declarative construction (`pipeline!` macro):**
- New module `src/macros.rs`: `pipeline!(map(f) >> filter(p) >> take(n))` builds
  a chain with no struct imports (fully `$crate`-qualified, UFCS compose).
- Companion `pipeline_descriptor!` emits a compile-time `&'static [StageSpec]`.
- Cross-consistency tests guarantee forward and inverse cannot drift.

**Layer A — Inversion (`Invertible` / `IsoMap`):**
- New module `src/invert.rs`: `Invertible<In,Out>` trait with associated
  `Inverse` type (true groupoid, involution). `IsoMap<F,G,In,Out>` — a streaming
  isomorphism pairing `to`/`from`, the transducer analogue of the `Iso` optic.
- `Compose` inverts via the groupoid law `(a ∘ b)⁻¹ = b⁻¹ ∘ a⁻¹`.
- Lossy transducers lack an `Invertible` impl, so inverting a pipeline
  containing them is a **compile error** — the groupoid excludes them by
  construction. New `StageSpec::IsoMap` variant.

**JS/WASM surface:**
- `Pipeline.describe()` -> serializable stage array.
- `Pipeline.isoMap(to, from)` builder.
- `Pipeline.canInvert()` / `Pipeline.invert()` (throws on lossy pipelines).

**Layer B — Provenance (`trace`):**
- New module `src/provenance.rs`: `trace(transducer, source)` records which
  input index produced each output. `Trace::kept_mask()` is the post-hoc
  "inverse of Filter". Works for any transducer (orthogonal to invertibility).

#### Documentation & Examples
- New mdBook chapters: "Reflection & Inversion" (API) and "Reversible
  Pipelines" (examples).
- New `docs/INVERSE_TRANSDUCER_DESIGN.md` design document.
- Runnable examples: `examples/reflection-inversion.html` (WASM demo),
  `examples/inversion_demo.rs`, `examples/provenance_demo.rs` (Rust binaries).
- README "Pipeline Reflection & Inversion" section.

### Changed

#### Licensing: MIT -> Apache-2.0 + CLA
- Orlando is now licensed under **Apache-2.0** with a Contributor License
  Agreement, per the Industrial Algebra ecosystem standard.
- `LICENSE` replaced with the full Apache-2.0 text.
- All 25 `.rs` files carry the header:
  `// Copyright (C) 2026 Industrial Algebra` / `// SPDX-License-Identifier: Apache-2.0`.
- New `CONTRIBUTING.md` requiring the IA CLA.

#### Cleanup
- Removed dead `src/simd.rs` (`map_f64_simd` was fake SIMD — lane-extract then
  per-scalar — and was never wired into `Pipeline`/collectors; README/lib docs
  had claimed otherwise).
- Synced `package.json` version (was drifting behind `Cargo.toml`).
- Removed stray `pkg-temp`/`pkg-test` build outputs; gitignored build artifacts.

### Testing
- Total tests: 769 (414 unit + 76 integration + 127 property + 152 doctests).
- New tests across all layers; groupoid & cross-consistency laws verified by
  construction. clippy `--all-targets` clean; wasm32 check clean.

## [0.5.0] - 2026-03-09

### Added

#### Phase 6b+: Profunctor Optics via Karpal

- Profunctor optics encoding via Karpal (karpal-core, karpal-profunctor, karpal-optics 0.2)
- `transform()` on Lens (Strong), Prism (Choice), Iso (Profunctor), Traversal (Traversing)
- `then()` composition on Lens, Fold, Traversal
- Cross-type conversions: `to_traversal()`, `to_fold()` on Lens/Prism/Iso/Optional
- `fold_map()`, `any()`, `all()`, `find()` on Fold
- `ComposedLens<S, A>` type alias
- `src/profunctor.rs` — re-exports of Strong, Choice, Traversing, FnP, ForgetF, TaggedF, Monoid
- Re-export of Karpal `Getter`, `Setter`, `Review` types

#### Phase 6b: Advanced Optics

**New Optics Types:**
- `Prism<S, A>` - Focus on sum types / enum variants with `preview()` and `review()`
- `Iso<S, A>` - Lossless bidirectional conversions with `to()` and `from()`
- `Fold<S, A>` - Read-only traversal with aggregation via `fold_of()`
- `Traversal<S, A>` - Collection-level lens with `get_all()` and `over_all()`

**JavaScript API:**
- `prism(matchFn, buildFn)` - Create a prism for tagged unions
- `iso(toFn, fromFn)` - Create an isomorphism
- `fold(extractFn)` - Create a read-only fold
- `traversal(getAllFn, overAllFn)` - Create a traversal for collections
- Property-based law tests for all optics (Prism laws, Iso bijection, Traversal laws)

#### Phase 6g: Geometric Optics

**Multivector coefficient array operations** (operate on plain `&[f64]` / `Float64Array`):

JavaScript API:
- `bladeGrade(index)` - Compute the grade of a basis blade from its index
- `bladesAtGradeCount(dimension, grade)` - Count basis blades at a grade
- `gradeIndices(dimension, grade)` - Get coefficient indices for a grade
- `gradeExtract(dimension, grade, mv)` - Extract coefficients of a specific grade
- `gradeProject(dimension, grade, mv)` - Project onto a single grade
- `gradeProjectMax(dimension, maxGrade, mv)` - Project onto grades up to max
- `gradeMask(dimension, mv)` - Bitmask of which grades are non-zero
- `hasGrade(dimension, grade, mv)` - Check for non-zero grade components
- `isPureGrade(dimension, mv)` - Check if only one grade is non-zero
- `componentGet(mv, bladeIndex)` / `componentSet(mv, bladeIndex, value)` - Single coefficient access
- `mvNorm(mv)` / `mvNormSquared(mv)` - Compute multivector magnitude
- `mvNormalize(mv)` - Normalize to unit length
- `mvReverse(dimension, mv)` - Grade-dependent sign reversal
- `gradeInvolution(dimension, mv)` - Grade involution

Rust API:
- `blade_grade`, `grade_indices`, `grade_extract`, `grade_project`, `grade_project_max`, `grade_mask`, `has_grade`, `is_pure_grade`, `component_get`, `component_set`, `norm`, `norm_squared`, `normalize`, `reverse`, `grade_involution`

#### Phase 5-JS: JavaScript Pipeline Enhancements

- `Pipeline.pluck(key)` - Extract a single property from each object
- `Pipeline.project(keys)` - Extract multiple properties from each object
- `Pipeline.compact()` - Remove all falsy values (null, undefined, false, 0, '', NaN)
- `Pipeline.flatten(depth)` - Flatten nested arrays to a given depth
- `Pipeline.whereMatches(spec)` - Filter objects matching a spec pattern

#### Phase 6c: Optics-Pipeline Integration

- `Pipeline.viewLens(lens)` - Apply a lens inline to extract focused values
- `Pipeline.overLens(lens, fn)` - Transform through a lens in the pipeline
- `Pipeline.filterLens(lens, pred)` - Filter by lens-focused value
- `Pipeline.setLens(lens, value)` - Set lens-focused value on every element

#### Phase 7: Reactive Streams

**Signal<T> - Time-varying values (Rust API):**
- `Signal::new(value)` - Create a reactive signal
- `signal.get()` / `signal.set(value)` / `signal.update(fn)` - Read/write/modify
- `signal.subscribe(callback)` - React to changes
- `signal.map(fn)` - Derived signal that auto-updates
- `signal.combine(other, fn)` - Combine two signals
- `signal.fold(stream, init, fn)` - Fold a stream into a signal

**Stream<T> - Push-based event streams (Rust API):**
- `Stream::new()` - Create an event stream
- `stream.emit(value)` - Push a value
- `stream.subscribe(callback)` - Listen for events
- `stream.map(fn)` / `stream.filter(pred)` / `stream.take(n)` - Streaming operations
- `stream.merge(other)` - Merge two streams
- `stream.fold(init, fn)` - Fold into a Signal (bridges discrete events to continuous values)

#### Phase 8: Rust API Polish

- `TransduceExt` trait - `.transduce(pipeline)` extension method for any iterator
- `TransducedIterator` - Lazy iterator adapter for transducer pipelines
- `PipelineBuilder` - Fluent Rust API: `PipelineBuilder::new().map(f).filter(g).take(n).run(iter)`

#### Testing
- Total tests: 717 (up from 243 in v0.4.0)

## [0.4.0] - 2026-01-07

### Added

#### Phase 6a: Functional Optics (Lenses)

**Core Optics:**
- `Lens<S, A>` - Total focus on nested data with get/set/over operations
- `Optional<S, A>` - Partial focus for nullable fields with safe None handling
- Lens composition via `compose()` for deep nested access
- All three lens laws verified via property-based tests (GetPut, PutGet, PutPut)

**JavaScript API:**
- `lens(property)` - Create a lens focusing on an object property
- `lensPath(path)` - Create a lens for deep nested paths via arrays
- `optional(property)` - Create an optional lens for nullable fields
- `JsLens` methods: `get()`, `set()`, `over()`, `compose()`
- `JsOptional` methods: `get()`, `getOr()`, `set()`, `over()`

**Unique Feature - Streaming Lenses:**
- **First lens library to integrate with transducers** for streaming data processing
- Extract nested values with lenses, transform with transducers in single pass
- Bounded memory processing of large datasets
- Combines functional optics with high-performance streaming

**Testing:**
- 24 new unit tests for Rust lens operations
- 12 property-based tests verifying lens laws automatically
- 14 comprehensive WASM tests for JavaScript API
- Lens composition correctness tests
- Optional Some/None behavior tests

**Documentation:**
- Comprehensive Phase 6 implementation plan
- Competitive analysis of existing lens libraries
- Lens laws documentation with examples
- Streaming lens integration examples
- Real-world use cases (React/Redux state updates, bulk transformations)

#### Infrastructure
- Total tests: 243 (229 unit + 127 property + 64 integration + 111 doc)
- Type aliases for clippy compliance
- Full TypeScript definitions auto-generated

### Changed
- Updated README.md with comprehensive optics section
- Enhanced project structure documentation
- Added optics to API reference tables

## [0.3.0] - 2025-01-24

### Added
- CI/CD pipeline for automated npm publishing
- Comprehensive publishing guide (PUBLISHING.md)
- .npmignore for npm package optimization

### Changed
- Updated repository URLs to actual GitHub repository

## [0.1.0] - 2025-01-23

### Added

#### Phase 1: Core Transducers (10 operations)
- `FlatMap` transducer for transforming and flattening nested structures
- `Partition` collector for splitting data into pass/fail groups
- `Find` collector for early-termination searches
- `Reject` transducer as inverse of filter
- `Chunk` transducer for batching elements
- `GroupBy` collector for aggregating by key function
- `None` collector as inverse of some
- `Contains` collector for membership testing
- `Zip`/`ZipWith` collectors for combining arrays
- JavaScript `pluck` helper for property extraction

#### Phase 2a: Multi-Input Operations (6 operations)
- `Merge` helper for round-robin interleaving
- `Intersection` helper for set intersection
- `Difference` helper for set difference
- `Union` helper for set union
- `SymmetricDifference` helper for XOR operations
- Hybrid composition pattern documentation

#### Phase 2b: Advanced Collectors (8 operations)
- `CartesianProduct` for generating all possible pairs
- `TopK` for efficient top-N queries
- `ReservoirSample` for uniform random sampling
- `PartitionBy` for consecutive grouping
- `Frequencies` for counting occurrences
- `ZipLongest` for combining arrays with fill values
- `Interpose` transducer (RepeatEach) for element repetition
- `Unique`/`UniqueBy` transducers for deduplication

#### Phase 3: Logic Functions (8 operations)
- `both` predicate combinator (AND logic)
- `either` predicate combinator (OR logic)
- `complement` predicate combinator (NOT logic)
- `allPass` combinator for multiple AND conditions
- `anyPass` combinator for multiple OR conditions
- `When` conditional transducer
- `Unless` conditional transducer
- `IfElse` branching transducer

#### Documentation & Examples
- Complete JavaScript/TypeScript API documentation
- API expansion roadmap
- Hybrid composition guide
- Advanced collectors HTML examples
- Logic functions HTML examples
- Performance benchmarks
- Library comparison benchmarks

#### Testing
- 328 total tests
  - 92 unit tests
  - 64 integration tests
  - 107 property-based tests
  - 65 documentation tests
- Property tests verifying mathematical laws
- Comprehensive integration test coverage

### Performance
- Zero intermediate allocations
- Single-pass execution
- Early termination support
- WASM SIMD optimizations for numeric data
- 3-19x faster than native JavaScript arrays

### Infrastructure
- Git hooks for pre-commit and pre-push checks
- Automated formatting (rustfmt)
- Automated linting (clippy)
- Multi-platform CI testing (Ubuntu, macOS, Windows)
- Code coverage reporting
- WASM test suite

[Unreleased]: https://github.com/justinelliottcobb/Orlando/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/justinelliottcobb/Orlando/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/justinelliottcobb/Orlando/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/justinelliottcobb/Orlando/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/justinelliottcobb/Orlando/compare/v0.1.0...v0.3.0
[0.1.0]: https://github.com/justinelliottcobb/Orlando/releases/tag/v0.1.0
