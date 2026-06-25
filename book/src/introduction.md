# Orlando

> Transform transformations, not data. Compositional data processing via WebAssembly.

Orlando brings the power of **transducers** to JavaScript and TypeScript through a blazing-fast Rust/WebAssembly implementation. Named after the bridger characters in Greg Egan's *Diaspora*, who embodied transformation at fundamental levels.

## What Are Transducers?

**Transducers compose transformations, not data.**

Traditional JavaScript array methods create intermediate arrays at each step:

```javascript
// Traditional approach - creates 2 intermediate arrays
const result = data
  .map(x => x * 2)        // intermediate array 1
  .filter(x => x > 10)    // intermediate array 2
  .slice(0, 5);           // final result

// For 1M items, this allocates ~24MB of intermediate memory
```

Orlando transducers execute transformations in a **single pass** with **zero intermediate allocations**:

```javascript
import init, { Pipeline } from 'orlando-transducers';
await init();

const pipeline = new Pipeline()
  .map(x => x * 2)
  .filter(x => x > 10)
  .take(5);

const result = pipeline.toArray(data);

// For 1M items, stops after finding 5 matches
// Memory: ~40 bytes (just the 5-element result)
```

## Key Features

- **No intermediate allocations** - Single pass over data
- **Early termination** - Stops processing as soon as possible
- **Composable** - Build complex pipelines from simple operations
- **WASM-powered** - Native performance via WebAssembly
- **Automatic fusion** - Map-Filter chains automatically optimized
- **Functional optics** - Lens, Prism, Iso, Fold, Traversal for immutable data
- **Profunctor encoding** - Principled optics composition via [Karpal](https://crates.io/crates/karpal-optics)
- **Reactive primitives** - Signal and Stream types (Rust API)
- **Geometric optics** - Multivector grade projection and extraction
- **Tiny** - <50KB compressed WASM bundle

## Performance

Real-world benchmarks show **3-19x speedup** over native JavaScript array methods:

| Scenario | JavaScript Arrays | Orlando | Speedup |
|----------|------------------|---------|---------|
| Map - Filter - Take 10 (100K items) | 2.3ms | 0.6ms | **3.8x** |
| Complex pipeline (10 ops, 50K items) | 8.7ms | 2.1ms | **4.1x** |
| Early termination (find first 5 in 1M items) | 15.2ms | 0.8ms | **19x** |

Orlando's architecture is designed around three principles:

1. **Zero intermediate arrays** - Array methods create a new array at each step
2. **Early termination** - Orlando stops processing immediately when conditions are met
3. **WASM execution** - Pre-compiled, consistent native performance

## Category Theory Foundation

Transducers are **natural transformations between fold functors**. A transducer transforms a reducing function:

```
forall Acc. ((Acc, Out) -> Acc) -> ((Acc, In) -> Acc)
```

This foundation guarantees:
- **Identity law**: `id . f = f . id = f`
- **Associativity**: `(f . g) . h = f . (g . h)`

Orlando's optics hierarchy is built on profunctor encoding via Karpal, providing mathematically principled composition across optic types (Lens, Prism, Iso, Fold, Traversal).

## When Should You Use Orlando?

### Great for:

- **Large datasets** (>1000 elements) - More data = bigger performance wins
- **Complex pipelines** (3+ operations) - Single-pass execution shines
- **Early termination** scenarios - `take`, `takeWhile`, find first N
- **Memory-constrained environments** - No intermediate allocations
- **Reusable transformation logic** - Define pipelines once, use many times

### Consider array methods for:

- **Small datasets** (<100 elements) - Overhead may not be worth it
- **Single operations** - `array.map(fn)` is simpler than a pipeline
- **Prototyping** - Array methods are more familiar during development
