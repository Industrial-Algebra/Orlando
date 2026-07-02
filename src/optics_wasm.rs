// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! WASM-compatible optics API for JavaScript interop.
//!
//! This module provides JavaScript bindings for functional lenses,
//! allowing type-safe access and updates to nested data structures.

use js_sys::{Function, Object, Reflect};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Type aliases to satisfy clippy type_complexity lint
type JsGetter = Rc<dyn Fn(&JsValue) -> JsValue>;
type JsSetter = Rc<dyn Fn(&JsValue, JsValue) -> JsValue>;
type JsOverAll = Rc<dyn Fn(&JsValue, &Function) -> JsValue>;

/// A Lens provides focused access to a part of a JavaScript object.
///
/// Lenses enable immutable updates to nested data structures in a composable way.
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { lens } from './pkg/orlando.js';
///
/// const user = { name: "Alice", age: 30 };
///
/// const nameLens = lens('name');
/// console.log(nameLens.get(user));           // "Alice"
///
/// const updated = nameLens.set(user, "Bob");
/// console.log(updated.name);                 // "Bob"
/// console.log(user.name);                    // "Alice" (unchanged)
///
/// const upper = nameLens.over(user, s => s.toUpperCase());
/// console.log(upper.name);                   // "ALICE"
/// ```
#[wasm_bindgen]
pub struct JsLens {
    pub(crate) get_fn: JsGetter,
    pub(crate) set_fn: JsSetter,
}

#[wasm_bindgen]
impl JsLens {
    /// Extract the focused value from the source object.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to extract from
    ///
    /// # Returns
    ///
    /// The focused value
    #[wasm_bindgen]
    pub fn get(&self, source: &JsValue) -> JsValue {
        (self.get_fn)(source)
    }

    /// Update the focused value immutably, returning a new object.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to update
    /// * `value` - The new value to set
    ///
    /// # Returns
    ///
    /// A new object with the updated value
    #[wasm_bindgen]
    pub fn set(&self, source: &JsValue, value: JsValue) -> JsValue {
        (self.set_fn)(source, value)
    }

    /// Transform the focused value using a function.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to transform
    /// * `f` - A JavaScript function to apply to the focused value
    ///
    /// # Returns
    ///
    /// A new object with the transformed value
    #[wasm_bindgen]
    pub fn over(&self, source: &JsValue, f: &Function) -> JsValue {
        let current = self.get(source);
        let this = JsValue::null();
        let updated = f.call1(&this, &current).unwrap_or_else(|_| current.clone());
        self.set(source, updated)
    }

    /// Compose two lenses to focus deeper into nested structures.
    ///
    /// # Arguments
    ///
    /// * `other` - The lens to compose with
    ///
    /// # Returns
    ///
    /// A new lens that focuses on the composition
    #[wasm_bindgen]
    pub fn compose(&self, other: &JsLens) -> JsLens {
        let self_get = self.get_fn.clone();
        let self_set = self.set_fn.clone();
        let other_get = other.get_fn.clone();
        let other_set = other.set_fn.clone();

        // Clone for the setter closure
        let self_get_2 = self_get.clone();

        JsLens {
            get_fn: Rc::new(move |source: &JsValue| {
                let intermediate = self_get(source);
                other_get(&intermediate)
            }),
            set_fn: Rc::new(move |source: &JsValue, value: JsValue| {
                let intermediate = self_get_2(source);
                let updated_intermediate = other_set(&intermediate, value);
                self_set(source, updated_intermediate)
            }),
        }
    }
}

