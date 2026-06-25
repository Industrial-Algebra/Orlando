# Orlando: High-Performance Transducers for JavaScript

> Transform transformations, not data. Compositional data processing via WebAssembly.

Orlando brings the power of **transducers** to JavaScript and TypeScript through a blazing-fast Rust/WebAssembly implementation. Named after the bridger characters in Greg Egan's *Diaspora*, who embodied transformation at fundamental levels.

[![npm version](https://img.shields.io/npm/v/orlando-transducers.svg)](https://www.npmjs.com/package/orlando-transducers)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## What Are Transducers?

**Transducers compose transformations, not data.**

Traditional JavaScript array methods create intermediate arrays at each step:

```javascript
// ❌ Traditional approach - creates 2 intermediate arrays
const result = data
  .map(x => x * 2)        // intermediate array 1
  .filter(x => x > 10)    // intermediate array 2
  .slice(0, 5);           // final result

// For 1M items, this allocates ~24MB of intermediate memory
```

Orlando transducers execute transformations in a **single pass** with **zero intermediate allocations**:

```javascript
// ✅ Orlando - single pass, no intermediates
import init, { Pipeline } from 'orlando-transducers';
await init();

const pipeline = new Pipeline()
  .map(x => x * 2)
  .filter(x => x > 10)
  .take(5);

const result = pipeline.toArray(data);

// For 1M items, stops after finding 5 matches!
// Memory: ~40 bytes (just the 5-element result)
```

### Performance Benefits

- **🚀 No intermediate allocations** - Single pass over data
- **⚡ Early termination** - Stops processing as soon as possible
- **🔧 Composable** - Build complex pipelines from simple operations
- **💪 WASM-powered** - Native performance via WebAssembly
- **⚡ Automatic fusion** - Map→Filter chains automatically optimized
- **📦 Tiny** - <50KB compressed WASM bundle

## Performance

Real-world benchmarks show **3-19x speedup** over native JavaScript array methods:

| Scenario | JavaScript Arrays | Orlando Transducers | Speedup |
|----------|------------------|---------------------|---------|
| Map → Filter → Take 10 (100K items) | 2.3ms | 0.6ms | **3.8x faster** |
| Complex pipeline (10 operations, 50K items) | 8.7ms | 2.1ms | **4.1x faster** |
| Early termination (find first 5 in 1M items) | 15.2ms | 0.8ms | **19x faster** 🔥 |

**Why is Orlando faster?**
1. **Zero intermediate arrays** - Array methods create a new array at each step
2. **Early termination** - Orlando stops processing immediately when conditions are met
3. **WASM execution** - Native performance via WebAssembly

[Run benchmarks in your browser →](examples/performance.html)

## Installation

```bash
npm install orlando-transducers
# or
yarn add orlando-transducers
# or
pnpm add orlando-transducers
```

**Using from CDN:**
```html
<script type="module">
  import init, { Pipeline } from 'https://unpkg.com/orlando-transducers';
  await init();
  // Use Pipeline...
</script>
```

## Quick Start

```javascript
import init, { Pipeline } from 'orlando-transducers';

// Initialize WASM (once per application)
await init();

// Create a reusable pipeline
const pipeline = new Pipeline()
  .map(x => x * 2)
  .filter(x => x % 3 === 0)
  .take(5);

// Execute on data
const data = Array.from({ length: 100 }, (_, i) => i + 1);
const result = pipeline.toArray(data);

console.log(result); // [6, 12, 18, 24, 30]
```

**TypeScript with full type safety:**
```typescript
import init, { Pipeline } from 'orlando-transducers';

await init();

interface User {
  id: number;
  name: string;
  active: boolean;
}

const activeUserEmails = new Pipeline()
  .filter((user: User) => user.active)
  .map((user: User) => user.email)
  .take(100);

const emails = activeUserEmails.toArray(users);
```

## Functional Lenses (Optics)

**NEW in v0.4.0, enhanced in v0.5.0!** Orlando includes high-performance functional lenses and a full optics hierarchy for immutable data access and transformation.

### What are Lenses?

**Lenses provide composable, type-safe access to nested data structures.**

Traditional JavaScript property updates require verbose spreading:

```javascript
// ❌ Traditional nested update - verbose and error-prone
const updated = {
  ...user,
  address: {
    ...user.address,
    city: "Boston"
  }
};
```

Orlando lenses make this clean and composable:

```javascript
// ✅ Orlando lens - clean and composable
import { lens, lensPath } from 'orlando-transducers';

const cityLens = lensPath(['address', 'city']);
const updated = cityLens.set(user, "Boston");
```

### Lens Operations

| Method | Description | Example |
|--------|-------------|---------|
| `get(obj)` | Extract the focused value | `nameLens.get(user)` → `"Alice"` |
| `set(obj, value)` | Immutably update the value | `nameLens.set(user, "Bob")` → new object |
| `over(obj, fn)` | Transform the value with a function | `nameLens.over(user, s => s.toUpperCase())` |
| `compose(other)` | Combine lenses for deeper access | `addressLens.compose(cityLens)` |

### Quick Examples

```javascript
import init, { lens, lensPath, optional } from 'orlando-transducers';
await init();

const user = {
  name: "Alice",
  email: "alice@example.com",
  address: {
    city: "NYC",
    zip: "10001"
  }
};

// Simple property lens
const nameLens = lens('name');
nameLens.get(user);              // "Alice"
nameLens.set(user, "Bob");       // { ...user, name: "Bob" }
nameLens.over(user, s => s.toUpperCase());  // { ...user, name: "ALICE" }

// Deep nested access with lensPath
const cityLens = lensPath(['address', 'city']);
cityLens.get(user);              // "NYC"
cityLens.set(user, "Boston");    // { ...user, address: { ...address, city: "Boston" }}

// Lens composition
const addressLens = lens('address');
const zipLens = lens('zip');
const userZipLens = addressLens.compose(zipLens);
userZipLens.get(user);           // "10001"

// Optional lenses for nullable fields
const phoneLens = optional('phone');
phoneLens.get(user);             // undefined (property doesn't exist)
phoneLens.getOr(user, "N/A");    // "N/A" (default value)
phoneLens.over(user, normalize); // No-op if undefined, transforms if exists
```

### Streaming Lenses: The Killer Feature 🔥

**Orlando uniquely combines lenses with transducers** for powerful streaming data transformations:

```javascript
import { lens, Pipeline } from 'orlando-transducers';

const users = [
  { name: "Alice", profile: { email: "alice@company.com", verified: true }},
  { name: "Bob", profile: { email: "bob@gmail.com", verified: false }},
  { name: "Carol", profile: { email: "carol@company.com", verified: true }}
];

// Extract → Filter → Transform pipeline
const emailLens = lensPath(['profile', 'email']);
const verifiedLens = lensPath(['profile', 'verified']);

const companyEmails = new Pipeline()
  .map(user => emailLens.get(user))           // Extract emails with lens
  .filter(email => email.endsWith('@company.com'))  // Stream filter
  .map(email => email.toLowerCase())          // Transform
  .toArray(users.filter(u => verifiedLens.get(u)));

// Result: ["alice@company.com", "carol@company.com"]
```

**No other lens library can do this!** Orlando is the first to integrate lenses with streaming transducers for processing large datasets with bounded memory.

### Lens Laws

All Orlando lenses are **mathematically proven correct** via property-based tests that verify the three lens laws:

1. **GetPut**: `set(s, get(s)) = s` - Setting what you got changes nothing
2. **PutGet**: `get(set(s, a)) = a` - Getting what you set returns that value
3. **PutPut**: `set(set(s, a1), a2) = set(s, a2)` - Setting twice equals setting once

This mathematical foundation ensures lenses compose correctly and behave predictably.

## API Reference

All methods return a new `Pipeline` instance, allowing for fluent method chaining.

### Transformations

| Method | Description | Example |
|--------|-------------|---------|
| `map(fn)` | Transform each element | `.map(x => x * 2)` |
| `filter(predicate)` | Keep only matching elements | `.filter(x => x > 5)` |
| `take(n)` | Take first n elements (early termination!) | `.take(10)` |
| `takeWhile(predicate)` | Take while predicate is true | `.takeWhile(x => x < 100)` |
| `drop(n)` | Skip first n elements | `.drop(5)` |
| `dropWhile(predicate)` | Skip while predicate is true | `.dropWhile(x => x < 10)` |
| `tap(fn)` | Execute side effects without modifying values | `.tap(x => console.log(x))` |
| `flatMap(fn)` | Transform and flatten nested arrays | `.flatMap(x => [x, x * 2])` |
| `reject(predicate)` | Remove matching elements (inverse of filter) | `.reject(x => x < 0)` |
| `chunk(n)` | Group elements into chunks of size n | `.chunk(3)` |
| `unique()` | Remove consecutive duplicates | `.unique()` |
| `scan(fn, initial)` | Accumulate with intermediate results | `.scan((a, b) => a + b, 0)` |

### Terminal Operations (Collectors)

These execute the pipeline and return a result:

| Method | Description | Example |
|--------|-------------|---------|
| `toArray(source)` | Collect results into an array | `pipeline.toArray(data)` |
| `reduce(source, reducer, initial)` | Custom reduction | `pipeline.reduce(data, (a,b) => a+b, 0)` |
| `find(source, predicate)` | Find first matching element | `find(pipeline, data, x => x > 10)` |
| `partition(source, predicate)` | Split into [matching, non-matching] | `partition(pipeline, data, isValid)` |
| `groupBy(source, keyFn)` | Group elements by key function | `groupBy(pipeline, data, x => x.type)` |
| `frequencies(source)` | Count occurrences of each element | `frequencies(data)` |
| `topK(source, k)` | Get k largest elements | `topK(scores, 10)` |

### Statistical Operations

| Function | Description | Example |
|----------|-------------|---------|
| `product(array)` | Multiply all numbers | `product([2, 3, 4])` → 24 |
| `mean(array)` | Arithmetic mean (average) | `mean([1, 2, 3, 4, 5])` → 3 |
| `median(array)` | Middle value | `median([1, 2, 3, 4, 5])` → 3 |
| `min(array)` / `max(array)` | Minimum/maximum value | `max([1, 5, 3])` → 5 |
| `minBy(array, keyFn)` / `maxBy(array, keyFn)` | Min/max by key function | `maxBy(users, u => u.score)` |
| `variance(array)` | Sample variance | `variance([2, 4, 6, 8])` |
| `stdDev(array)` | Standard deviation | `stdDev([2, 4, 6, 8])` |
| `quantile(array, p)` | P-th quantile (0-1) | `quantile(data, 0.95)` |
| `mode(array)` | Most frequent value | `mode([1, 2, 2, 3])` → 2 |

### Collection Utilities

| Function | Description | Example |
|----------|-------------|---------|
| `sortBy(array, keyFn)` | Sort by key function | `sortBy(users, u => u.age)` |
| `sortWith(array, cmpFn)` | Sort with comparator | `sortWith(nums, (a,b) => a - b)` |
| `reverse(array)` | Reverse order | `reverse([1, 2, 3])` → [3, 2, 1] |
| `range(start, end, step)` | Generate numeric sequence | `range(0, 10, 2)` → [0, 2, 4, 6, 8] |
| `repeat(value, n)` | Repeat value N times | `repeat('x', 3)` → ['x', 'x', 'x'] |
| `cycle(array, n)` | Repeat array N times | `cycle([1, 2], 3)` → [1, 2, 1, 2, 1, 2] |
| `unfold(seed, fn, limit)` | Generate from seed | `unfold(1, x => x * 2, 5)` → [2, 4, 8, 16, 32] |
| `path(obj, pathArray)` | Safe deep property access | `path(user, ['profile', 'email'])` |
| `pathOr(obj, path, default)` | Path with default value | `pathOr(config, ['port'], 8080)` |
| `evolve(obj, transforms)` | Nested transformations | `evolve(user, { age: n => n + 1 })` |

### Logic Functions

Predicate combinators and conditional transformations for cleaner conditional logic:

| Function | Description | Example |
|----------|-------------|---------|
| `both(p1, p2)` | Combine predicates with AND | `both(isPositive, isEven)` |
| `either(p1, p2)` | Combine predicates with OR | `either(isSmall, isLarge)` |
| `complement(pred)` | Negate a predicate | `complement(isEven)` |
| `allPass(predicates)` | All predicates must pass | `allPass([isValid, isActive])` |
| `anyPass(predicates)` | Any predicate must pass | `anyPass([isZero, isDivisibleBy10])` |
| `When(pred, fn)` | Transform only when predicate is true | `new When(x => x > 0, x => x * 2)` |
| `Unless(pred, fn)` | Transform only when predicate is false | `new Unless(x => x > 0, _ => 0)` |
| `IfElse(pred, onTrue, onFalse)` | Branch on condition | `new IfElse(x => x >= 0, double, halve)` |

### Multi-Input Operations

Operations for combining and comparing multiple arrays:

| Function | Description | Example |
|----------|-------------|---------|
| `merge(arrays)` | Interleave multiple arrays | `merge([a, b, c])` |
| `zip(a, b)` | Combine into pairs | `zip([1,2], ['a','b'])` |
| `zipLongest(a, b, fill)` | Zip with fill for different lengths | `zipLongest(a, b, null)` |
| `intersection(a, b)` | Elements in both arrays | `intersection(a, b)` |
| `union(a, b)` | Unique elements from both | `union(a, b)` |
| `difference(a, b)` | Elements in a but not b | `difference(a, b)` |
| `cartesianProduct(a, b)` | All possible pairs | `cartesianProduct(colors, sizes)` |

### Optics

Functional optics for immutable, composable data access and transformation:

| Function/Method | Description | Example |
|-----------------|-------------|---------|
| `lens(property)` | Focus on an object property | `lens('name')` |
| `lensPath(path)` | Focus on a nested path | `lensPath(['address', 'city'])` |
| `optional(property)` | Focus on a nullable field | `optional('phone')` |
| `prism(matchFn, buildFn)` | Focus on a sum type / variant | `prism(x => x.tag === 'Some' ? x.value : undefined, v => ({tag: 'Some', value: v}))` |
| `iso(toFn, fromFn)` | Lossless bidirectional conversion | `iso(c => c * 9/5 + 32, f => (f - 32) * 5/9)` |
| `fold(extractFn)` | Read-only traversal | `fold(obj => Object.values(obj))` |
| `traversal(getAllFn, overAllFn)` | Collection-level lens | `traversal(arr => arr, (arr, fn) => arr.map(fn))` |
| `.get(obj)` | Extract the focused value | `nameLens.get(user)` |
| `.set(obj, value)` | Immutably update the value | `nameLens.set(user, "Bob")` |
| `.over(obj, fn)` | Transform with a function | `nameLens.over(user, toUpper)` |
| `.compose(other)` | Compose optics for deep access | `addrLens.compose(cityLens)` |
| `.getOr(obj, default)` | Get with default (optional only) | `phoneLens.getOr(user, "N/A")` |

### Pipeline Enhancements

JavaScript-specific convenience methods:

| Method | Description | Example |
|--------|-------------|---------|
| `.pluck(key)` | Extract a single property | `.pluck('name')` |
| `.project(keys)` | Extract multiple properties | `.project(['id', 'name'])` |
| `.compact()` | Remove falsy values | `.compact()` |
| `.flatten(depth)` | Flatten nested arrays | `.flatten(2)` |
| `.whereMatches(spec)` | Pattern-match filter | `.whereMatches({ active: true })` |
| `.viewLens(lens)` | Extract via lens | `.viewLens(nameLens)` |
| `.overLens(lens, fn)` | Transform via lens | `.overLens(priceLens, p => p * 0.9)` |
| `.filterLens(lens, pred)` | Filter by lens value | `.filterLens(ageLens, a => a >= 18)` |
| `.setLens(lens, value)` | Set via lens | `.setLens(statusLens, "published")` |

### Profunctor Optics (v0.5.0)

Orlando's optics are backed by a profunctor encoding via [Karpal](https://crates.io/crates/karpal-optics), enabling principled composition and cross-type conversions:

| Method | Description | Example |
|--------|-------------|---------|
| `.transform()` | Access the profunctor encoding | `lens.transform()` (Strong), `prism.transform()` (Choice) |
| `.then(other)` | Compose optics | `lens_a.then(lens_b)` |
| `.to_traversal()` | Convert to traversal | `lens.to_traversal()` |
| `.to_fold()` | Convert to fold | `prism.to_fold()` |
| `fold.fold_map(obj, f)` | Map and combine with monoid | `fold.fold_map(&data, \|x\| Sum(x))` |
| `fold.any(obj, pred)` | Check if any focused value matches | `fold.any(&data, \|x\| x > 5)` |
| `fold.all(obj, pred)` | Check if all focused values match | `fold.all(&data, \|x\| x > 0)` |
| `fold.find(obj, pred)` | Find first matching value | `fold.find(&data, \|x\| x > 5)` |

### Geometric Optics

Operations on multivector coefficient arrays (`Float64Array`):

| Function | Description | Example |
|----------|-------------|---------|
| `bladeGrade(index)` | Grade of a basis blade | `bladeGrade(3)` → 2 |
| `gradeExtract(p,q,r,grade,mv)` | Extract grade coefficients | `gradeExtract(3,0,0,2,mv)` |
| `gradeProject(p,q,r,grades,mv)` | Project onto grades | `gradeProject(3,0,0,[0,2],mv)` |
| `mvNorm(mv)` | Multivector magnitude | `mvNorm(mv)` |
| `mvNormalize(mv)` | Normalize to unit length | `mvNormalize(mv)` |
| `mvReverse(mv)` | Grade-dependent sign reversal | `mvReverse(mv)` |

**Full API documentation:** [docs/api/JAVASCRIPT.md](docs/api/JAVASCRIPT.md)

## Real-World Examples

### Immutable State Updates with Lenses

```javascript
import { lens, lensPath } from 'orlando-transducers';

// React/Redux-style immutable state updates
const state = {
  user: {
    profile: {
      name: "Alice",
      email: "alice@example.com"
    },
    preferences: {
      theme: "dark",
      notifications: true
    }
  }
};

// Update nested properties immutably
const nameLens = lensPath(['user', 'profile', 'name']);
const themeLens = lensPath(['user', 'preferences', 'theme']);

const newState = themeLens.set(
  nameLens.set(state, "Alicia"),
  "light"
);

// Original state unchanged, new state has both updates
console.log(state.user.profile.name);  // "Alice"
console.log(newState.user.profile.name);  // "Alicia"
console.log(newState.user.preferences.theme);  // "light"
```

### Bulk Updates with Lenses + Transducers

```javascript
import { lens, Pipeline } from 'orlando-transducers';

const products = [
  { id: 1, name: "Widget", price: 10, category: "tools" },
  { id: 2, name: "Gadget", price: 20, category: "tools" },
  { id: 3, name: "Doohickey", price: 15, category: "accessories" }
];

// Apply 20% discount to all tools
const priceLens = lens('price');
const categoryLens = lens('category');

const discounted = products.map(product =>
  categoryLens.get(product) === 'tools'
    ? priceLens.over(product, price => price * 0.8)
    : product
);
```

### Pagination

```javascript
function paginate(data, page, pageSize) {
  return new Pipeline()
    .drop((page - 1) * pageSize)
    .take(pageSize)
    .toArray(data);
}

const page2 = paginate(users, 2, 20); // Get page 2 (items 21-40)
```

[Try the interactive pagination demo →](examples/pagination.html)

### Data Processing Pipeline

```javascript
// Filter active users, normalize emails, find company addresses
const companyEmails = new Pipeline()
  .filter(user => user.active)
  .map(user => ({
    id: user.id,
    email: user.email.toLowerCase()
  }))
  .filter(user => user.email.endsWith('@company.com'))
  .take(100);

const result = companyEmails.toArray(users);
```

### Product Search with Multiple Filters

```javascript
const searchProducts = (products, { category, minPrice, maxPrice, minRating }) => {
  return new Pipeline()
    .filter(p => p.category === category)
    .filter(p => p.price >= minPrice && p.price <= maxPrice)
    .filter(p => p.rating >= minRating)
    .filter(p => p.inStock)
    .take(20)
    .toArray(products);
};

const results = searchProducts(catalog, {
  category: 'electronics',
  minPrice: 50,
  maxPrice: 500,
  minRating: 4.0
});
```

### Early Termination for Performance

```javascript
// Find first 10 prime numbers in a large dataset
const isPrime = n => {
  if (n < 2) return false;
  for (let i = 2; i <= Math.sqrt(n); i++) {
    if (n % i === 0) return false;
  }
  return true;
};

const pipeline = new Pipeline()
  .filter(isPrime)
  .take(10);

// Stops immediately after finding 10 primes!
// Traditional .filter().slice(0,10) would check ALL numbers
const firstTenPrimes = pipeline.toArray(hugeRange);
```

### Debugging with Tap

```javascript
const pipeline = new Pipeline()
  .tap(x => console.log('Input:', x))
  .map(x => x * 2)
  .tap(x => console.log('After doubling:', x))
  .filter(x => x > 10)
  .tap(x => console.log('After filter:', x));

const result = pipeline.toArray(data);
```

**More examples:**
- [Interactive Demo](examples/index.html) - Build and test pipelines in your browser
- [Real-World Data Processing](examples/data-processing.html) - ETL, log analysis, analytics
- [Performance Benchmarks](examples/performance.html) - Compare against native arrays
- [Library Comparison](examples/benchmark-comparison.html) - vs Underscore, Ramda, Lodash, Lazy.js
- [Migration Guide](docs/api/MIGRATION.md) - Convert from array methods to Orlando

## Benchmarks

Orlando has been benchmarked against popular JavaScript libraries to demonstrate real-world performance advantages.

### Libraries Compared

- **Native Array methods** - Built-in JavaScript
- **Underscore.js** - Classic utility library
- **Ramda** - Functional programming library
- **Lodash** - Modern utility library
- **Lazy.js** - Lazy evaluation library

### Key Results

Based on benchmarks across multiple scenarios:

| Scenario | Orlando vs Native | Winner |
|----------|------------------|--------|
| Map → Filter → Take (100K) | **4.8x faster** | Orlando 🏆 |
| Complex Pipeline (10 ops) | **3.2x faster** | Orlando 🏆 |
| Early Termination (1M) | **18.7x faster** 🔥 | Orlando 🏆 |
| Object Processing (500K) | **2.8x faster** | Orlando 🏆 |
| Simple Map (1M) | 1.3x slower | Native Array |

**Early termination provides the biggest wins** - Orlando stops processing as soon as conditions are met, while native arrays must complete all operations first.

### Running Benchmarks

**Node.js:**
```bash
npm install
npm run build:nodejs
npm run bench:all        # Full benchmark suite
npm run bench:quick      # Quick benchmarks
```

**Browser:**
- Open [examples/benchmark-comparison.html](examples/benchmark-comparison.html)
- Click "Run Benchmark" to compare against all libraries
- See visual comparison with speedup calculations

**Detailed results:** See [benchmarks/BENCHMARK_RESULTS.md](benchmarks/BENCHMARK_RESULTS.md) for complete data.

## When Should You Use Orlando?

### ✅ Great for:

- **Large datasets** (>1000 elements) - More data = bigger performance wins
- **Complex pipelines** (3+ operations) - Single-pass execution shines
- **Early termination** scenarios - `take`, `takeWhile`, find first N
- **Memory-constrained environments** - No intermediate allocations
- **Performance-critical code** - WASM-powered native speed
- **Reusable transformation logic** - Define pipelines once, use many times

### ⚠️ Consider array methods for:

- **Small datasets** (<100 elements) - Overhead may not be worth it
- **Single operations** - `array.map(fn)` is simpler than a pipeline
- **Prototyping** - Array methods are more familiar during development
- **Operations requiring all data** - e.g., `sort`, `reverse` (Orlando doesn't optimize these)

## Rust API

Orlando is a first-class Rust crate with ergonomic iterator extensions, reactive primitives, and a fluent builder API.

### TransduceExt & PipelineBuilder (v0.5.0)

```rust
use orlando_transducers::iter_ext::{TransduceExt, PipelineBuilder};
use orlando_transducers::{Map, Filter, Take};

// Extension trait — call .transduce() on any iterator
let result: Vec<i32> = (1..100)
    .transduce(Map::new(|x: i32| x * 2)
        .compose(Filter::new(|x: &i32| *x > 10))
        .compose(Take::new(5)));

// Fluent builder API — no manual composition needed
let result = PipelineBuilder::new()
    .map(|x: i32| x * 2)
    .filter(|x: &i32| *x > 10)
    .take(5)
    .run(1..100);
```

### Reactive Signals & Streams (v0.5.0)

```rust
use orlando_transducers::signal::Signal;
use orlando_transducers::stream::Stream;

// Signal — a time-varying value with derived computations
let celsius = Signal::new(0.0);
let fahrenheit = celsius.map(|c| c * 9.0 / 5.0 + 32.0);
celsius.set(100.0);
assert_eq!(*fahrenheit.get(), 212.0);

// Stream — push-based event processing
let clicks = Stream::new();
let doubles = clicks.map(|x: i32| x * 2);
doubles.subscribe(|val| println!("Got: {}", val));
clicks.emit(21); // prints "Got: 42"

// Bridge streams to signals with fold
let counter = Signal::new(0);
let increments = Stream::new();
counter.fold(&increments, 0, |acc, _| acc + 1);
```

## Documentation

- **[JavaScript/TypeScript API](docs/api/JAVASCRIPT.md)** - Complete API reference
- **[Hybrid Composition Guide](docs/HYBRID_COMPOSITION.md)** - Combining transducers with multi-input operations
- **[Migration Guide](docs/api/MIGRATION.md)** - Convert from array methods to Orlando
- **[WASM Boundary Performance](docs/WASM_BOUNDARY_PERFORMANCE.md)** - Deep dive: Why every instruction counts
- **[Optimization Guide](docs/OPTIMIZATIONS.md)** - Performance optimizations and best practices
- **[Fusion Optimization](docs/FUSION_OPTIMIZATION.md)** - How Map→Filter chains are automatically optimized
- **[Examples](examples/)** - Interactive demos and real-world use cases

## Category Theory Foundation

For those interested in the mathematical underpinnings:

Transducers are **natural transformations between fold functors**. A transducer transforms a reducing function:

```
∀Acc. ((Acc, Out) -> Acc) -> ((Acc, In) -> Acc)
```

This foundation guarantees:
- **Identity law**: `id ∘ f = f ∘ id = f`
- **Associativity**: `(f ∘ g) ∘ h = f ∘ (g ∘ h)`

The library includes comprehensive property-based tests verifying these laws.

## Development

### For Rust Developers

Orlando can also be used as a native Rust library:

```toml
[dependencies]
orlando-transducers = "0.5.0"
```

```rust
use orlando::*;

let pipeline = Map::new(|x: i32| x * 2)
    .compose(Filter::new(|x: &i32| *x % 3 == 0))
    .compose(Take::new(5));

let result = to_vec(&pipeline, 1..100);
// result: [6, 12, 18, 24, 30]
```

**Rust collectors:** `to_vec`, `sum`, `count`, `first`, `last`, `every`, `some`

### Building from Source

```bash
# Clone repository
git clone https://github.com/yourusername/orlando.git
cd orlando

# Install Git hooks (optional but recommended)
./scripts/setup-hooks.sh

# Run tests
cargo test --target x86_64-unknown-linux-gnu

# Build WASM for JavaScript
wasm-pack build --target web

# Build optimized WASM
wasm-pack build --target web --release
```

### Project Structure

```
orlando/
├── src/
│   ├── lib.rs                  # Core library & re-exports
│   ├── step.rs                 # Step monad (early termination)
│   ├── transducer.rs           # Transducer trait & composition
│   ├── transforms.rs           # Map, Filter, Take, etc.
│   ├── collectors.rs           # Terminal operations
│   ├── logic.rs                # Predicate combinators
│   ├── optics.rs               # Optics (Lens, Optional, Prism, Iso, Fold, Traversal)
│   ├── profunctor.rs           # Profunctor re-exports (Strong, Choice, Traversing)
│   ├── optics_wasm.rs          # Optics JavaScript WASM API
│   ├── geometric_optics.rs     # Multivector coefficient array operations
│   ├── geometric_optics_wasm.rs # Geometric optics WASM API
│   ├── signal.rs               # Reactive Signal<T>
│   ├── stream.rs               # Reactive Stream<T>
│   ├── iter_ext.rs             # Rust iterator extensions & PipelineBuilder
│   └── pipeline.rs             # JavaScript Pipeline WASM API
├── docs/api/                   # API documentation
├── examples/                   # Interactive HTML examples
├── tests/                      # Integration & property tests
└── benches/                    # Performance benchmarks
```

## Browser Compatibility

Orlando works in all modern browsers with WebAssembly support:

- ✅ Chrome 57+
- ✅ Firefox 52+
- ✅ Safari 11+
- ✅ Edge 16+
- ✅ Node.js 12+ (with WASM support)

## Contributing

Contributions welcome! Areas we'd love help with:

- Additional transformations (partition, chunk, etc.)
- Performance benchmarks
- Documentation improvements
- Real-world example applications

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Why "Orlando"?

Named after the bridger characters in Greg Egan's science fiction novel *Diaspora*, who facilitated transformation and change at fundamental levels. Transducers similarly transform the very nature of how we compose data operations.

## Inspiration & Resources

- **Clojure's Transducers** - Original inspiration ([docs](https://clojure.org/reference/transducers))
- **Rich Hickey's Talk** - "Transducers" ([video](https://www.youtube.com/watch?v=6mTbuzafcII))
- **Category Theory for Programmers** - Mathematical foundations ([book](https://github.com/hmemcpy/milewski-ctfp-pdf))
- **wasm-bindgen** - Rust/WASM interop ([repo](https://github.com/rustwasm/wasm-bindgen))

---

<p align="center">
  <strong>Transform transformations, not data.</strong> 🚀
  <br>
  <sub>Built with Rust • Powered by WebAssembly • Inspired by Category Theory</sub>
</p>
