# Getting Started

## Installation

```bash
npm install orlando-transducers
# or
yarn add orlando-transducers
# or
pnpm add orlando-transducers
```

### Using from CDN

```html
<script type="module">
  import init, { Pipeline } from 'https://unpkg.com/orlando-transducers';
  await init();
  // Use Pipeline...
</script>
```

## Initializing WASM

Orlando uses WebAssembly under the hood. You need to initialize the WASM module once before using any API:

```javascript
import init, { Pipeline } from 'orlando-transducers';

// Initialize WASM (once per application)
await init();
```

In a framework context, initialize in your app's entry point:

```javascript
// main.js / index.js
import init from 'orlando-transducers';

async function bootstrap() {
  await init();
  // Now all Orlando APIs are ready
  startApp();
}

bootstrap();
```

## Your First Pipeline

```javascript
import init, { Pipeline } from 'orlando-transducers';
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

Key concepts:
- **Pipelines are reusable** - define once, execute on any data
- **Fluent API** - chain transformations with method calls
- **Lazy execution** - nothing runs until you call a terminal operation (`.toArray()`, `.reduce()`, etc.)
- **Early termination** - `.take(5)` stops processing after 5 results

## TypeScript

Orlando works with TypeScript out of the box:

```typescript
import init, { Pipeline } from 'orlando-transducers';
await init();

interface User {
  id: number;
  name: string;
  email: string;
  active: boolean;
}

const activeEmails = new Pipeline()
  .filter((user: User) => user.active)
  .map((user: User) => user.email)
  .take(100);

const emails = activeEmails.toArray(users);
```

## Core Concepts

### Transformations vs Collectors

**Transformations** build up the pipeline:

```javascript
const pipeline = new Pipeline()
  .map(x => x * 2)        // transformation
  .filter(x => x > 10)    // transformation
  .take(5);                // transformation
```

**Collectors** (terminal operations) execute the pipeline and produce a result:

```javascript
const array = pipeline.toArray(data);                    // collect to array
const sum = pipeline.reduce(data, (a, b) => a + b, 0);  // reduce to value
```

### Pipeline Reuse

A key advantage over array methods is that pipelines are **reusable objects**:

```javascript
const normalize = new Pipeline()
  .filter(x => x != null)
  .map(x => x.trim().toLowerCase())
  .filter(x => x.length > 0);

// Use on different datasets
const emails = normalize.toArray(rawEmails);
const names = normalize.toArray(rawNames);
const tags = normalize.toArray(rawTags);
```

### Early Termination

Orlando stops processing the moment it has enough results:

```javascript
// Only processes ~13 elements out of 1,000,000
const result = new Pipeline()
  .map(x => x * 2)
  .filter(x => x % 3 === 0)
  .take(5)
  .toArray(Array.from({ length: 1_000_000 }, (_, i) => i));
```

This is where Orlando's biggest performance wins come from. Traditional array methods must process the entire array at every step.

## Using as a Rust Crate

Orlando is also a first-class Rust library:

```toml
[dependencies]
orlando-transducers = "0.6.0"
```

```rust
use orlando_transducers::iter_ext::PipelineBuilder;

let result = PipelineBuilder::new()
    .map(|x: i32| x * 2)
    .filter(|x: &i32| *x > 10)
    .take(5)
    .run(1..100);

assert_eq!(result, vec![12, 14, 16, 18, 20]);
```

## Browser Compatibility

Orlando works in all modern browsers with WebAssembly support:

- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+
- Node.js 12+
