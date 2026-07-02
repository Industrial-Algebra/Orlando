# Reversible Pipelines

Practical patterns combining reflection, inversion, and provenance. These
examples assume the API from
[Reflection & Inversion](../api/reflection-inversion.md).

## Reflecting a pipeline for logging

`describe()` turns a pipeline into a serializable stage list — useful for
logging, debugging, and storing pipeline definitions as config.

```rust
use orlando_transducers::pipeline;
use orlando_transducers::Describable;

let p = pipeline!(map(|x: i32| x + 1) >> filter(|x: &i32| *x % 2 == 0) >> take(3));

// Log what the pipeline does, without running it.
let summary: Vec<&'static str> = p.describe().iter().map(|s| s.name()).collect();
println!("pipeline = {}", summary.join(" >> "));
// pipeline = map >> filter >> take
```

## Reversible unit conversion

A pipeline built entirely from `isoMap` stages is invertible. Round-trip data
through the forward transform and its inverse to verify correctness.

```rust
use orlando_transducers::invert::IsoMap;
use orlando_transducers::transducer::Transducer;
use orlando_transducers::collectors::to_vec;

// Two-step temperature transform: C -> F, then offset by 10.
let forward = IsoMap::new(|c: f64| c * 9.0 / 5.0 + 32.0, |f: f64| (f - 32.0) * 5.0 / 9.0)
    .compose(IsoMap::new(|x: f64| x + 10.0, |y: f64| y - 10.0));

let celsius = vec![0.0, 37.0, 100.0];
let stored = to_vec(&forward, celsius.clone());

// ...later, recover the originals by inverting:
let recovered = to_vec(&forward.invert(), stored);
assert_eq!(recovered, celsius);
```

## Provenance: reconstructing what Filter dropped

For a lossy pipeline, `trace` records which inputs survived — the kept-mask is
the practical "inverse" when a true inverse is impossible.

```rust
use orlando_transducers::provenance::trace;
use orlando_transducers::transforms::Filter;
use orlando_transducers::transducer::Transducer;

let records = vec![
    (1, "alice", true),
    (2, "bob", false),
    (3, "carol", true),
    (4, "dave", false),
];

// Keep only active records.
let active = Filter::new(|r: &(i32, &str, bool)| r.2);
let (kept, t) = trace(&active, records.clone());

assert_eq!(kept.len(), 2);
// The mask tells us exactly which inputs survived.
assert_eq!(t.kept_mask(records.len()), vec![true, false, true, false]);
```

## Combining inversion and reflection

Because `invert()` produces another `Describable` pipeline, you can reflect
both directions from a single declaration.

```rust
use orlando_transducers::pipeline;
use orlando_transducers::Describable;

let p = pipeline!(
    iso_map(|x: i32| x * 2, |y: i32| y / 2) >>
    iso_map(|x: i32| x + 10, |y: i32| y - 10)
);

let forward = p.describe();
let reverse = p.invert().describe();
// Each direction is two isoMap stages.
assert_eq!(forward.len(), 2);
assert_eq!(reverse.len(), 2);
```
