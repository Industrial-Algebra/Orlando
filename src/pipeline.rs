//! WASM-compatible pipeline API for JavaScript interop.
//!
//! This module provides a fluent API for building transducer pipelines
//! that can be called from JavaScript via WASM.

use js_sys::{Array, Function, Object, Reflect};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::console;

/// A pipeline represents a composition of transducers that can be applied to data.
///
/// This is the main API for JavaScript consumers.
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { Pipeline } from './pkg/orlando.js';
///
/// const pipeline = new Pipeline()
///   .map(x => x * 2)
///   .filter(x => x > 5)
///   .take(3);
///
/// const result = pipeline.toArray([1, 2, 3, 4, 5, 6]);
/// console.log(result); // [6, 8, 10]
/// ```
#[wasm_bindgen]
pub struct Pipeline {
    operations: Vec<Operation>,
}

/// Internal representation of pipeline operations
#[derive(Clone)]
enum Operation {
    Map(Rc<dyn Fn(JsValue) -> JsValue>),
    Filter(Rc<dyn Fn(&JsValue) -> bool>),
    /// Fused Map + Filter for better performance
    MapFilter {
        map: Rc<dyn Fn(JsValue) -> JsValue>,
        filter: Rc<dyn Fn(&JsValue) -> bool>,
    },
    FlatMap(Rc<dyn Fn(JsValue) -> Vec<JsValue>>),
    Take(usize),
    TakeWhile(Rc<dyn Fn(&JsValue) -> bool>),
    Drop(usize),
    DropWhile(Rc<dyn Fn(&JsValue) -> bool>),
    Tap(Rc<dyn Fn(&JsValue)>),
    /// Invertible map: a streaming isomorphism pairing `to`/`from` (Layer A).
    ///
    /// Forward (as a [`Transducer`](crate::transducer::Transducer)) applies `to`;
    /// [`invert`](Pipeline::invert) swaps the two. Distinct from [`Map`](Self::Map),
    /// which is not reversible (no inverse is captured).
    IsoMap {
        to: Rc<dyn Fn(JsValue) -> JsValue>,
        from: Rc<dyn Fn(JsValue) -> JsValue>,
    },
}