/// Create a lens that focuses on a property of an object.
///
/// # Arguments
///
/// * `prop` - The property name to focus on
///
/// # Returns
///
/// A lens that focuses on the specified property
///
/// # Examples
///
/// ```javascript
/// const nameLens = lens('name');
/// const user = { name: "Alice", age: 30 };
/// console.log(nameLens.get(user)); // "Alice"
/// ```
#[wasm_bindgen]
pub fn lens(prop: &str) -> JsLens {
    let prop_get = prop.to_string();
    let prop_set = prop.to_string();

    JsLens {
        get_fn: Rc::new(move |source: &JsValue| {
            if let Some(obj) = source.dyn_ref::<Object>() {
                Reflect::get(obj, &JsValue::from_str(&prop_get)).unwrap_or(JsValue::undefined())
            } else {
                JsValue::undefined()
            }
        }),
        set_fn: Rc::new(move |source: &JsValue, value: JsValue| {
            if let Some(obj) = source.dyn_ref::<Object>() {
                // Clone the object
                let new_obj = Object::assign(&Object::new(), obj);
                // Set the property
                let _ = Reflect::set(&new_obj, &JsValue::from_str(&prop_set), &value);
                new_obj.into()
            } else {
                source.clone()
            }
        }),
    }
}

/// Create a lens that focuses on a nested path in an object.
///
/// # Arguments
///
/// * `path` - Array of property names representing the path
///
/// # Returns
///
/// A lens that focuses on the nested value
///
/// # Examples
///
/// ```javascript
/// const cityLens = lensPath(['address', 'city']);
/// const user = { name: "Alice", address: { city: "NYC", zip: "10001" } };
/// console.log(cityLens.get(user)); // "NYC"
/// ```
#[wasm_bindgen(js_name = lensPath)]
pub fn lens_path(path: &JsValue) -> Result<JsLens, JsValue> {
    let arr = path
        .dyn_ref::<js_sys::Array>()
        .ok_or_else(|| JsValue::from_str("path must be an array"))?;

    if arr.length() == 0 {
        return Err(JsValue::from_str("path cannot be empty"));
    }

    // Start with the first property lens
    let first_prop = arr
        .get(0)
        .as_string()
        .ok_or_else(|| JsValue::from_str("path elements must be strings"))?;

    let mut result = lens(&first_prop);

    // Compose with remaining properties
    for i in 1..arr.length() {
        let prop = arr
            .get(i)
            .as_string()
            .ok_or_else(|| JsValue::from_str("path elements must be strings"))?;
        let next_lens = lens(&prop);
        result = result.compose(&next_lens);
    }

    Ok(result)
}

/// An Optional lens focuses on a property that may not exist.
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { optional } from './pkg/orlando.js';
///
/// const addressLens = optional('address');
/// const user1 = { name: "Alice", address: { city: "NYC" } };
/// const user2 = { name: "Bob" };
///
/// console.log(addressLens.get(user1));  // { city: "NYC" }
/// console.log(addressLens.get(user2));  // undefined
///
/// // over is a no-op when value doesn't exist
/// const updated = addressLens.over(user2, addr => ({ ...addr, city: "LA" }));
/// console.log(updated);  // { name: "Bob" } (unchanged)
/// ```
#[wasm_bindgen]
pub struct JsOptional {
    pub(crate) get_fn: JsGetter,
    pub(crate) set_fn: JsSetter,
}

#[wasm_bindgen]
impl JsOptional {
    /// Extract the focused value, which may be undefined.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to extract from
    ///
    /// # Returns
    ///
    /// The focused value or undefined if it doesn't exist
    #[wasm_bindgen]
    pub fn get(&self, source: &JsValue) -> JsValue {
        (self.get_fn)(source)
    }

    /// Extract the focused value with a default if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to extract from
    /// * `default` - The default value to return if the property doesn't exist
    ///
    /// # Returns
    ///
    /// The focused value or the default
    #[wasm_bindgen(js_name = getOr)]
    pub fn get_or(&self, source: &JsValue, default: JsValue) -> JsValue {
        let value = self.get(source);
        if value.is_undefined() || value.is_null() {
            default
        } else {
            value
        }
    }

    /// Update the focused value immutably if it exists.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to update
    /// * `value` - The new value to set
    ///
    /// # Returns
    ///
    /// A new object with the updated value
    #[wasm_bindgen]
    pub fn set(&self, source: &JsValue, value: JsValue) -> JsValue {
        (self.set_fn)(source, value)
    }

