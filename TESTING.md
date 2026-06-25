# Testing Strategy for Orlando

This document outlines the comprehensive testing strategy for the Orlando transducer library.

## Test Suite Overview

Orlando employs a multi-layered testing approach to ensure correctness, performance, and reliability:

1. **Unit Tests** - Test individual components in isolation
2. **Integration Tests** - Test complete pipelines and workflows
3. **Property-Based Tests** - Verify algebraic laws and invariants
4. **Fuzz Tests** - Find edge cases and bugs through randomization
5. **WASM Tests** - Verify JavaScript interop and browser compatibility
6. **Benchmarks** - Track performance over time

## Running Tests

### All Tests (Native)

```bash
cargo test --target x86_64-unknown-linux-gnu
```

### Unit Tests Only

```bash
cargo test --lib --target x86_64-unknown-linux-gnu
```

### Integration Tests

```bash
cargo test --test integration --target x86_64-unknown-linux-gnu
```

### Property-Based Tests

```bash
cargo test --test property_tests --target x86_64-unknown-linux-gnu
```

### WASM Tests

```bash
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

### Fuzz Testing

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run specific fuzz target
cargo fuzz run fuzz_transducer_pipeline

# Run with more iterations
cargo fuzz run fuzz_transducer_pipeline -- -runs=1000000

# Run all fuzz targets
cargo fuzz run fuzz_collectors
```

### Benchmarks

```bash
cargo bench --target x86_64-unknown-linux-gnu
```

### Code Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --target x86_64-unknown-linux-gnu --out Html

# Open coverage report
open tarpaulin-report.html
```

## Test Organization

### Unit Tests (`src/`)

Located alongside source code in each module:

- `src/step.rs` - Step monad tests
- `src/transducer.rs` - Transducer trait tests
- `src/transforms.rs` - Individual transform tests
- `src/collectors.rs` - Collector function tests

### Integration Tests (`tests/`)

- `tests/integration.rs` - End-to-end pipeline tests (19 tests)
- `tests/property_tests.rs` - Property-based tests (20+ properties)
- `tests/wasm_tests.rs` - WASM-specific tests (10 tests)

### Fuzz Tests (`fuzz/`)

- `fuzz/fuzz_targets/fuzz_transducer_pipeline.rs` - Pipeline fuzzing
- `fuzz/fuzz_targets/fuzz_collectors.rs` - Collector fuzzing

### Benchmarks (`benches/`)

- `benches/performance.rs` - Performance comparison benchmarks

## Property-Based Testing

Property-based tests verify algebraic laws and invariants using randomly generated data:

### Map Fusion Law
```
map(f) ∘ map(g) = map(g ∘ f)
```

### Filter Composition Law
```
filter(p) ∘ filter(q) = filter(λx. p(x) ∧ q(x))
```

### Identity Laws
```
id ∘ f = f
f ∘ id = f
```

### Associativity Law
```
(f ∘ g) ∘ h = f ∘ (g ∘ h)
```

### Bounds Checking
- `take(n)` produces at most `n` elements
- `filter` never increases length
- `map` preserves length

### Correctness Properties
- `sum` equals manual summation
- `count` equals vector length
- `first` equals first element
- `last` equals last element

## Fuzz Testing

Fuzz tests use `cargo-fuzz` (libFuzzer) to find edge cases:

### Coverage
- Pipeline composition with random operations
- All collector functions
- Early termination scenarios
- Large datasets
- Edge values (MIN, MAX, 0)

### Invariants Checked
- Result length ≤ input length (for most operations)
- No panics or crashes
- Consistent behavior with standard library

## WASM Testing

WASM tests verify browser compatibility and JavaScript interop:

### Browser Testing
- Firefox (headless)
- Chrome (headless)
- Real browser testing via `wasm-bindgen-test`

### Coverage
- Basic pipeline operations
- Early termination
- All collectors
- Category theory laws
- Step monad

## Continuous Integration

GitHub Actions runs the full test suite on:

### Platforms
- Ubuntu (Linux)
- macOS
- Windows

### Rust Versions
- Stable
- Beta  
- Nightly

### Checks
- ✅ Unit tests
- ✅ Integration tests
- ✅ Property-based tests
- ✅ WASM tests
- ✅ Benchmarks
- ✅ Code coverage
- ✅ Rustfmt (formatting)
- ✅ Clippy (linting)

## Coverage Goals

Target coverage: **>85%**

Current coverage by module:
- `step.rs` - ~95%
- `transducer.rs` - ~90%
- `transforms.rs` - ~90%
- `collectors.rs` - ~95%
- `pipeline.rs` - ~70% (WASM-only code)

## Test Data Strategy

### Small Datasets
- 0-10 elements for basic correctness
- Edge cases: empty, single element

### Medium Datasets
- 100-1,000 elements for typical usage
- Performance characteristics

### Large Datasets  
- 100,000-1,000,000 elements for stress testing
- Early termination efficiency
- Memory usage

### Edge Values
- `i32::MIN`, `i32::MAX`
- Zero
- Negative values
- Consecutive duplicates (for `unique`)

## Performance Testing

Benchmarks compare Orlando against:

1. **Standard library iterators** - Baseline comparison
2. **Manual for loops** - Theoretical best case
3. **Pure JS arrays** - WASM performance target

### Benchmark Scenarios
- `map → filter → take` (common pipeline)
- Complex 10-operation pipeline
- Early termination efficiency
- Large dataset processing
- Numeric operations (sum, count)

## Adding New Tests

When adding new features:

1. **Unit tests** - Test the component in isolation
2. **Integration test** - Test in a complete pipeline
3. **Property test** - Verify algebraic properties
4. **Fuzz test** - Add to fuzz targets if stateful
5. **WASM test** - If affects JavaScript API
6. **Benchmark** - If performance-critical

### Example: Adding a New Transducer

```rust
// 1. Unit test (in src/transforms.rs)
#[test]
fn test_my_transducer() {
    let pipeline = MyTransducer::new(params);
    let result = to_vec(&pipeline, input);
    assert_eq!(result, expected);
}