#[wasm_bindgen]
impl Pipeline {
    /// Create a new empty pipeline.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Pipeline {
        Pipeline {
            operations: Vec::new(),
        }
    }

    /// Add a map operation to the pipeline.
    ///
    /// # Arguments
    ///
    /// * `f` - A JavaScript function that transforms each value
    #[wasm_bindgen]
    pub fn map(&self, f: &Function) -> Pipeline {
        let f = f.clone();
        let mut ops = self.operations.clone();

        let map_fn = Rc::new(move |val: JsValue| -> JsValue {
            let this = JsValue::null();
            f.call1(&this, &val).unwrap_or(JsValue::undefined())
        }) as Rc<dyn Fn(JsValue) -> JsValue>;

        ops.push(Operation::Map(map_fn));
        Pipeline { operations: ops }
    }

    /// Add a filter operation to the pipeline.
    ///
    /// # Arguments
    ///
    /// * `pred` - A JavaScript function that returns true for values to keep
    #[wasm_bindgen]
    pub fn filter(&self, pred: &Function) -> Pipeline {
        let pred = pred.clone();
        let mut ops = self.operations.clone();

        let filter_fn = Rc::new(move |val: &JsValue| -> bool {
            let this = JsValue::null();
            match pred.call1(&this, val) {
                Ok(result) => result.as_bool().unwrap_or(false),
                Err(_) => false,
            }
        }) as Rc<dyn Fn(&JsValue) -> bool>;

        // OPTIMIZATION: Fuse Map + Filter into a single operation
        // This reduces function call overhead and improves cache locality
        if let Some(Operation::Map(map_fn)) = ops.pop() {
            ops.push(Operation::MapFilter {
                map: map_fn,
                filter: filter_fn,
            });
        } else {
            ops.push(Operation::Filter(filter_fn));
        }

        Pipeline { operations: ops }
    }

    /// Add a flatMap operation to the pipeline.
    ///
    /// Maps each value to an array and flattens the results.
    ///
    /// # Arguments
    ///
    /// * `f` - A JavaScript function that returns an array for each value
    #[wasm_bindgen(js_name = flatMap)]
    pub fn flat_map(&self, f: &Function) -> Pipeline {
        let f = f.clone();
        let mut ops = self.operations.clone();

        let flatmap_fn = Rc::new(move |val: JsValue| -> Vec<JsValue> {
            let this = JsValue::null();
            match f.call1(&this, &val) {
                Ok(result) => {
                    // Convert JsValue array to Vec<JsValue>
                    if let Ok(array) = result.dyn_into::<Array>() {
                        (0..array.length()).map(|i| array.get(i)).collect()
                    } else {
                        vec![]
                    }
                }
                Err(_) => vec![],
            }
        }) as Rc<dyn Fn(JsValue) -> Vec<JsValue>>;

        ops.push(Operation::FlatMap(flatmap_fn));
        Pipeline { operations: ops }
    }

    /// Take the first n elements.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of elements to take
    #[wasm_bindgen]
    pub fn take(&self, n: usize) -> Pipeline {
        let mut ops = self.operations.clone();
        ops.push(Operation::Take(n));
        Pipeline { operations: ops }
    }

    /// Take elements while predicate is true.
    ///
    /// # Arguments
    ///
    /// * `pred` - A JavaScript function that returns true to continue taking
    #[wasm_bindgen(js_name = takeWhile)]
    pub fn take_while(&self, pred: &Function) -> Pipeline {
        let pred = pred.clone();
        let mut ops = self.operations.clone();
        ops.push(Operation::TakeWhile(Rc::new(move |val| {
            let this = JsValue::null();
            match pred.call1(&this, val) {
                Ok(result) => result.as_bool().unwrap_or(false),
                Err(_) => false,
            }
        })));
        Pipeline { operations: ops }
    }

    /// Skip the first n elements.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of elements to skip
    #[wasm_bindgen]
    pub fn drop(&self, n: usize) -> Pipeline {
        let mut ops = self.operations.clone();
        ops.push(Operation::Drop(n));
        Pipeline { operations: ops }
    }

    /// Skip elements while predicate is true.
    ///
    /// # Arguments
    ///
    /// * `pred` - A JavaScript function that returns true to continue skipping
    #[wasm_bindgen(js_name = dropWhile)]
    pub fn drop_while(&self, pred: &Function) -> Pipeline {
        let pred = pred.clone();
        let mut ops = self.operations.clone();
        ops.push(Operation::DropWhile(Rc::new(move |val| {
            let this = JsValue::null();
            match pred.call1(&this, val) {
                Ok(result) => result.as_bool().unwrap_or(false),
                Err(_) => false,
            }
        })));
        Pipeline { operations: ops }
    }

    /// Perform side effects without transforming values.
    ///
    /// # Arguments
    ///
    /// * `f` - A JavaScript function to call for each value
    #[wasm_bindgen]
    pub fn tap(&self, f: &Function) -> Pipeline {
        let f = f.clone();
        let mut ops = self.operations.clone();
        ops.push(Operation::Tap(Rc::new(move |val| {
            let this = JsValue::null();
            let _ = f.call1(&this, val);
        })));
        Pipeline { operations: ops }
    }

    /// Extract a property from each object (JavaScript convenience).
    ///
    /// This is cleaner than `.map(x => x.propertyName)` for extracting properties.
    ///
    /// # Arguments
    ///
    /// * `property_name` - The name of the property to extract
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const users = [
    ///   { name: 'Alice', age: 30 },
    ///   { name: 'Bob', age: 25 }
    /// ];
    /// const names = new Pipeline().pluck('name').toArray(users);
    /// // names: ['Alice', 'Bob']
    /// ```
    #[wasm_bindgen]
    pub fn pluck(&self, property_name: &str) -> Pipeline {
        let prop_key = JsValue::from_str(property_name);
        let mut ops = self.operations.clone();

        let map_fn = Rc::new(move |val: JsValue| -> JsValue {
            // Use Reflect.get to extract the property
            Reflect::get(&val, &prop_key).unwrap_or(JsValue::undefined())
        }) as Rc<dyn Fn(JsValue) -> JsValue>;

        ops.push(Operation::Map(map_fn));
        Pipeline { operations: ops }
    }

    /// Extract multiple properties from each object, creating new objects with only those keys.
    ///
    /// This is equivalent to `.map(x => ({ key1: x.key1, key2: x.key2, ... }))`.
    ///
    /// # Arguments
    ///
    /// * `keys` - A JavaScript array of property name strings to extract
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const users = [
    ///   { id: 1, name: 'Alice', age: 30, email: 'alice@example.com' },
    ///   { id: 2, name: 'Bob', age: 25, email: 'bob@example.com' }
    /// ];
    /// const projected = new Pipeline().project(['id', 'name']).toArray(users);
    /// // [{ id: 1, name: 'Alice' }, { id: 2, name: 'Bob' }]
    /// ```
    #[wasm_bindgen]
    pub fn project(&self, keys: &JsValue) -> Pipeline {
        let keys_array: Array = keys.clone().dyn_into().unwrap_or_else(|_| Array::new());

        // Pre-extract key strings to avoid repeated conversion
        let key_strings: Vec<JsValue> = (0..keys_array.length())
            .map(|i| keys_array.get(i))
            .collect();

        let mut ops = self.operations.clone();

        let map_fn = Rc::new(move |val: JsValue| -> JsValue {
            let result = Object::new();
            for key in &key_strings {
                if let Ok(prop_val) = Reflect::get(&val, key) {
                    let _ = Reflect::set(&result, key, &prop_val);
                }
            }
            result.into()
        }) as Rc<dyn Fn(JsValue) -> JsValue>;

        ops.push(Operation::Map(map_fn));
        Pipeline { operations: ops }
    }

    /// Remove all falsy values from the stream.
    ///
    /// Filters out `null`, `undefined`, `false`, `0`, `NaN`, and `''` (empty string).
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const data = [1, null, 'hello', undefined, 0, false, '', 42];
    /// const result = new Pipeline().compact().toArray(data);
    /// // [1, 'hello', 42]
    /// ```
    #[wasm_bindgen]
    pub fn compact(&self) -> Pipeline {
        let mut ops = self.operations.clone();

        let filter_fn = Rc::new(move |val: &JsValue| -> bool {
            val.as_bool() != Some(false)
                && !val.is_null()
                && !val.is_undefined()
                && val.as_f64() != Some(0.0)
                && val.as_string().is_none_or(|s| !s.is_empty())
                && !is_nan(val)
        }) as Rc<dyn Fn(&JsValue) -> bool>;

        ops.push(Operation::Filter(filter_fn));
        Pipeline { operations: ops }
    }

    /// Flatten nested arrays to a given depth.
    ///
    /// Each element in the stream that is an array will be expanded. Nesting
    /// is flattened recursively up to the specified depth.
    ///
    /// # Arguments
    ///
    /// * `depth` - Maximum depth to flatten. Use 1 for a single level of flattening.
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const data = [[1, 2], [3, [4, 5]], [6]];
    /// const result = new Pipeline().flatten(1).toArray(data);
    /// // [1, 2, 3, [4, 5], 6]
    ///
    /// const deep = new Pipeline().flatten(2).toArray(data);
    /// // [1, 2, 3, 4, 5, 6]
    /// ```
    #[wasm_bindgen]
    pub fn flatten(&self, depth: usize) -> Pipeline {
        let mut ops = self.operations.clone();

        let flatmap_fn = Rc::new(move |val: JsValue| -> Vec<JsValue> { flatten_value(val, depth) })
            as Rc<dyn Fn(JsValue) -> Vec<JsValue>>;

        ops.push(Operation::FlatMap(flatmap_fn));
        Pipeline { operations: ops }
    }

    /// Filter objects that match all properties in a spec object.
    ///
    /// Keeps only elements where every key in the spec has a matching value.
    /// Comparison uses strict equality (`===`-like semantics).
    ///
    /// # Arguments
    ///
    /// * `spec` - A JavaScript object whose key-value pairs must all match
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const users = [
    ///   { name: 'Alice', active: true, role: 'admin' },
    ///   { name: 'Bob', active: false, role: 'user' },
    ///   { name: 'Charlie', active: true, role: 'user' }
    /// ];
    ///
    /// const result = new Pipeline()
    ///   .where_matches({ active: true })
    ///   .toArray(users);
    /// // [{ name: 'Alice', ... }, { name: 'Charlie', ... }]
    /// ```
    #[wasm_bindgen(js_name = whereMatches)]
    pub fn where_matches(&self, spec: &JsValue) -> Pipeline {
        let spec_obj = spec.clone();

        // Pre-extract spec keys and values
        let spec_keys = Object::keys(&Object::from(spec_obj.clone()));
        let mut spec_entries: Vec<(JsValue, JsValue)> = Vec::new();
        for i in 0..spec_keys.length() {
            let key = spec_keys.get(i);
            if let Ok(val) = Reflect::get(&spec_obj, &key) {
                spec_entries.push((key, val));
            }
        }

        let mut ops = self.operations.clone();

        let filter_fn = Rc::new(move |val: &JsValue| -> bool {
            for (key, expected) in &spec_entries {
                match Reflect::get(val, key) {
                    Ok(actual) => {
                        if !js_strict_eq(&actual, expected) {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            }
            true
        }) as Rc<dyn Fn(&JsValue) -> bool>;

        ops.push(Operation::Filter(filter_fn));
        Pipeline { operations: ops }
    }

    /// Apply a lens to extract the focused value from each element.
    ///
    /// Equivalent to `.map(x => myLens.get(x))` but avoids the JS function call overhead.
    ///
    /// # Arguments
    ///
    /// * `optic` - A JsLens to apply
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const nameLens = lens('name');
    /// const users = [{ name: "Alice" }, { name: "Bob" }];
    /// const names = new Pipeline().viewLens(nameLens).toArray(users);
    /// // names: ["Alice", "Bob"]
    /// ```
    #[wasm_bindgen(js_name = viewLens)]
    pub fn view_lens(&self, optic: &crate::optics_wasm::JsLens) -> Pipeline {
        let get_fn = optic.get_fn.clone();
        let mut ops = self.operations.clone();

        let map_fn = Rc::new(move |val: JsValue| -> JsValue { get_fn(&val) })
            as Rc<dyn Fn(JsValue) -> JsValue>;

        ops.push(Operation::Map(map_fn));
        Pipeline { operations: ops }
    }

    /// Transform each element's focused value through a lens using a function.
    ///
    /// Equivalent to `.map(x => myLens.over(x, fn))`.
    ///
    /// # Arguments
    ///
    /// * `optic` - A JsLens to apply
    /// * `f` - A JavaScript function to transform the focused value
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const priceLens = lens('price');
    /// const items = [{ name: "A", price: 10 }, { name: "B", price: 20 }];
    /// const discounted = new Pipeline()
    ///   .overLens(priceLens, p => p * 0.9)
    ///   .toArray(items);
    /// // [{ name: "A", price: 9 }, { name: "B", price: 18 }]
    /// ```
    #[wasm_bindgen(js_name = overLens)]
    pub fn over_lens(&self, optic: &crate::optics_wasm::JsLens, f: &Function) -> Pipeline {
        let get_fn = optic.get_fn.clone();
        let set_fn = optic.set_fn.clone();
        let f = f.clone();
        let mut ops = self.operations.clone();

        let map_fn = Rc::new(move |val: JsValue| -> JsValue {
            let current = get_fn(&val);
            let this = JsValue::null();
            let updated = f.call1(&this, &current).unwrap_or_else(|_| current.clone());
            set_fn(&val, updated)
        }) as Rc<dyn Fn(JsValue) -> JsValue>;

        ops.push(Operation::Map(map_fn));
        Pipeline { operations: ops }
    }

    /// Filter elements based on a predicate applied to the focused value of a lens.
    ///
    /// Equivalent to `.filter(x => pred(myLens.get(x)))`.
    ///
    /// # Arguments
    ///
    /// * `optic` - A JsLens to extract the value to test
    /// * `pred` - A JavaScript predicate function
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const ageLens = lens('age');
    /// const users = [{ name: "Alice", age: 25 }, { name: "Bob", age: 17 }];
    /// const adults = new Pipeline()
    ///   .filterLens(ageLens, a => a >= 18)
    ///   .toArray(users);
    /// // [{ name: "Alice", age: 25 }]
    /// ```
    #[wasm_bindgen(js_name = filterLens)]
    pub fn filter_lens(&self, optic: &crate::optics_wasm::JsLens, pred: &Function) -> Pipeline {
        let get_fn = optic.get_fn.clone();
        let pred = pred.clone();
        let mut ops = self.operations.clone();

        let filter_fn = Rc::new(move |val: &JsValue| -> bool {
            let focused = get_fn(val);
            let this = JsValue::null();
            match pred.call1(&this, &focused) {
                Ok(result) => result.as_bool().unwrap_or(false),
                Err(_) => false,
            }
        }) as Rc<dyn Fn(&JsValue) -> bool>;

        ops.push(Operation::Filter(filter_fn));
        Pipeline { operations: ops }
    }

    /// Set the focused value of a lens on every element.
    ///
    /// Equivalent to `.map(x => myLens.set(x, value))`.
    ///
    /// # Arguments
    ///
    /// * `optic` - A JsLens to apply
    /// * `value` - The value to set
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const statusLens = lens('status');
    /// const items = [{ id: 1, status: "draft" }, { id: 2, status: "draft" }];
    /// const published = new Pipeline()
    ///   .setLens(statusLens, "published")
    ///   .toArray(items);
    /// // [{ id: 1, status: "published" }, { id: 2, status: "published" }]
    /// ```
    #[wasm_bindgen(js_name = setLens)]
    pub fn set_lens(&self, optic: &crate::optics_wasm::JsLens, value: JsValue) -> Pipeline {
        let set_fn = optic.set_fn.clone();
        let mut ops = self.operations.clone();

        let map_fn = Rc::new(move |val: JsValue| -> JsValue { set_fn(&val, value.clone()) })
            as Rc<dyn Fn(JsValue) -> JsValue>;

        ops.push(Operation::Map(map_fn));
        Pipeline { operations: ops }
    }

    /// Execute the pipeline and collect results into an array.
    ///
    /// # Arguments
    ///
    /// * `source` - JavaScript array to process
    #[wasm_bindgen(js_name = toArray)]
    pub fn to_array(&self, source: &Array) -> Array {
        let result = Array::new();
        let mut should_stop = false;
        let mut state = ProcessState::new();

        for i in 0..source.length() {
            if should_stop {
                break;
            }

            let val = source.get(i);
            let results = self.process_value_with_state(val, &mut state);

            for res in results {
                match res {
                    ProcessResult::Continue(v) => {
                        result.push(&v);
                    }
                    ProcessResult::Skip => {
                        // Skip this value
                    }
                    ProcessResult::Stop(v) => {
                        if let Some(val) = v {
                            result.push(&val);
                        }
                        should_stop = true;
                        break;
                    }
                }
            }
        }

        result
    }

    /// Reduce the source array with a custom reducer function.
    ///
    /// # Arguments
    ///
    /// * `source` - JavaScript array to process
    /// * `reducer` - JavaScript function (acc, val) => acc
    /// * `initial` - Initial accumulator value
    #[wasm_bindgen]
    pub fn reduce(&self, source: &Array, reducer: &Function, initial: JsValue) -> JsValue {
        let mut acc = initial;
        let mut should_stop = false;
        let mut state = ProcessState::new();

        for i in 0..source.length() {
            if should_stop {
                break;
            }

            let val = source.get(i);
            let results = self.process_value_with_state(val, &mut state);

            for res in results {
                match res {
                    ProcessResult::Continue(v) => {
                        let this = JsValue::null();
                        acc = reducer.call2(&this, &acc, &v).unwrap_or(acc);
                    }
                    ProcessResult::Skip => {}
                    ProcessResult::Stop(v) => {
                        if let Some(val) = v {
                            let this = JsValue::null();
                            acc = reducer.call2(&this, &acc, &val).unwrap_or(acc);
                        }
                        should_stop = true;
                        break;
                    }
                }
            }
        }

        acc
    }

    /// Log pipeline execution to console (for debugging).
    #[wasm_bindgen(js_name = logExecution)]
    pub fn log_execution(&self, source: &Array) -> Array {
        console::log_1(&"Pipeline execution:".into());

        let pipeline = self.tap(&Function::new_with_args("x", "console.log('Value:', x)"));

        pipeline.to_array(source)
    }

    // -----------------------------------------------------------------------
    // Reflection & inversion (inverse-transducer design, Steps 2/4 lifted to JS)
    // -----------------------------------------------------------------------

    /// Add an invertible map (streaming isomorphism) to the pipeline.
    ///
    /// Unlike [`map`](Self::map), an `isoMap` captures *both* directions, so a
    /// pipeline built entirely from `isoMap` stages can be
    /// [inverted](Self::invert).
    ///
    /// **Caller responsibility:** `to` and `from` must be true inverses.
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const toF = new Pipeline().isoMap(c => c * 9/5 + 32, f => (f - 32) * 5/9);
    /// ```
    #[wasm_bindgen(js_name = isoMap)]
    pub fn iso_map(&self, to: &Function, from: &Function) -> Pipeline {
        let to_fn = to.clone();
        let from_fn = from.clone();

        let to_rc = Rc::new(move |val: JsValue| -> JsValue {
            let this = JsValue::null();
            to_fn.call1(&this, &val).unwrap_or(JsValue::undefined())
        }) as Rc<dyn Fn(JsValue) -> JsValue>;

        let from_rc = Rc::new(move |val: JsValue| -> JsValue {
            let this = JsValue::null();
            from_fn.call1(&this, &val).unwrap_or(JsValue::undefined())
        }) as Rc<dyn Fn(JsValue) -> JsValue>;

        let mut ops = self.operations.clone();
        ops.push(Operation::IsoMap {
            to: to_rc,
            from: from_rc,
        });
        Pipeline { operations: ops }
    }

    /// Describe this pipeline as a serializable array of stage descriptors.
    ///
    /// Each element is a plain object `{ op: string, ...params }`. This is the
    /// JavaScript surface of the Rust [`StageSpec`](crate::describe::StageSpec)
    /// reflection layer — it enables pipeline-as-config, round-tripping through
    /// JSON, and visualization.
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const p = new Pipeline().map(x => x * 2).filter(x => x > 5).take(3);
    /// p.describe();
    /// // [
    /// //   { op: 'map' },
    /// //   { op: 'filter' },
    /// //   { op: 'take', n: 3 }
    /// // ]
    /// ```
    #[wasm_bindgen]
    pub fn describe(&self) -> Array {
        let out = Array::new();
        for op in &self.operations {
            let obj = Object::new();
            match op {
                Operation::Map(_) => {
                    Reflect::set(&obj, &"op".into(), &"map".into()).ok();
                }
                Operation::Filter(_) => {
                    Reflect::set(&obj, &"op".into(), &"filter".into()).ok();
                }
                Operation::MapFilter { .. } => {
                    // Fused map+filter — reported truthfully as one fused stage.
                    Reflect::set(&obj, &"op".into(), &"mapFilter".into()).ok();
                }
                Operation::FlatMap(_) => {
                    Reflect::set(&obj, &"op".into(), &"flatMap".into()).ok();
                }
                Operation::Take(n) => {
                    Reflect::set(&obj, &"op".into(), &"take".into()).ok();
                    Reflect::set(&obj, &"n".into(), &JsValue::from(*n)).ok();
                }
                Operation::TakeWhile(_) => {
                    Reflect::set(&obj, &"op".into(), &"takeWhile".into()).ok();
                }
                Operation::Drop(n) => {
                    Reflect::set(&obj, &"op".into(), &"drop".into()).ok();
                    Reflect::set(&obj, &"n".into(), &JsValue::from(*n)).ok();
                }
                Operation::DropWhile(_) => {
                    Reflect::set(&obj, &"op".into(), &"dropWhile".into()).ok();
                }
                Operation::Tap(_) => {
                    Reflect::set(&obj, &"op".into(), &"tap".into()).ok();
                }
                Operation::IsoMap { .. } => {
                    Reflect::set(&obj, &"op".into(), &"isoMap".into()).ok();
                    Reflect::set(&obj, &"reversible".into(), &JsValue::from_bool(true)).ok();
                }
            }
            out.push(&obj);
        }
        out
    }

    /// Whether every stage in this pipeline is invertible (the bijective subset).
    ///
    /// Returns `true` only when all stages are `isoMap` (or the pipeline is
    /// empty, which is trivially the identity and thus invertible). Lossy stages
    /// (`filter`, `take`, `drop`, plain `map`, …) make this return `false`.
    ///
    /// Use this to guard [`invert`](Self::invert) without catching an exception.
    #[wasm_bindgen(js_name = canInvert)]
    pub fn can_invert(&self) -> bool {
        self.operations
            .iter()
            .all(|op| matches!(op, Operation::IsoMap { .. }))
    }

    /// Produce the inverse pipeline.
    ///
    /// Reverses stage order and swaps each `isoMap`'s `to`/`from`, realizing
    /// the groupoid law `(a ∘ b)⁻¹ = b⁻¹ ∘ a⁻¹`. An empty pipeline inverts to
    /// an empty pipeline (the identity is self-inverse).
    ///
    /// **Throws** a JavaScript `Error` if the pipeline is not invertible (i.e.
    /// [`canInvert`](Self::can_invert) is false). Check with `canInvert()` first
    /// if you prefer not to catch.
    ///
    /// # Examples (JavaScript)
    ///
    /// ```javascript
    /// const toF = new Pipeline()
    ///   .isoMap(c => c * 9/5 + 32, f => (f - 32) * 5/9)
    ///   .isoMap(x => x + 10, y => y - 10);
    /// const toC = toF.invert();
    /// ```
    #[wasm_bindgen]
    pub fn invert(&self) -> Result<Pipeline, JsValue> {
        if !self.can_invert() {
            return Err(JsValue::from_str(
                "Pipeline is not invertible: it contains lossy stages \
                 (filter, take, drop, map, ...). Only isoMap stages are reversible. \
                 Call canInvert() to check beforehand.",
            ));
        }

        // Reverse order and swap to/from for each IsoMap. The empty pipeline
        // (identity) maps to an empty pipeline.
        let ops: Vec<Operation> = self.operations.iter().rev().map(|op| match op {
            Operation::IsoMap { to, from } => Operation::IsoMap {
                to: Rc::clone(from),
                from: Rc::clone(to),
            },
            // can_invert() guarantees only IsoMap reaches here.
            _ => unreachable!("can_invert guards invert: only IsoMap is reversible"),
        }).collect();

        Ok(Pipeline { operations: ops })
    }

    // Internal helper to process a single value through the pipeline
    fn process_value_with_state(
        &self,
        val: JsValue,
        state: &mut ProcessState,
    ) -> Vec<ProcessResult> {
        self.process_value_from(val, 0, state)
    }

    // Process a value starting from a specific operation index
    #[allow(unused_assignments)]
    fn process_value_from(
        &self,
        mut val: JsValue,
        start_idx: usize,
        state: &mut ProcessState,
    ) -> Vec<ProcessResult> {
        for (idx, op) in self.operations.iter().enumerate().skip(start_idx) {
            match op {
                Operation::Map(f) => {
                    val = f(val);
                }
                Operation::Filter(pred) => {
                    if !pred(&val) {
                        return vec![ProcessResult::Skip];
                    }
                }
                // OPTIMIZED: Fused Map + Filter in single operation
                // This eliminates one function call and one match arm per element
                Operation::MapFilter { map, filter } => {
                    val = map(val);
                    if !filter(&val) {
                        return vec![ProcessResult::Skip];
                    }
                }
                Operation::FlatMap(f) => {
                    // Expand the value into multiple values
                    let expanded = f(val);
                    let mut results = Vec::new();

                    // Process each expanded value through the remaining operations
                    for expanded_val in expanded {
                        let sub_results = self.process_value_from(expanded_val, idx + 1, state);

                        // Check if we should stop early
                        let should_stop = sub_results
                            .iter()
                            .any(|r| matches!(r, ProcessResult::Stop(_)));

                        results.extend(sub_results);

                        if should_stop {
                            break;
                        }
                    }

                    return results;
                }
                Operation::Take(n) => {
                    state.take_count += 1;
                    if state.take_count > *n {
                        return vec![ProcessResult::Stop(None)];
                    }
                }
                Operation::TakeWhile(pred) => {
                    if !pred(&val) {
                        return vec![ProcessResult::Stop(None)];
                    }
                }
                Operation::Drop(n) => {
                    if state.drop_count < *n {
                        state.drop_count += 1;
                        return vec![ProcessResult::Skip];
                    }
                }
                Operation::DropWhile(pred) => {
                    if !state.dropping && pred(&val) {
                        return vec![ProcessResult::Skip];
                    } else {
                        state.dropping = false;
                    }
                }
                Operation::Tap(f) => {
                    f(&val);
                }
                Operation::IsoMap { to, .. } => {
                    val = to(val);
                }
            }
        }

        vec![ProcessResult::Continue(val)]
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

enum ProcessResult {
    Continue(JsValue),
    Skip,
    Stop(Option<JsValue>),
}

/// State maintained during pipeline processing
struct ProcessState {
    take_count: usize,
    drop_count: usize,
    dropping: bool,
}

impl ProcessState {
    fn new() -> Self {
        ProcessState {
            take_count: 0,
            drop_count: 0,
            dropping: false,
        }
    }
}

// Export convenience functions

/// Create a new pipeline.
#[wasm_bindgen(js_name = pipeline)]
pub fn create_pipeline() -> Pipeline {
    Pipeline::new()
}

// ============================================================================
// Multi-Input Operations (Phase 2a)
// ============================================================================

/// Merge multiple arrays by interleaving their elements in round-robin fashion.
///
/// Takes elements from each array in turn until all arrays are exhausted.
/// If arrays have different lengths, continues with remaining arrays.
///
/// # JavaScript Example
///
/// ```javascript
/// import { merge } from 'orlando-transducers';
///
/// const a = [1, 2, 3];
/// const b = [4, 5, 6];
/// const result = merge([a, b]);
/// // result: [1, 4, 2, 5, 3, 6]
/// ```
#[wasm_bindgen]
pub fn merge(arrays: Array) -> Array {
    let result = Array::new();

    // Convert JS arrays to iterators
    let mut iters: Vec<_> = (0..arrays.length())
        .map(|i| {
            let arr = arrays
                .get(i)
                .dyn_into::<Array>()
                .unwrap_or_else(|_| Array::new());
            (arr, 0)
        })
        .collect();

    let mut active = true;
    while active {
        active = false;
        for (arr, idx) in &mut iters {
            if *idx < arr.length() {
                result.push(&arr.get(*idx));
                *idx += 1;
                active = true;
            }
        }
    }

    result
}

/// Compute the intersection of two arrays (elements in both A and B).
///
/// Returns elements that appear in both arrays, preserving order from the first array.
/// Duplicates from the first array are included if the element exists in the second.
///
/// # JavaScript Example
///
/// ```javascript
/// import { intersection } from 'orlando-transducers';
///
/// const a = [1, 2, 3, 4];
/// const b = [3, 4, 5, 6];
/// const result = intersection(a, b);
/// // result: [3, 4]
/// ```
#[wasm_bindgen]
pub fn intersection(array_a: &Array, array_b: &Array) -> Array {
    use std::collections::HashSet;

    // Build a set from array B for O(1) lookup
    let mut set_b = HashSet::new();
    for i in 0..array_b.length() {
        let val = array_b.get(i);
        // Use JSON stringification for comparison (works for primitives and objects)
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            set_b.insert(json.as_string().unwrap_or_default());
        }
    }

    let result = Array::new();
    for i in 0..array_a.length() {
        let val = array_a.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            if set_b.contains(&json.as_string().unwrap_or_default()) {
                result.push(&val);
            }
        }
    }

    result
}

/// Compute the difference of two arrays (elements in A but not in B).
///
/// Returns elements from the first array that don't appear in the second,
/// preserving order from the first array.
///
/// # JavaScript Example
///
/// ```javascript
/// import { difference } from 'orlando-transducers';
///
/// const a = [1, 2, 3, 4];
/// const b = [3, 4, 5, 6];
/// const result = difference(a, b);
/// // result: [1, 2]
/// ```
#[wasm_bindgen]
pub fn difference(array_a: &Array, array_b: &Array) -> Array {
    use std::collections::HashSet;

    // Build a set from array B for O(1) lookup
    let mut set_b = HashSet::new();
    for i in 0..array_b.length() {
        let val = array_b.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            set_b.insert(json.as_string().unwrap_or_default());
        }
    }

    let result = Array::new();
    for i in 0..array_a.length() {
        let val = array_a.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            if !set_b.contains(&json.as_string().unwrap_or_default()) {
                result.push(&val);
            }
        }
    }

    result
}

/// Compute the union of two arrays (unique elements from both A and B).
///
/// Returns all unique elements that appear in either array.
/// Order is preserved: all unique elements from A first, then unique elements from B.
///
/// # JavaScript Example
///
/// ```javascript
/// import { union } from 'orlando-transducers';
///
/// const a = [1, 2, 3];
/// const b = [3, 4, 5];
/// const result = union(a, b);
/// // result: [1, 2, 3, 4, 5]
/// ```
#[wasm_bindgen]
pub fn union(array_a: &Array, array_b: &Array) -> Array {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let result = Array::new();

    // Add unique elements from A
    for i in 0..array_a.length() {
        let val = array_a.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            if seen.insert(json.as_string().unwrap_or_default()) {
                result.push(&val);
            }
        }
    }

    // Add unique elements from B
    for i in 0..array_b.length() {
        let val = array_b.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            if seen.insert(json.as_string().unwrap_or_default()) {
                result.push(&val);
            }
        }
    }

    result
}

/// Compute the symmetric difference of two arrays (elements in A or B but not both).
///
/// Returns elements that appear in exactly one of the two arrays.
/// Order: unique-to-A elements first, then unique-to-B elements.
///
/// # JavaScript Example
///
/// ```javascript
/// import { symmetricDifference } from 'orlando-transducers';
///
/// const a = [1, 2, 3, 4];
/// const b = [3, 4, 5, 6];
/// const result = symmetricDifference(a, b);
/// // result: [1, 2, 5, 6]
/// ```
#[wasm_bindgen(js_name = symmetricDifference)]
pub fn symmetric_difference(array_a: &Array, array_b: &Array) -> Array {
    use std::collections::HashSet;

    // Build sets from both arrays
    let mut set_a = HashSet::new();
    for i in 0..array_a.length() {
        let val = array_a.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            set_a.insert(json.as_string().unwrap_or_default());
        }
    }

    let mut set_b = HashSet::new();
    for i in 0..array_b.length() {
        let val = array_b.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            set_b.insert(json.as_string().unwrap_or_default());
        }
    }

    let result = Array::new();
    let mut seen = HashSet::new();

    // Elements in A but not B
    for i in 0..array_a.length() {
        let val = array_a.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            let json_str = json.as_string().unwrap_or_default();
            if !set_b.contains(&json_str) && seen.insert(json_str) {
                result.push(&val);
            }
        }
    }

    // Elements in B but not A
    for i in 0..array_b.length() {
        let val = array_b.get(i);
        if let Ok(json) = js_sys::JSON::stringify(&val) {
            let json_str = json.as_string().unwrap_or_default();
            if !set_a.contains(&json_str) && seen.insert(json_str) {
                result.push(&val);
            }
        }
    }

    result
}

