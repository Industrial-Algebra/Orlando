# Inverse Transducers — Design Document

**Status:** Proposed (2026-06-25)
**Author:** Analysis & design session
**Related:** [API Expansion Roadmap](API_EXPANSION_ROADMAP.md), [Hybrid Composition](HYBRID_COMPOSITION.md)

---

## 1. Motivation

Orlando today is a **one-way composer**: you build a pipeline like
`Map ∘ Filter ∘ Take` and run it forward over data. There is no way to:

- **Inspect** a built pipeline (what stages does it have?)
- **Serialize** a pipeline to config / JSON
- **Invert** a pipeline to recover inputs from outputs
- **Trace** which inputs produced a given output

This document designs the *inverse* — a system that can **break down data
the way Orlando currently composes it**. It also captures the quality gaps
discovered during analysis and the immediate cleanup steps that precede the
inverse work.

A transducer is, formally:

```
∀Acc. ((Acc, Out) -> Acc) -> ((Acc, In) -> Acc)
```

Composition is categorical composition, right-to-left. The "inverse" admits
three distinct, complementary meanings, addressed in Layers A–C below.

---

## 2. Current State (verified 2026-06-25)

- **Version:** Cargo `0.5.0`, 717 tests (369 lib, all green), clippy clean.
- **Branch:** `develop`. Remote: `king-ghodorah` (tailnet).
- **Phases 1–8 + 6b+ complete.** Phase 9 (perf & quality) and Phase 10
  (docs & v1.0) remain.
- **Architecture:** `Step<T>` monad → `Transducer<In,Out>` trait →
  `Compose<T1,T2,In,Mid,Out>` (nested generic, **purely type-level**) →
  15 transforms, ~40 collectors, full optics hierarchy (Lens/Optional/Prism/
  Iso/Fold/Traversal), profunctor backing via Karpal, geometric optics on
  Clifford coefficient arrays, reactive Signal/Stream, Rust iterator ext +
  fluent builder, WASM `Pipeline` with MapFilter fusion.

### 2.1 Quality gaps found

| # | Gap | Severity |
|---|-----|----------|
| 1 | **`simd.rs` is dead code.** README/lib.rs advertise SIMD optimization, but `map_f64_simd`/`sum_f64_simd`/`mul_f64_simd` are referenced *only* in `tests/wasm_tests.rs`. Nothing in `Pipeline` or any collector calls them. Worse, `map_f64_simd` is **fake SIMD**: it loads a `v128`, extracts both lanes, and calls `f` on each scalar — strictly slower than scalar due to load/extract overhead. Only `sum_f64_simd` and `mul_f64_simd` genuinely vectorize. | High (credibility) |
| 2 | **Version drift + stray artifacts.** `Cargo.toml` = 0.5.0, `package.json` = **0.4.0**. Three `pkg*` dirs exist: `pkg` (0.5.0), `pkg-temp` (0.1.0), `pkg-test` (0.1.1). | Medium |
| 3 | **No runtime introspection of composed Rust pipelines.** `Compose<T1,T2,…>` is fully type-level; you cannot ask a built pipeline for its stages. Only the WASM `Operation` enum carries structure. This blocks serialization, visualization, replay, and the inverse design. | High (architectural) |
| 4 | **Karpal coupling.** Optics re-export Karpal types directly (`Getter`/`Review`/`Setter`). Locks Orlando to a specific profunctor library version. | Low |

---

## 3. Immediate Cleanup (Step 1)

Precedes any new feature work. Removes the credibility gap and tidies the
build.

1. **Remove `simd.rs`** and its standalone WASM test. Remove/qualify SIMD
   claims in `README.md` and `src/lib.rs` doc comment. Record SIMD as a
   **future proper numeric fast-path** task (a genuine `Float64Array` op in
   `pipeline.rs`, not the current fake-lane `map`).
2. **Delete `pkg-temp/` and `pkg-test/`** stray build outputs.
3. **Sync `package.json` version** to `0.5.0`.
4. **`.gitignore`** build artifacts (`pkg*/`, `target/`) so they stop
   leaking into the tree.

Rationale for removal over wiring: `map_f64_simd` provides no benefit (fake
lane extraction), and wiring the genuinely-vectorized `sum`/`mul` into the
JS `Pipeline` is a real feature with its own scope — it belongs in a
dedicated numeric-fast-path task, not bolted onto a cleanup step.

---

## 4. Inverse Design — Three Layers

### Layer A — Reversible (bijective) transducers

Restrict to the **invertible subset**: `Map` with an invertible function,
plus ops backed by an `Iso` (`to`/`from`). These form a **groupoid** under
composition, so the inverse is algebraically clean:

```
(t₁ ∘ t₂)⁻¹  =  t₂⁻¹ ∘ t₁⁻¹
```

This pairs naturally with the existing `Iso` optic. A `Map<IsoFn>` is a
*streaming isomorphism* — the transducer-level analogue of `Iso`. The
inverse pipeline is the reversed composition with each `to` swapped for
`from`.

**Excluded by construction** (lossy, not groupoid-eligible): `Filter`,
`Take`, `Drop`, `Unique`, `FlatMap` (one-to-many), `Chunk` (many-to-one,
though bijective when no partial chunk). These belong to Layer B.

