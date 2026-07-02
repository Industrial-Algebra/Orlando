// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! WASM bindings for geometric optics operating on Clifford algebra coefficient arrays.
//!
//! These functions expose Orlando's geometric optics to JavaScript via WebAssembly.
//! All operations work on flat `Float64Array` coefficient arrays where index `i`
//! corresponds to the basis blade whose basis vectors are the set bits of `i`.

use crate::geometric_optics;
use js_sys::Float64Array;
use wasm_bindgen::prelude::*;

/// Convert a JS Float64Array to a Rust Vec<f64>.
fn to_vec(arr: &Float64Array) -> Vec<f64> {
    arr.to_vec()
}

/// Convert a Rust Vec<f64> to a JS Float64Array.
fn to_float64_array(v: &[f64]) -> Float64Array {
    let arr = Float64Array::new_with_length(v.len() as u32);
    arr.copy_from(v);
    arr
}

/// Compute the grade (number of basis vectors) of a basis blade index.
///
/// ```javascript
/// import { bladeGrade } from './pkg/orlando.js';
/// bladeGrade(0);     // 0 (scalar)
/// bladeGrade(0b011); // 2 (bivector e12)
/// ```
#[wasm_bindgen(js_name = "bladeGrade")]
pub fn blade_grade(blade_index: usize) -> u32 {
    geometric_optics::blade_grade(blade_index)
}

/// Count how many basis blades exist at a given grade in an n-dimensional algebra.
///
/// ```javascript
/// import { bladesAtGradeCount } from './pkg/orlando.js';
/// bladesAtGradeCount(3, 1); // 3 (three vectors in 3D)
/// bladesAtGradeCount(3, 2); // 3 (three bivectors in 3D)
/// ```
#[wasm_bindgen(js_name = "bladesAtGradeCount")]
pub fn blades_at_grade_count(dimension: u32, grade: u32) -> usize {
    geometric_optics::blades_at_grade_count(dimension, grade)
}

/// Return the coefficient array indices of all basis blades at a given grade.
///
/// ```javascript
/// import { gradeIndices } from './pkg/orlando.js';
/// gradeIndices(3, 1); // [1, 2, 4] (e1, e2, e3)
/// gradeIndices(3, 2); // [3, 5, 6] (e12, e13, e23)
/// ```
#[wasm_bindgen(js_name = "gradeIndices")]
pub fn grade_indices(dimension: u32, grade: u32) -> Vec<usize> {
    geometric_optics::grade_indices(dimension, grade)
}

/// Extract coefficients at a specific grade from a multivector coefficient array.
///
/// ```javascript
/// import { gradeExtract } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4, 5, 6, 7, 8]);
/// gradeExtract(3, 1, mv); // Float64Array [2, 3, 5] (vectors)
/// gradeExtract(3, 2, mv); // Float64Array [4, 6, 7] (bivectors)
/// ```
#[wasm_bindgen(js_name = "gradeExtract")]
pub fn grade_extract(dimension: u32, grade: u32, coefficients: &Float64Array) -> Float64Array {
    let v = to_vec(coefficients);
    to_float64_array(&geometric_optics::grade_extract(dimension, grade, &v))
}

/// Project a multivector to a single grade, zeroing all other grades.
///
/// ```javascript
/// import { gradeProject } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4, 5, 6, 7, 8]);
/// gradeProject(3, 1, mv); // Float64Array [0, 2, 3, 0, 5, 0, 0, 0]
/// ```
#[wasm_bindgen(js_name = "gradeProject")]
pub fn grade_project(dimension: u32, grade: u32, coefficients: &Float64Array) -> Float64Array {
    let v = to_vec(coefficients);
    to_float64_array(&geometric_optics::grade_project(dimension, grade, &v))
}

/// Project a multivector to keep only grades up to max_grade.
///
/// ```javascript
/// import { gradeProjectMax } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4, 5, 6, 7, 8]);
/// gradeProjectMax(3, 1, mv); // Float64Array [1, 2, 3, 0, 5, 0, 0, 0]
/// ```
#[wasm_bindgen(js_name = "gradeProjectMax")]
pub fn grade_project_max(
    dimension: u32,
    max_grade: u32,
    coefficients: &Float64Array,
) -> Float64Array {
    let v = to_vec(coefficients);
    to_float64_array(&geometric_optics::grade_project_max(
        dimension, max_grade, &v,
    ))
}

/// Compute a bitmask of which grades have nonzero coefficients.
///
/// ```javascript
/// import { gradeMask } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4, 5, 6, 7, 8]);
/// gradeMask(3, mv); // 0b1111 = 15 (all grades present)
/// ```
#[wasm_bindgen(js_name = "gradeMask")]
pub fn grade_mask(dimension: u32, coefficients: &Float64Array) -> u32 {
    let v = to_vec(coefficients);
    geometric_optics::grade_mask(dimension, &v)
}