// ============================================================================
// Phase 2b: Additional Operations
// ============================================================================

/// Take the last N elements from an array.
///
/// This operation processes the entire array and returns only the last N elements.
/// Unlike `take()`, which stops early, this requires buffering the full result.
///
/// # JavaScript Example
///
/// ```javascript
/// import { takeLast } from 'orlando-transducers';
///
/// const data = [1, 2, 3, 4, 5];
/// const result = takeLast(data, 3);
/// // result: [3, 4, 5]
/// ```
#[wasm_bindgen(js_name = takeLast)]
pub fn take_last(source: &Array, n: u32) -> Array {
    let len = source.length();

    let result = Array::new();

    if n >= len {
        // Return all elements if n is greater than or equal to array length
        for i in 0..len {
            result.push(&source.get(i));
        }
    } else {
        // Return last n elements
        let start = len - n;
        for i in start..len {
            result.push(&source.get(i));
        }
    }

    result
}

/// Drop the last N elements from an array.
///
/// This operation processes the entire array and returns all elements except the last N.
///
/// # JavaScript Example
///
/// ```javascript
/// import { dropLast } from 'orlando-transducers';
///
/// const data = [1, 2, 3, 4, 5];
/// const result = dropLast(data, 2);
/// // result: [1, 2, 3]
/// ```
#[wasm_bindgen(js_name = dropLast)]
pub fn drop_last(source: &Array, n: u32) -> Array {
    let len = source.length();

    let result = Array::new();

    if n >= len {
        // Return empty array if n is greater than or equal to array length
        return result;
    }

    // Return all but last n elements
    let end = len - n;
    for i in 0..end {
        result.push(&source.get(i));
    }

    result
}