### Layer B — Provenance / inverse-trace (for lossy ops)

For non-invertible transducers, "inverse" means **"which inputs produced
these outputs?"** Keep a trace tape alongside the forward pass:

| Forward op | Its "inverse" (provenance) |
|---|---|
| `Filter(p)` | boolean mask over the input stream |
| `Take(n)` | "the prefix of length n" |
| `Drop(n)` | "the suffix after index n" |
| `Chunk(k)` | flatten (groupoid-inverse; bijective iff no partial chunk) |
| non-invertible `Map(f)` | needs original input; only recoverable if recorded |

This is a *post-hoc decompose*, not a true inverse — it needs the source
stream or a recorded trace. A `Filter` viewed backward is a selection mask.

### Layer C — Reflection / decomposition (the real "break down as composed")

This is the heart of the request. **Today you cannot inspect a composed Rust
pipeline.** `Compose<T1,T2,…>` is a nested generic with no runtime data.
The inverse-as-decompose requires:

- A `StageSpec` value type — a serializable descriptor:
  `{ op: "map"|"filter"|"take"|…, params }`.
- A `describe()` trait method yielding `Vec<StageSpec>` for any composed
  transducer.
- **One macro that emits both the forward chain and the descriptor** from a
  single declaration, so compose and decompose can never drift.

---

## 5. Rust Macro Design

A declarative `pipeline!` macro generating the forward `Compose` chain
**and** a static descriptor array in one expansion:

```rust
orlando::pipeline! {
    map(|x: i32| x * 2)
    >> filter(|x: &i32| *x > 5)
    >> take(3)
}
// expands to:
//   pub static DESCRIPTOR: &[StageSpec] = &[
//       StageSpec::Map, StageSpec::Filter, StageSpec::Take(3),
//   ];
//   pub fn build() -> impl Transducer<i32, i32> {
//       Map::new(..).compose(Filter::new(..)).compose(Take::new(3))
//   }
```

For the invertible subset, a proc-macro `iso_pipeline!` (or
`#[transducer(iso)]`) that demands a `to`/`from` pair and auto-generates
the inverse:

```rust
orlando::iso_pipeline! {
    celsius_to_fahrenheit:
        to   = |c: f64| c * 9.0/5.0 + 32.0,
        from = |f: f64| (f - 32.0) * 5.0/9.0;
}
// generates a Transducer + its .invert() -> reversed Iso Transducer
```

A `#[derive(Transducer)]` path is viable for custom invertible ops, but
**must reject lossy bodies at compile time** — restricting to the groupoid
is the entire point.

The descriptor layer is what unlocks everything downstream: serialization,
equality, visualization, replay, and the WASM surface.

---

## 6. TypeScript / WASM Surface

On the `Pipeline` class, add:

- **`describe(): StageSpec[]`** — JSON-serializable stage list (from the
  macro-generated descriptor). Enables pipeline-as-config, round-tripping
  through JSON, and visual editing.
- **`invert(): Pipeline | null`** — succeeds only for the bijective subset
  (Layer A); returns `null` if any stage is lossy.
- **`InversePipeline`** — wraps a provenance trace (Layer B): given a
  forward run + source, answers "which inputs fed output *i*?" as a typed
  iterator/mask.

Generated `.d.ts` becomes a discriminated union:

```ts
type StageSpec =
  | { op: "map" }
  | { op: "filter" }
  | { op: "take"; n: number }
  | { op: "chunk"; size: number }
  | { op: "iso"; reversible: true }
  | { ... };

interface Pipeline {
  describe(): StageSpec[];
  invert(): Pipeline | null;
  // existing: map, filter, take, toArray, ...
}
```

---

## 7. Roadmap & Sequencing

| Step | Scope | Layer |
|------|-------|-------|
| **1** *(this branch, cleanup)* | Remove `simd.rs` + fake-SIMD claims; delete `pkg-temp`/`pkg-test`; sync `package.json` to 0.5.0; `.gitignore` artifacts. | — |
| **2** | `StageSpec` value type + `describe()` trait on Rust core. | C |
| **3** | `pipeline!` macro emitting descriptor + chain together. | C |
| **4** | Reversible subset (Layer A) via Iso pairing + `.invert()`. | A |
| **5** | WASM/TS surface: `describe` / `invert` / `InversePipeline`. | A, B, C |
| **6** | Provenance traces for lossy ops (Layer B). | B |

Steps 2–3 are the foundation: they turn Orlando from a one-way composer into
an **inspectable, serializable** pipeline system. Steps 4–5 add
**reversibility**. Step 6 adds **provenance** for the lossy majority.

---

## 8. Why This Matters

Orlando's optics layer already has bidirectionality (`Iso`). Extending
bidirectionality to the streaming/transducer layer — and adding reflection
so pipelines are inspectable — turns Orlando from a fast executor into a
**first-class, serializable, reversible transformation system**. The macro
guarantees forward and inverse never drift, and `StageSpec` becomes the
single source of truth shared by Rust, WASM, and TypeScript.