// 2. Integration test (in tests/integration.rs)
#[test]
fn test_my_transducer_in_pipeline() {
    let pipeline = Map::new(f)
        .compose(MyTransducer::new(params))
        .compose(Filter::new(p));
    // ...
}

// 3. Property test (in tests/property_tests.rs)
proptest! {
    #[test]
    fn test_my_transducer_property(vec in prop::collection::vec(any::<i32>(), 0..100)) {
        // Verify property holds for all inputs
    }
}

// 4. Update fuzz target (in fuzz/fuzz_targets/...)
// Add new operation variant

// 5. WASM test (in tests/wasm_tests.rs) if applicable
#[wasm_bindgen_test]
fn test_wasm_my_transducer() {
    // ...
}

// 6. Benchmark (in benches/performance.rs) if needed
fn benchmark_my_transducer(c: &mut Criterion) {
    // ...
}
```

## Test Naming Conventions

- Unit tests: `test_<feature>`
- Integration tests: `test_<scenario>`
- Property tests: `test_<property>_<law>`
- WASM tests: `test_wasm_<feature>`
- Fuzz targets: `fuzz_<component>`

## Debugging Failed Tests

### Property Test Failures

Property tests will output the minimal failing case:

```rust
thread 'test_map_fusion' panicked at 'Test failed: ...'
minimal failing input: vec = [42, -17, 0, MAX]
```

### Fuzz Test Crashes

Fuzz tests save failing inputs to `fuzz/artifacts/`:

```bash
# Reproduce a crash
cargo fuzz run fuzz_transducer_pipeline fuzz/artifacts/crash-abc123
```

### Coverage Gaps

Check coverage report for untested code paths:

```bash
cargo tarpaulin --target x86_64-unknown-linux-gnu --out Html
open tarpaulin-report.html
```

## Test Maintenance

- **Weekly**: Run full test suite including fuzz tests
- **Per commit**: Run unit + integration tests
- **Per release**: Run all tests + benchmarks + coverage
- **Monthly**: Review and update property tests

## Resources

- [Property-Based Testing](https://hypothesis.works/articles/what-is-property-based-testing/)
- [Fuzz Testing](https://rust-fuzz.github.io/book/)
- [wasm-bindgen Testing](https://rustwasm.github.io/wasm-bindgen/wasm-bindgen-test/index.html)
- [Criterion Benchmarks](https://bheisler.github.io/criterion.rs/book/)

---

**Questions?** Open an issue or check the [main README](README.md).