/// Create sliding windows of size N over an array.
///
/// Returns an array of arrays, where each sub-array is a window of N consecutive elements.
/// Windows overlap - each window slides by one element.
///
/// # JavaScript Example
///
/// ```javascript
/// import { aperture } from 'orlando-transducers';
///
/// const data = [1, 2, 3, 4, 5];
/// const result = aperture(data, 3);
/// // result: [[1, 2, 3], [2, 3, 4], [3, 4, 5]]
/// ```
#[wasm_bindgen]
pub fn aperture(source: &Array, size: u32) -> Array {
    let len = source.length();
    let result = Array::new();

    if size == 0 || size > len {
        return result;
    }

    // Create sliding windows
    for i in 0..=(len - size) {
        let window = Array::new();
        for j in 0..size {
            window.push(&source.get(i + j));
        }
        result.push(&window);
    }

    result
}

// ============================================================================
// Phase 4: Aggregation & Statistical Operations (JavaScript Bindings)
// ============================================================================

/// Calculate the product of all numbers in an array.
#[wasm_bindgen]
pub fn product(source: &Array) -> f64 {
    let mut result = 1.0;
    for i in 0..source.length() {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            result *= num;
        }
    }
    result
}

/// Calculate the arithmetic mean (average) of numbers in an array.
#[wasm_bindgen]
pub fn mean(source: &Array) -> JsValue {
    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut sum = 0.0;
    for i in 0..len {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            sum += num;
        }
    }

    JsValue::from_f64(sum / (len as f64))
}