/// Check if a coefficient array has any nonzero coefficients at a given grade.
///
/// ```javascript
/// import { hasGrade } from './pkg/orlando.js';
/// const mv = new Float64Array([0, 1, 2, 0, 3, 0, 0, 0]);
/// hasGrade(3, 1, mv); // true (has vectors)
/// hasGrade(3, 2, mv); // false (no bivectors)
/// ```
#[wasm_bindgen(js_name = "hasGrade")]
pub fn has_grade(dimension: u32, grade: u32, coefficients: &Float64Array) -> bool {
    let v = to_vec(coefficients);
    geometric_optics::has_grade(dimension, grade, &v)
}

/// Check if a coefficient array is a pure k-vector (only one grade is nonzero).
///
/// ```javascript
/// import { isPureGrade } from './pkg/orlando.js';
/// isPureGrade(3, new Float64Array([0, 1, 2, 0, 3, 0, 0, 0])); // true (pure vector)
/// isPureGrade(3, new Float64Array([1, 1, 0, 0, 0, 0, 0, 0])); // false (scalar + vector)
/// ```
#[wasm_bindgen(js_name = "isPureGrade")]
pub fn is_pure_grade(dimension: u32, coefficients: &Float64Array) -> bool {
    let v = to_vec(coefficients);
    geometric_optics::is_pure_grade(dimension, &v)
}

/// Get a single coefficient by basis blade index.
///
/// ```javascript
/// import { componentGet } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4]);
/// componentGet(mv, 0); // 1.0 (scalar)
/// componentGet(mv, 2); // 3.0 (e2)
/// ```
#[wasm_bindgen(js_name = "componentGet")]
pub fn component_get(coefficients: &Float64Array, blade_index: usize) -> Option<f64> {
    let v = to_vec(coefficients);
    geometric_optics::component_get(&v, blade_index)
}

/// Set a single coefficient by basis blade index, returning a new array.
///
/// ```javascript
/// import { componentSet } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4]);
/// componentSet(mv, 1, 99); // Float64Array [1, 99, 3, 4]
/// ```
#[wasm_bindgen(js_name = "componentSet")]
pub fn component_set(coefficients: &Float64Array, blade_index: usize, value: f64) -> Float64Array {
    let v = to_vec(coefficients);
    to_float64_array(&geometric_optics::component_set(&v, blade_index, value))
}

/// Compute the Euclidean norm (magnitude) of a coefficient array.
///
/// ```javascript
/// import { mvNorm } from './pkg/orlando.js';
/// mvNorm(new Float64Array([3, 4])); // 5.0
/// ```
#[wasm_bindgen(js_name = "mvNorm")]
pub fn mv_norm(coefficients: &Float64Array) -> f64 {
    let v = to_vec(coefficients);
    geometric_optics::norm(&v)
}

/// Compute the squared norm (avoids sqrt, useful for comparisons).
///
/// ```javascript
/// import { mvNormSquared } from './pkg/orlando.js';
/// mvNormSquared(new Float64Array([3, 4])); // 25.0
/// ```
#[wasm_bindgen(js_name = "mvNormSquared")]
pub fn mv_norm_squared(coefficients: &Float64Array) -> f64 {
    let v = to_vec(coefficients);
    geometric_optics::norm_squared(&v)
}

/// Normalize a coefficient array to unit magnitude.
/// Returns null if the norm is zero.
///
/// ```javascript
/// import { mvNormalize } from './pkg/orlando.js';
/// mvNormalize(new Float64Array([3, 4])); // Float64Array [0.6, 0.8]
/// ```
#[wasm_bindgen(js_name = "mvNormalize")]
pub fn mv_normalize(coefficients: &Float64Array) -> Option<Float64Array> {
    let v = to_vec(coefficients);
    geometric_optics::normalize(&v).map(|n| to_float64_array(&n))
}

/// Compute the grade involution (negates odd-grade components).
///
/// ```javascript
/// import { gradeInvolution } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4, 5, 6, 7, 8]);
/// gradeInvolution(3, mv); // Float64Array [1, -2, -3, 4, -5, 6, 7, -8]
/// ```
#[wasm_bindgen(js_name = "gradeInvolution")]
pub fn grade_involution(dimension: u32, coefficients: &Float64Array) -> Float64Array {
    let v = to_vec(coefficients);
    to_float64_array(&geometric_optics::grade_involution(dimension, &v))
}

/// Compute the reversion of a coefficient array.
/// Negates grades 2 and 3 (mod 4).
///
/// ```javascript
/// import { mvReverse } from './pkg/orlando.js';
/// const mv = new Float64Array([1, 2, 3, 4, 5, 6, 7, 8]);
/// mvReverse(3, mv); // Float64Array [1, 2, 3, -4, 5, -6, -7, -8]
/// ```
#[wasm_bindgen(js_name = "mvReverse")]
pub fn mv_reverse(dimension: u32, coefficients: &Float64Array) -> Float64Array {
    let v = to_vec(coefficients);
    to_float64_array(&geometric_optics::reverse(dimension, &v))
}