    /// Transform the focused value using a function, only if it exists.
    ///
    /// If the value doesn't exist, returns the source unchanged.
    ///
    /// # Arguments
    ///
    /// * `source` - The JavaScript object to transform
    /// * `f` - A JavaScript function to apply to the focused value
    ///
    /// # Returns
    ///
    /// A new object with the transformed value, or the original if value doesn't exist
    #[wasm_bindgen]
    pub fn over(&self, source: &JsValue, f: &Function) -> JsValue {
        let current = self.get(source);
        if current.is_undefined() || current.is_null() {
            // Value doesn't exist, return source unchanged
            source.clone()
        } else {
            let this = JsValue::null();
            let updated = f.call1(&this, &current).unwrap_or_else(|_| current.clone());
            self.set(source, updated)
        }
    }
}

/// Create an optional lens that focuses on a property that may not exist.
///
/// # Arguments
///
/// * `prop` - The property name to focus on
///
/// # Returns
///
/// An optional lens that focuses on the specified property
///
/// # Examples
///
/// ```javascript
/// const addressLens = optional('address');
/// const user = { name: "Alice" };
/// console.log(addressLens.get(user)); // undefined
/// console.log(addressLens.getOr(user, { city: "Unknown" })); // { city: "Unknown" }
/// ```
#[wasm_bindgen]
pub fn optional(prop: &str) -> JsOptional {
    let prop_get = prop.to_string();
    let prop_set = prop.to_string();

    JsOptional {
        get_fn: Rc::new(move |source: &JsValue| {
            if let Some(obj) = source.dyn_ref::<Object>() {
                Reflect::get(obj, &JsValue::from_str(&prop_get)).unwrap_or(JsValue::undefined())
            } else {
                JsValue::undefined()
            }
        }),
        set_fn: Rc::new(move |source: &JsValue, value: JsValue| {
            if let Some(obj) = source.dyn_ref::<Object>() {
                // Clone the object
                let new_obj = Object::assign(&Object::new(), obj);
                // Set the property
                let _ = Reflect::set(&new_obj, &JsValue::from_str(&prop_set), &value);
                new_obj.into()
            } else {
                source.clone()
            }
        }),
    }
}

/// A Prism focuses on a variant of a sum type (tagged union).
///
/// Prisms are created with a match function (that returns the value or undefined)
/// and a build function (that wraps a value into the sum type).
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { prism } from './pkg/orlando.js';
///
/// // Match/build for a Result-like tagged union
/// const okPrism = prism(
///   obj => obj.tag === 'ok' ? obj.value : undefined,
///   value => ({ tag: 'ok', value })
/// );
///
/// console.log(okPrism.preview({ tag: 'ok', value: 42 }));   // 42
/// console.log(okPrism.preview({ tag: 'err', error: 'x' })); // undefined
/// console.log(okPrism.review(42)); // { tag: 'ok', value: 42 }
/// ```
#[wasm_bindgen]
pub struct JsPrism {
    preview_fn: Rc<dyn Fn(&JsValue) -> JsValue>,
    review_fn: Rc<dyn Fn(JsValue) -> JsValue>,
}

#[wasm_bindgen]
impl JsPrism {
    /// Try to extract the focused variant from the source.
    ///
    /// Returns the value if it matches, or undefined if not.
    #[wasm_bindgen]
    pub fn preview(&self, source: &JsValue) -> JsValue {
        (self.preview_fn)(source)
    }

    /// Construct the sum type from the focused value.
    #[wasm_bindgen]
    pub fn review(&self, value: JsValue) -> JsValue {
        (self.review_fn)(value)
    }

    /// Transform the focused variant using a function, if it matches.
    ///
    /// If the value doesn't match, returns the source unchanged.
    #[wasm_bindgen]
    pub fn over(&self, source: &JsValue, f: &Function) -> JsValue {
        let current = self.preview(source);
        if current.is_undefined() || current.is_null() {
            source.clone()
        } else {
            let this = JsValue::null();
            let updated = f.call1(&this, &current).unwrap_or_else(|_| current.clone());
            self.review(updated)
        }
    }
}