/// Find the median (middle value) of numbers in an array.
#[wasm_bindgen]
pub fn median(source: &Array) -> JsValue {
    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut values: Vec<f64> = Vec::new();
    for i in 0..len {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            values.push(num);
        }
    }

    if values.is_empty() {
        return JsValue::undefined();
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        JsValue::from_f64(values[mid])
    } else {
        JsValue::from_f64((values[mid - 1] + values[mid]) / 2.0)
    }
}

/// Find the minimum value in an array.
#[wasm_bindgen]
pub fn min(source: &Array) -> JsValue {
    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut min_val = f64::INFINITY;
    for i in 0..len {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            if num < min_val {
                min_val = num;
            }
        }
    }

    if min_val == f64::INFINITY {
        JsValue::undefined()
    } else {
        JsValue::from_f64(min_val)
    }
}

/// Find the maximum value in an array.
#[wasm_bindgen]
pub fn max(source: &Array) -> JsValue {
    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut max_val = f64::NEG_INFINITY;
    for i in 0..len {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            if num > max_val {
                max_val = num;
            }
        }
    }

    if max_val == f64::NEG_INFINITY {
        JsValue::undefined()
    } else {
        JsValue::from_f64(max_val)
    }
}

/// Find the element with the minimum value for a given key function.
#[wasm_bindgen(js_name = minBy)]
pub fn min_by(source: &Array, key_fn: &Function) -> JsValue {
    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut min_element = JsValue::undefined();
    let mut min_key = f64::INFINITY;

    let this = JsValue::null();
    for i in 0..len {
        let element = source.get(i);
        if let Ok(key_val) = key_fn.call1(&this, &element) {
            if let Some(key) = key_val.as_f64() {
                if key < min_key {
                    min_key = key;
                    min_element = element;
                }
            }
        }
    }

    min_element
}

