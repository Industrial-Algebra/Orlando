# Reflection & Inversion

Orlando pipelines are **inspectable, serializable, and — for the bijective
subset — reversible**. This chapter covers the three complementary layers that
let you "break down data the way Orlando composes it."

For the full design rationale, see
[`docs/INVERSE_TRANSDUCER_DESIGN.md`](https://github.com/Industrial-Algebra/Orlando/blob/develop/docs/INVERSE_TRANSDUCER_DESIGN.md).

| Layer | Question it answers | Who can use it |
|-------|---------------------|----------------|
| **C — Reflection** | "What stages does this pipeline have?" | Any transducer |
| **A — Inversion** | "Undo this transformation." | The bijective subset only |
| **B — Provenance** | "Which inputs produced these outputs?" | Any transducer (needs the source) |

## Layer C: Reflection (`describe`)

Every built-in transducer implements `Describable` and can report its stages as
a serializable `Vec<StageSpec>`.

### Rust

```rust
use orlando_transducers::{Describable, StageSpec};
use orlando_transducers::transforms::{Map, Filter, Take};
use orlando_transducers::transducer::Transducer;

let p = Map::new(|x: i32| x * 2)
    .compose(Filter::new(|x: &i32| *x > 5))
    .compose(Take::new(3));

assert_eq!(p.describe(), vec![
    StageSpec::Map,
    StageSpec::Filter,
    StageSpec::Take { n: 3 },
]);
```

`StageSpec` carries captured parameters where meaningful (`Take { n }`,
`Chunk { size }`, …) and is structural metadata only — it never includes the
closure body (closures are not serializable). Each variant has a stable
`name()` (e.g. `"map"`, `"takeWhile"`) for UI/serialization, and
`is_reversible()` classifies the groupoid-eligible ops.

### JavaScript

```javascript
const p = new Pipeline().map(x => x * 2).filter(x => x > 5).take(3);
p.describe();
// [ { op: 'map' }, { op: 'filter' }, { op: 'take', n: 3 } ]
```

The descriptor is JSON-serializable, enabling pipeline-as-config and
round-tripping through storage or a UI.

## Declarative construction (`pipeline!` macro)

The `pipeline!` macro builds a chain with natural `>>` syntax and needs no
struct imports (it is fully `$crate`-qualified and uses UFCS for `compose`).
The result is both runnable **and** describable.

```rust
use orlando_transducers::pipeline;

let p = pipeline!(map(|x: i32| x * 2) >> filter(|x: &i32| *x > 5) >> take(3));
// p implements both Transducer and Describable.
```

A companion `pipeline_descriptor!` emits a compile-time, zero-allocation
`&'static [StageSpec]` for the same stages:

```rust
use orlando_transducers::{pipeline_descriptor, StageSpec};

const DESC: &[StageSpec] = pipeline_descriptor!(map >> filter >> take(3));
assert_eq!(DESC, &[StageSpec::Map, StageSpec::Filter, StageSpec::Take { n: 3 }]);
```

Because both macros dispatch through the same per-operation mapping, and a
`pipeline!`-built chain's `.describe()` is tested against the matching
`pipeline_descriptor!`, **forward composition and decomposition cannot drift**.

## Layer A: Inversion (`Invertible` / `IsoMap`)

Most pipelines are *lossy* — `Filter` drops elements, `Take` truncates, `FlatMap`
fans out — and destroy information, so they have no true inverse. The
**bijective subset** (the groupoid) *can* be reversed.

### The `IsoMap` type

`IsoMap` is a streaming isomorphism pairing a `to` and `from` function — the
transducer analogue of the `Iso` optic, lifted to streams.

```rust
use orlando_transducers::invert::{Invertible, IsoMap};
use orlando_transducers::collectors::to_vec;

// Celsius ⇄ Fahrenheit
let to_f = IsoMap::new(|c: f64| c * 9.0 / 5.0 + 32.0, |f: f64| (f - 32.0) * 5.0 / 9.0);

let celsius = vec![0.0, 100.0, 25.0];
let fahrenheit = to_vec(&to_f, celsius.clone());      // [32, 212, 77]
let recovered  = to_vec(&to_f.invert(), fahrenheit);  // [0, 100, 25]
```

**Caller responsibility:** `to` and `from` must be true inverses.

### Composition reverses by the groupoid law

`(a ∘ b)⁻¹ = b⁻¹ ∘ a⁻¹`. A composed pipeline inverts by reversing stage order
and inverting each part:

```rust
use orlando_transducers::invert::IsoMap;
use orlando_transducers::transducer::Transducer;
use orlando_transducers::collectors::to_vec;

let a = IsoMap::new(|x: i32| x * 2, |y: i32| y / 2);
let b = IsoMap::new(|x: i32| x + 10, |y: i32| y - 10);
let forward = a.compose(b);

let input = vec![1, 2, 3];
let output = to_vec(&forward, input.clone());        // [12, 14, 16]
let recovered = to_vec(&forward.invert(), output);   // [1, 2, 3]
```

### Excluded by construction

Lossy transducers (`Filter`, `Take`, `Drop`, `Unique`, `FlatMap`, plain `Map`,
…) **do not implement `Invertible`**, so calling `.invert()` on a pipeline that
contains them is a **compile error**. The groupoid excludes the lossy majority
by construction — you cannot accidentally invert something non-invertible.

### JavaScript

```javascript
const toF = new Pipeline()
  .isoMap(c => c * 9/5 + 32, f => (f - 32) * 5/9)
  .isoMap(x => x + 10, y => y - 10);

if (toF.canInvert()) {
  const toC = toF.invert();   // reverses order, swaps to/from
}
// A pipeline with a lossy stage throws on invert():
// new Pipeline().filter(x => x > 0).invert();  // Error
```

`canInvert()` returns `true` only when every stage is an `isoMap` (or the
pipeline is empty — the identity, which is self-inverse).

## Layer B: Provenance (`trace`)

For lossy pipelines, the post-hoc "inverse" answers **which inputs produced
these outputs?** The `trace` function records a tape mapping each output to its
source input index.

```rust
use orlando_transducers::provenance::trace;
use orlando_transducers::transforms::{Map, Filter};
use orlando_transducers::transducer::Transducer;

let p = Filter::new(|x: &i32| x % 2 == 0).compose(Map::new(|x: i32| x * 2));
let data = vec![1, 2, 3, 4, 5, 6];

let (outputs, t) = trace(&p, data.clone());
assert_eq!(outputs, vec![4, 8, 12]);
// outputs[0] came from input[1], etc.
assert_eq!(t.sources, vec![1, 3, 5]);

// The "inverse of Filter": a boolean mask over the original input.
assert_eq!(t.kept_mask(data.len()), vec![false, true, false, true, false, true]);
```

Per-op provenance semantics:

| Op | Provenance |
|----|------------|
| `Map` / `Filter` | output tagged with its producing input |
| `FlatMap` | every fanned-out output shares the originating input's index |
| `Chunk` / `Aperture` | the emitted group tagged with its completing input |
| `Take` / `Drop` | standard prefix / suffix indexing |

Unlike inversion, provenance is **post-hoc** — it needs the source stream
because that is where the lost information lives. `trace` works on any
transducer, lossy or invertible.