/// Create a prism from match and build functions.
///
/// # Arguments
///
/// * `match_fn` - A function that extracts the variant's value, or returns undefined
/// * `build_fn` - A function that constructs the sum type from a value
///
/// # Examples
///
/// ```javascript
/// const okPrism = prism(
///   obj => obj.tag === 'ok' ? obj.value : undefined,
///   value => ({ tag: 'ok', value })
/// );
/// ```
#[wasm_bindgen]
pub fn prism(match_fn: &Function, build_fn: &Function) -> JsPrism {
    let match_fn = match_fn.clone();
    let build_fn = build_fn.clone();

    JsPrism {
        preview_fn: Rc::new(move |source: &JsValue| {
            let this = JsValue::null();
            match_fn
                .call1(&this, source)
                .unwrap_or(JsValue::undefined())
        }),
        review_fn: Rc::new(move |value: JsValue| {
            let this = JsValue::null();
            build_fn
                .call1(&this, &value)
                .unwrap_or(JsValue::undefined())
        }),
    }
}

/// An Iso represents a lossless bidirectional conversion between two representations.
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { iso } from './pkg/orlando.js';
///
/// const celsiusFahrenheit = iso(
///   c => c * 9/5 + 32,
///   f => (f - 32) * 5/9
/// );
///
/// console.log(celsiusFahrenheit.to(100));   // 212
/// console.log(celsiusFahrenheit.from(32));  // 0
/// ```
#[wasm_bindgen]
pub struct JsIso {
    to_fn: Rc<dyn Fn(&JsValue) -> JsValue>,
    from_fn: Rc<dyn Fn(JsValue) -> JsValue>,
}

#[wasm_bindgen]
impl JsIso {
    /// Convert from source representation to target.
    #[wasm_bindgen]
    pub fn to(&self, source: &JsValue) -> JsValue {
        (self.to_fn)(source)
    }

    /// Convert from target representation to source.
    #[wasm_bindgen]
    pub fn from(&self, value: JsValue) -> JsValue {
        (self.from_fn)(value)
    }

    /// Transform via the isomorphism: convert to target, apply f, convert back.
    #[wasm_bindgen]
    pub fn over(&self, source: &JsValue, f: &Function) -> JsValue {
        let target = self.to(source);
        let this = JsValue::null();
        let updated = f.call1(&this, &target).unwrap_or_else(|_| target.clone());
        self.from(updated)
    }

    /// Return a reversed Iso that swaps the to and from directions.
    #[wasm_bindgen]
    pub fn reverse(&self) -> JsIso {
        let original_to = self.to_fn.clone();
        let original_from = self.from_fn.clone();

        JsIso {
            to_fn: Rc::new(move |a: &JsValue| (original_from)(a.clone())),
            from_fn: Rc::new(move |s: JsValue| (original_to)(&s)),
        }
    }
}

/// Create an isomorphism from to and from functions.
///
/// # Arguments
///
/// * `to_fn` - A function that converts source → target
/// * `from_fn` - A function that converts target → source
#[wasm_bindgen]
pub fn iso(to_fn: &Function, from_fn: &Function) -> JsIso {
    let to_fn = to_fn.clone();
    let from_fn = from_fn.clone();

    JsIso {
        to_fn: Rc::new(move |source: &JsValue| {
            let this = JsValue::null();
            to_fn.call1(&this, source).unwrap_or(JsValue::undefined())
        }),
        from_fn: Rc::new(move |value: JsValue| {
            let this = JsValue::null();
            from_fn.call1(&this, &value).unwrap_or(JsValue::undefined())
        }),
    }
}

/// A Fold extracts zero or more values from a structure (read-only).
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { fold } from './pkg/orlando.js';
///
/// const evensFold = fold(arr => arr.filter(x => x % 2 === 0));
/// console.log(evensFold.foldOf([1, 2, 3, 4, 5, 6])); // [2, 4, 6]
/// ```
#[wasm_bindgen]
pub struct JsFold {
    fold_fn: Rc<dyn Fn(&JsValue) -> JsValue>,
}

#[wasm_bindgen]
impl JsFold {
    /// Extract all focused values from the structure.
    ///
    /// Returns a JavaScript array of the extracted values.
    #[wasm_bindgen(js_name = foldOf)]
    pub fn fold_of(&self, source: &JsValue) -> JsValue {
        (self.fold_fn)(source)
    }