/// Find the element with the maximum value for a given key function.
#[wasm_bindgen(js_name = maxBy)]
pub fn max_by(source: &Array, key_fn: &Function) -> JsValue {
    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut max_element = JsValue::undefined();
    let mut max_key = f64::NEG_INFINITY;

    let this = JsValue::null();
    for i in 0..len {
        let element = source.get(i);
        if let Ok(key_val) = key_fn.call1(&this, &element) {
            if let Some(key) = key_val.as_f64() {
                if key > max_key {
                    max_key = key;
                    max_element = element;
                }
            }
        }
    }

    max_element
}

/// Calculate the variance of numbers in an array.
#[wasm_bindgen]
pub fn variance(source: &Array) -> JsValue {
    let len = source.length();
    if len < 2 {
        return JsValue::undefined();
    }

    let mut values: Vec<f64> = Vec::new();
    for i in 0..len {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            values.push(num);
        }
    }

    if values.len() < 2 {
        return JsValue::undefined();
    }

    let n = values.len() as f64;
    let mean_val: f64 = values.iter().sum::<f64>() / n;

    let sum_squared_diff: f64 = values
        .iter()
        .map(|x| {
            let diff = x - mean_val;
            diff * diff
        })
        .sum();

    JsValue::from_f64(sum_squared_diff / (n - 1.0))
}