    /// Check if the fold finds any values.
    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self, source: &JsValue) -> bool {
        let result = self.fold_of(source);
        if let Some(arr) = result.dyn_ref::<js_sys::Array>() {
            arr.length() == 0
        } else {
            true
        }
    }

    /// Count the number of focused values.
    #[wasm_bindgen]
    pub fn length(&self, source: &JsValue) -> u32 {
        let result = self.fold_of(source);
        if let Some(arr) = result.dyn_ref::<js_sys::Array>() {
            arr.length()
        } else {
            0
        }
    }

    /// Get the first focused value, or undefined.
    #[wasm_bindgen]
    pub fn first(&self, source: &JsValue) -> JsValue {
        let result = self.fold_of(source);
        if let Some(arr) = result.dyn_ref::<js_sys::Array>() {
            if arr.length() > 0 {
                arr.get(0)
            } else {
                JsValue::undefined()
            }
        } else {
            JsValue::undefined()
        }
    }
}

/// Create a fold from an extraction function.
///
/// # Arguments
///
/// * `extract_fn` - A function that extracts an array of values from a source
#[wasm_bindgen]
pub fn fold(extract_fn: &Function) -> JsFold {
    let extract_fn = extract_fn.clone();

    JsFold {
        fold_fn: Rc::new(move |source: &JsValue| {
            let this = JsValue::null();
            extract_fn
                .call1(&this, source)
                .unwrap_or(JsValue::undefined())
        }),
    }
}

/// A Traversal focuses on zero or more values within a structure,
/// supporting both reading and writing.
///
/// # Examples (in JavaScript)
///
/// ```javascript
/// import { traversal } from './pkg/orlando.js';
///
/// const each = traversal(
///   arr => arr,
///   (arr, f) => arr.map(f)
/// );
///
/// console.log(each.getAll([1, 2, 3]));              // [1, 2, 3]
/// console.log(each.overAll([1, 2, 3], x => x * 2)); // [2, 4, 6]
/// ```
#[wasm_bindgen]
pub struct JsTraversal {
    get_all_fn: Rc<dyn Fn(&JsValue) -> JsValue>,
    over_all_fn: JsOverAll,
}

#[wasm_bindgen]
impl JsTraversal {
    /// Extract all focused values from the structure.
    #[wasm_bindgen(js_name = getAll)]
    pub fn get_all(&self, source: &JsValue) -> JsValue {
        (self.get_all_fn)(source)
    }

    /// Transform all focused values using a function.
    #[wasm_bindgen(js_name = overAll)]
    pub fn over_all(&self, source: &JsValue, f: &Function) -> JsValue {
        (self.over_all_fn)(source, f)
    }

    /// Set all focused values to a single value.
    #[wasm_bindgen(js_name = setAll)]
    pub fn set_all(&self, source: &JsValue, value: JsValue) -> JsValue {
        // Create a JS function that always returns the given value
        let const_fn = Function::new_with_args("_", "return this");

        // Bind the value as `this` so the function returns it
        let bound: Function = const_fn
            .bind1(&value, &JsValue::undefined())
            .unchecked_into();
        (self.over_all_fn)(source, &bound)
    }
}

/// Create a traversal from getAll and overAll functions.
///
/// # Arguments
///
/// * `get_all_fn` - A function that extracts all focused values as an array
/// * `over_all_fn` - A function that transforms all focused values: `(source, transformFn) => newSource`
#[wasm_bindgen]
pub fn traversal(get_all_fn: &Function, over_all_fn: &Function) -> JsTraversal {
    let get_all_fn = get_all_fn.clone();
    let over_all_fn = over_all_fn.clone();

    JsTraversal {
        get_all_fn: Rc::new(move |source: &JsValue| {
            let this = JsValue::null();
            get_all_fn
                .call1(&this, source)
                .unwrap_or(JsValue::undefined())
        }),
        over_all_fn: Rc::new(move |source: &JsValue, f: &Function| {
            let this = JsValue::null();
            over_all_fn
                .call2(&this, source, f)
                .unwrap_or(source.clone())
        }),
    }
}

#[cfg(test)]
mod tests {
    // WASM tests will be in tests/wasm_tests.rs
}