/// Calculate the standard deviation of numbers in an array.
#[wasm_bindgen(js_name = stdDev)]
pub fn std_dev(source: &Array) -> JsValue {
    match variance(source) {
        v if v.is_undefined() => JsValue::undefined(),
        v => {
            let var_val = v.as_f64().unwrap();
            JsValue::from_f64(var_val.sqrt())
        }
    }
}

/// Calculate a quantile (percentile) value.
#[wasm_bindgen]
pub fn quantile(source: &Array, p: f64) -> JsValue {
    if !(0.0..=1.0).contains(&p) {
        return JsValue::undefined();
    }

    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut values: Vec<f64> = Vec::new();
    for i in 0..len {
        let val = source.get(i);
        if let Some(num) = val.as_f64() {
            values.push(num);
        }
    }

    if values.is_empty() {
        return JsValue::undefined();
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = values.len();
    if n == 1 {
        return JsValue::from_f64(values[0]);
    }

    let index = p * (n - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        JsValue::from_f64(values[lower])
    } else {
        let weight = index - lower as f64;
        JsValue::from_f64(values[lower] * (1.0 - weight) + values[upper] * weight)
    }
}

/// Find the mode (most frequent element) in an array.
#[wasm_bindgen]
pub fn mode(source: &Array) -> JsValue {
    use std::collections::HashMap;

    let len = source.length();
    if len == 0 {
        return JsValue::undefined();
    }

    let mut freq_map: HashMap<String, (JsValue, usize)> = HashMap::new();

    for i in 0..len {
        let element = source.get(i);
        let key = format!("{:?}", element);

        freq_map
            .entry(key)
            .and_modify(|(_, count)| *count += 1)
            .or_insert((element.clone(), 1));
    }

    freq_map
        .into_iter()
        .max_by_key(|(_, (_, count))| *count)
        .map(|(_, (value, _))| value)
        .unwrap_or(JsValue::undefined())
}

// ============================================================================
// Phase 5c: Path Operations (JavaScript-specific)
// ============================================================================

/// Access a nested property using a path array.
///
/// Returns undefined if any part of the path doesn't exist.
///
/// # JavaScript Example
///
/// ```javascript
/// import { path } from 'orlando-transducers';
///
/// const user = {
///   profile: {
///     name: 'Alice',
///     address: {
///       city: 'NYC'
///     }
///   }
/// };
///
/// const name = path(user, ['profile', 'name']);
/// // 'Alice'
///
/// const city = path(user, ['profile', 'address', 'city']);
/// // 'NYC'
///
/// const missing = path(user, ['profile', 'age']);
/// // undefined
/// ```
#[wasm_bindgen]
pub fn path(obj: &JsValue, path_array: &Array) -> JsValue {
    let mut current = obj.clone();

    for i in 0..path_array.length() {
        let key = path_array.get(i);

        if let Some(key_str) = key.as_string() {
            match Reflect::get(&current, &JsValue::from_str(&key_str)) {
                Ok(value) => {
                    if value.is_undefined() {
                        return JsValue::undefined();
                    }
                    current = value;
                }
                Err(_) => return JsValue::undefined(),
            }
        } else {
            return JsValue::undefined();
        }
    }

    current
}

/// Access a nested property with a default value.
///
/// Returns the default value if any part of the path doesn't exist.
///
/// # JavaScript Example
///
/// ```javascript
/// import { pathOr } from 'orlando-transducers';
///
/// const user = {
///   profile: {
///     name: 'Alice'
///   }
/// };
///
/// const name = pathOr(user, ['profile', 'name'], 'Anonymous');
/// // 'Alice'
///
/// const age = pathOr(user, ['profile', 'age'], 0);
/// // 0 (default value)
/// ```
#[wasm_bindgen(js_name = pathOr)]
pub fn path_or(obj: &JsValue, path_array: &Array, default: &JsValue) -> JsValue {
    let result = path(obj, path_array);

    if result.is_undefined() {
        default.clone()
    } else {
        result
    }
}

/// Transform nested properties using a transformation object.
///
/// Applies transformation functions to specific paths in an object,
/// returning a new object with the transformations applied.
///
/// # JavaScript Example
///
/// ```javascript
/// import { evolve } from 'orlando-transducers';
///
/// const user = {
///   name: 'alice',
///   age: 25,
///   profile: {
///     bio: 'hello world'
///   }
/// };
///
/// const transformations = {
///   name: (n) => n.toUpperCase(),
///   age: (a) => a + 1,
///   'profile.bio': (b) => b + '!'
/// };
///
/// const evolved = evolve(user, transformations);
/// // {
/// //   name: 'ALICE',
/// //   age: 26,
/// //   profile: { bio: 'hello world!' }
/// // }
/// ```
#[wasm_bindgen]
pub fn evolve(obj: &JsValue, transformations: &JsValue) -> Result<JsValue, JsValue> {
    // Clone the object to avoid mutation
    let json_string = js_sys::JSON::stringify(obj)?;
    let result = js_sys::JSON::parse(&json_string.as_string().unwrap_or_default())?;

    // Get all transformation keys
    let keys = js_sys::Object::keys(&js_sys::Object::from(transformations.clone()));

    for i in 0..keys.length() {
        let key_str = keys.get(i).as_string().unwrap();

        // Get the transformation function
        if let Ok(transform_fn) = Reflect::get(transformations, &JsValue::from_str(&key_str)) {
            if !transform_fn.is_function() {
                continue;
            }

            let func = js_sys::Function::from(transform_fn);

            // Handle nested paths (e.g., "profile.bio")
            if key_str.contains('.') {
                let path_parts: Vec<&str> = key_str.split('.').collect();
                let path_array = Array::new();
                for part in &path_parts {
                    path_array.push(&JsValue::from_str(part));
                }

                // Get the current value at this path
                let current_value = path(&result, &path_array);

                if !current_value.is_undefined() {
                    // Apply transformation
                    let this = JsValue::null();
                    if let Ok(new_value) = func.call1(&this, &current_value) {
                        // Set the new value at the nested path
                        let mut target = result.clone();
                        for (idx, part) in path_parts.iter().enumerate() {
                            if idx == path_parts.len() - 1 {
                                // Last part - set the value
                                let _ = Reflect::set(&target, &JsValue::from_str(part), &new_value);
                            } else {
                                // Navigate deeper
                                if let Ok(next) = Reflect::get(&target, &JsValue::from_str(part)) {
                                    target = next;
                                }
                            }
                        }
                    }
                }
            } else {
                // Simple top-level property
                if let Ok(current_value) = Reflect::get(&result, &JsValue::from_str(&key_str)) {
                    let this = JsValue::null();
                    if let Ok(new_value) = func.call1(&this, &current_value) {
                        let _ = Reflect::set(&result, &JsValue::from_str(&key_str), &new_value);
                    }
                }
            }
        }
    }

    Ok(result)
}

// ============================================================================
// Phase 5a & 5b: Collection Utilities (JavaScript Bindings)
// ============================================================================

/// Sort array elements by a key function.
///
/// # JavaScript Example
///
/// ```javascript
/// import { sortBy } from 'orlando-transducers';
///
/// const users = [
///   { name: 'Alice', age: 30 },
///   { name: 'Bob', age: 25 },
///   { name: 'Charlie', age: 35 }
/// ];
///
/// const sorted = sortBy(users, u => u.age);
/// // [{ name: 'Bob', age: 25 }, ...]
/// ```
#[wasm_bindgen(js_name = sortBy)]
pub fn sort_by(source: &Array, key_fn: &Function) -> Array {
    let len = source.length();
    let mut items: Vec<(JsValue, f64)> = Vec::new();

    let this = JsValue::null();
    for i in 0..len {
        let item = source.get(i);
        if let Ok(key) = key_fn.call1(&this, &item) {
            if let Some(key_num) = key.as_f64() {
                items.push((item, key_num));
            }
        }
    }

    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let result = Array::new();
    for (item, _) in items {
        result.push(&item);
    }
    result
}

/// Sort array elements with a custom comparator.
///
/// # JavaScript Example
///
/// ```javascript
/// import { sortWith } from 'orlando-transducers';
///
/// const numbers = [3, 1, 4, 1, 5, 9, 2, 6];
/// const descending = sortWith(numbers, (a, b) => b - a);
/// // [9, 6, 5, 4, 3, 2, 1, 1]
/// ```
#[wasm_bindgen(js_name = sortWith)]
pub fn sort_with(source: &Array, comparator: &Function) -> Array {
    let len = source.length();
    let mut items: Vec<JsValue> = Vec::new();

    for i in 0..len {
        items.push(source.get(i));
    }

    let this = JsValue::null();
    items.sort_by(|a, b| {
        if let Ok(result) = comparator.call2(&this, a, b) {
            if let Some(cmp) = result.as_f64() {
                if cmp < 0.0 {
                    return std::cmp::Ordering::Less;
                } else if cmp > 0.0 {
                    return std::cmp::Ordering::Greater;
                }
            }
        }
        std::cmp::Ordering::Equal
    });

    let result = Array::new();
    for item in items {
        result.push(&item);
    }
    result
}

/// Reverse the order of array elements.
///
/// # JavaScript Example
///
/// ```javascript
/// import { reverse } from 'orlando-transducers';
///
/// const reversed = reverse([1, 2, 3, 4, 5]);
/// // [5, 4, 3, 2, 1]
/// ```
#[wasm_bindgen]
pub fn reverse(source: &Array) -> Array {
    let len = source.length();
    let result = Array::new();

    for i in (0..len).rev() {
        result.push(&source.get(i));
    }

    result
}

/// Generate a range of numbers.
///
/// # JavaScript Example
///
/// ```javascript
/// import { range } from 'orlando-transducers';
///
/// const numbers = range(0, 10, 1);
/// // [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
///
/// const evens = range(0, 20, 2);
/// // [0, 2, 4, 6, 8, 10, 12, 14, 16, 18]
/// ```
#[wasm_bindgen]
pub fn range(start: i32, end: i32, step: i32) -> Array {
    let result = Array::new();

    if step == 0 {
        return result;
    }

    let mut current = start;
    if step > 0 {
        while current < end {
            result.push(&JsValue::from_f64(current as f64));
            current += step;
        }
    } else {
        while current > end {
            result.push(&JsValue::from_f64(current as f64));
            current += step;
        }
    }

    result
}

/// Repeat a value N times.
///
/// # JavaScript Example
///
/// ```javascript
/// import { repeat } from 'orlando-transducers';
///
/// const zeros = repeat(0, 5);
/// // [0, 0, 0, 0, 0]
///
/// const template = repeat({ status: 'pending' }, 3);
/// // [{ status: 'pending' }, ...]
/// ```
#[wasm_bindgen]
pub fn repeat(value: &JsValue, n: u32) -> Array {
    let result = Array::new();

    for _ in 0..n {
        result.push(value);
    }

    result
}

/// Repeat an array N times.
///
/// # JavaScript Example
///
/// ```javascript
/// import { cycle } from 'orlando-transducers';
///
/// const pattern = cycle([1, 2, 3], 3);
/// // [1, 2, 3, 1, 2, 3, 1, 2, 3]
/// ```
#[wasm_bindgen]
pub fn cycle(source: &Array, n: u32) -> Array {
    let len = source.length();
    let result = Array::new();

    for _ in 0..n {
        for i in 0..len {
            result.push(&source.get(i));
        }
    }

    result
}

/// Generate values by unfolding from a seed.
///
/// # JavaScript Example
///
/// ```javascript
/// import { unfold } from 'orlando-transducers';
///
/// // Generate powers of 2
/// const powers = unfold(1, x => {
///   const next = x * 2;
///   return next <= 1000 ? next : null;
/// }, 20);
/// // [2, 4, 8, 16, 32, 64, 128, 256, 512]
/// ```
#[wasm_bindgen]
pub fn unfold(seed: &JsValue, f: &Function, limit: u32) -> Array {
    let result = Array::new();
    let mut current = seed.clone();
    let this = JsValue::null();

    for _ in 0..limit {
        match f.call1(&this, &current) {
            Ok(next) => {
                if next.is_null() || next.is_undefined() {
                    break;
                }
                result.push(&next);
                current = next;
            }
            Err(_) => break,
        }
    }

    result
}

// ============================================================================
// Internal helpers for Phase 5-JS pipeline methods
// ============================================================================

/// Recursively flatten a JsValue (if it's an array) up to a given depth.
fn flatten_value(val: JsValue, depth: usize) -> Vec<JsValue> {
    if depth == 0 {
        return vec![val];
    }

    match val.dyn_into::<Array>() {
        Ok(array) => {
            let mut result = Vec::new();
            for i in 0..array.length() {
                let item = array.get(i);
                result.extend(flatten_value(item, depth - 1));
            }
            result
        }
        Err(original) => vec![original],
    }
}

/// Check if a JsValue is NaN.
fn is_nan(val: &JsValue) -> bool {
    js_sys::Number::is_nan(val)
}

/// Strict equality comparison for JsValue, mirroring JavaScript `===`.
fn js_strict_eq(a: &JsValue, b: &JsValue) -> bool {
    // wasm_bindgen's == uses Object.is() semantics, which is close to ===
    // but differs for +0/-0 and NaN. For our use case (matching spec values
    // which are typically primitives), this is sufficient.
    a == b
}
