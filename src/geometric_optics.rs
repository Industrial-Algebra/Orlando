// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! # Geometric Optics: Algebra-Aware Data Access
//!
//! Optics that understand geometric/algebraic structure, operating on
//! flat coefficient arrays (`&[f64]`). These are compatible with any
//! Clifford algebra representation that stores coefficients indexed by
//! basis blade (where blade index = bitset of basis vectors).
//!
//! No dependency on Amari — these work on plain `Vec<f64>` / `&[f64]`.
//! When Amari is available, `From<Multivector>` impls provide seamless
//! conversion.
//!
//! ## Algebra Convention
//!
//! For a Clifford algebra Cl(p,q,r) with n = p+q+r dimensions:
//! - Total dimension: 2^n coefficients
//! - Coefficient index i corresponds to basis blade with basis vectors
//!   determined by the set bits of i
//! - Grade of blade i = popcount(i) (number of set bits)
//!
//! ## Modeled After
//!
//! - ShaperOS `namespace_guard::grade_project` — zeroes coefficients above a grade
//! - ShaperOS `bladefs::BladeIndex::blades_at_grade` — queries blades at a grade
//! - ShaperOS `algebra::compute_grade_mask` — which grades have nonzero coefficients
//!
//! ## Usage
//!
//! ```rust
//! use orlando_transducers::geometric_optics::*;
//!
//! // 3D algebra Cl(3,0,0): 8 coefficients
//! // Index: 0=scalar, 1=e1, 2=e2, 3=e12, 4=e3, 5=e13, 6=e23, 7=e123
//! let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
//!
//! // Grade lens: focus on grade-1 (vector) coefficients
//! let vectors = grade_extract(3, 1, &mv);
//! assert_eq!(vectors, vec![2.0, 3.0, 5.0]); // e1, e2, e3
//!
//! // Grade projection: zero out everything except grade 1
//! let projected = grade_project(3, 1, &mv);
//! assert_eq!(projected, vec![0.0, 2.0, 3.0, 0.0, 5.0, 0.0, 0.0, 0.0]);
//!
//! // Grade mask: which grades are present?
//! let mask = grade_mask(3, &mv);
//! assert_eq!(mask, 0b1111); // grades 0,1,2,3 all present
//! ```

/// Compute the grade (number of set bits) of a basis blade index.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::blade_grade;
///
/// assert_eq!(blade_grade(0), 0);    // scalar
/// assert_eq!(blade_grade(0b001), 1); // e1
/// assert_eq!(blade_grade(0b011), 2); // e12
/// assert_eq!(blade_grade(0b111), 3); // e123
/// ```
#[inline]
#[must_use]
pub const fn blade_grade(blade_index: usize) -> u32 {
    blade_index.count_ones()
}

/// Compute the number of basis blades at a given grade in an n-dimensional algebra.
///
/// This is the binomial coefficient C(n, grade).
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::blades_at_grade_count;
///
/// assert_eq!(blades_at_grade_count(3, 0), 1);  // 1 scalar
/// assert_eq!(blades_at_grade_count(3, 1), 3);  // 3 vectors
/// assert_eq!(blades_at_grade_count(3, 2), 3);  // 3 bivectors
/// assert_eq!(blades_at_grade_count(3, 3), 1);  // 1 trivector
/// ```
#[must_use]
pub fn blades_at_grade_count(dimension: u32, grade: u32) -> usize {
    if grade > dimension {
        return 0;
    }
    binomial(dimension, grade)
}

/// Compute binomial coefficient C(n, k).
fn binomial(n: u32, k: u32) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    // Use the smaller k for efficiency
    let k = k.min(n - k) as usize;
    let mut result: usize = 1;
    for i in 0..k {
        result = result * (n as usize - i) / (i + 1);
    }
    result
}

/// Return the indices of all basis blades at a given grade.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::grade_indices;
///
/// // In 3D: grade-1 blades are at indices 1(e1), 2(e2), 4(e3)
/// assert_eq!(grade_indices(3, 1), vec![1, 2, 4]);
///
/// // Grade-2 (bivectors): indices 3(e12), 5(e13), 6(e23)
/// assert_eq!(grade_indices(3, 2), vec![3, 5, 6]);
/// ```
#[must_use]
pub fn grade_indices(dimension: u32, grade: u32) -> Vec<usize> {
    let total = 1usize << dimension;
    (0..total).filter(|i| blade_grade(*i) == grade).collect()
}

/// Extract coefficients at a specific grade from a multivector coefficient array.
///
/// Returns only the coefficients whose basis blade has the specified grade,
/// in index order.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::grade_extract;
///
/// // 3D multivector: [scalar, e1, e2, e12, e3, e13, e23, e123]
/// let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
///
/// assert_eq!(grade_extract(3, 0, &mv), vec![1.0]);           // scalar
/// assert_eq!(grade_extract(3, 1, &mv), vec![2.0, 3.0, 5.0]); // vectors
/// assert_eq!(grade_extract(3, 2, &mv), vec![4.0, 6.0, 7.0]); // bivectors
/// assert_eq!(grade_extract(3, 3, &mv), vec![8.0]);           // trivector
/// ```
#[must_use]
pub fn grade_extract(dimension: u32, grade: u32, coefficients: &[f64]) -> Vec<f64> {
    grade_indices(dimension, grade)
        .iter()
        .filter_map(|&i| coefficients.get(i).copied())
        .collect()
}

/// Project a multivector to a single grade, zeroing all other grades.
///
/// Returns a new coefficient array of the same length where only
/// coefficients at the specified grade are preserved.
///
/// Modeled after ShaperOS `namespace_guard::grade_project`.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::grade_project;
///
/// let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// let vectors_only = grade_project(3, 1, &mv);
/// assert_eq!(vectors_only, vec![0.0, 2.0, 3.0, 0.0, 5.0, 0.0, 0.0, 0.0]);
/// ```
#[must_use]
pub fn grade_project(dimension: u32, grade: u32, coefficients: &[f64]) -> Vec<f64> {
    let total = 1usize << dimension;
    let mut result = vec![0.0; total.min(coefficients.len())];
    for (i, coeff) in coefficients.iter().enumerate().take(total) {
        if blade_grade(i) == grade {
            result[i] = *coeff;
        }
    }
    result
}

/// Project a multivector to keep only grades up to `max_grade`, zeroing higher grades.
///
/// Modeled after ShaperOS `namespace_guard::grade_project` which restricts
/// visibility to grades 0..=k for a Gr(k,n) namespace.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::grade_project_max;
///
/// let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// let up_to_grade1 = grade_project_max(3, 1, &mv);
/// assert_eq!(up_to_grade1, vec![1.0, 2.0, 3.0, 0.0, 5.0, 0.0, 0.0, 0.0]);
/// ```
#[must_use]
pub fn grade_project_max(dimension: u32, max_grade: u32, coefficients: &[f64]) -> Vec<f64> {
    let total = 1usize << dimension;
    let mut result = coefficients.to_vec();
    result.resize(total, 0.0);
    for (i, coeff) in result.iter_mut().enumerate().take(total) {
        if blade_grade(i) > max_grade {
            *coeff = 0.0;
        }
    }
    result.truncate(coefficients.len());
    result
}

/// Compute the grade mask: a bitmask indicating which grades have nonzero coefficients.
///
/// Bit k is set if any coefficient at grade k is nonzero.
///
/// Modeled after ShaperOS `algebra::compute_grade_mask`.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::grade_mask;
///
/// // All grades present
/// let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// assert_eq!(grade_mask(3, &mv), 0b1111);
///
/// // Only scalar
/// let scalar = vec![5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
/// assert_eq!(grade_mask(3, &scalar), 0b0001);
///
/// // Only bivectors
/// let bivector = vec![0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 0.0];
/// assert_eq!(grade_mask(3, &bivector), 0b0100);
/// ```
#[must_use]
pub fn grade_mask(dimension: u32, coefficients: &[f64]) -> u32 {
    let total = 1usize << dimension;
    let mut mask = 0u32;
    for (i, coeff) in coefficients.iter().enumerate().take(total) {
        if *coeff != 0.0 {
            mask |= 1 << blade_grade(i);
        }
    }
    mask
}

/// Check if a coefficient array has any nonzero coefficients at a given grade.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::has_grade;
///
/// let mv = vec![0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0];
/// assert!(has_grade(3, 1, &mv));  // has vectors
/// assert!(!has_grade(3, 2, &mv)); // no bivectors
/// ```
#[must_use]
pub fn has_grade(dimension: u32, grade: u32, coefficients: &[f64]) -> bool {
    grade_mask(dimension, coefficients) & (1 << grade) != 0
}

/// Check if a coefficient array is a pure k-vector (only one grade is nonzero).
///
/// Modeled after ShaperOS `bladefs::BladeIndex::pure_kvectors`.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::is_pure_grade;
///
/// let vector = vec![0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0];
/// assert!(is_pure_grade(3, &vector));
///
/// let mixed = vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
/// assert!(!is_pure_grade(3, &mixed)); // scalar + vector
/// ```
#[must_use]
pub fn is_pure_grade(dimension: u32, coefficients: &[f64]) -> bool {
    let mask = grade_mask(dimension, coefficients);
    mask.count_ones() <= 1
}

/// Get a single coefficient by basis blade index.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::component_get;
///
/// let mv = vec![1.0, 2.0, 3.0, 4.0];
/// assert_eq!(component_get(&mv, 0), Some(1.0));  // scalar
/// assert_eq!(component_get(&mv, 2), Some(3.0));  // e2
/// assert_eq!(component_get(&mv, 10), None);      // out of bounds
/// ```
#[must_use]
pub fn component_get(coefficients: &[f64], blade_index: usize) -> Option<f64> {
    coefficients.get(blade_index).copied()
}

/// Set a single coefficient by basis blade index, returning a new array.
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::component_set;
///
/// let mv = vec![1.0, 2.0, 3.0, 4.0];
/// let updated = component_set(&mv, 1, 99.0);
/// assert_eq!(updated, vec![1.0, 99.0, 3.0, 4.0]);
/// ```
#[must_use]
pub fn component_set(coefficients: &[f64], blade_index: usize, value: f64) -> Vec<f64> {
    let mut result = coefficients.to_vec();
    if blade_index < result.len() {
        result[blade_index] = value;
    }
    result
}

/// Compute the norm (magnitude) of a coefficient array.
///
/// Uses the Euclidean norm: sqrt(sum of squares of all coefficients).
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::norm;
///
/// let mv = vec![3.0, 4.0];
/// assert!((norm(&mv) - 5.0).abs() < 1e-10);
/// ```
#[must_use]
pub fn norm(coefficients: &[f64]) -> f64 {
    coefficients.iter().map(|c| c * c).sum::<f64>().sqrt()
}

/// Compute the squared norm (avoids sqrt, useful for comparisons).
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::norm_squared;
///
/// let mv = vec![3.0, 4.0];
/// assert!((norm_squared(&mv) - 25.0).abs() < 1e-10);
/// ```
#[must_use]
pub fn norm_squared(coefficients: &[f64]) -> f64 {
    coefficients.iter().map(|c| c * c).sum()
}

/// Normalize a coefficient array to unit magnitude.
///
/// Returns None if the norm is zero (or near-zero).
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::{normalize, norm};
///
/// let mv = vec![3.0, 4.0];
/// let unit = normalize(&mv).unwrap();
/// assert!((norm(&unit) - 1.0).abs() < 1e-10);
/// ```
#[must_use]
pub fn normalize(coefficients: &[f64]) -> Option<Vec<f64>> {
    let n = norm(coefficients);
    if n < f64::EPSILON {
        return None;
    }
    Some(coefficients.iter().map(|c| c / n).collect())
}

/// Compute the grade involution of a coefficient array.
///
/// Grade involution negates odd-grade components:
/// `grade_involution(x) = sum_k (-1)^k <x>_k`
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::grade_involution;
///
/// // [scalar, e1, e2, e12, e3, e13, e23, e123]
/// let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// let inv = grade_involution(3, &mv);
/// // Grades 0,2 unchanged; grades 1,3 negated
/// assert_eq!(inv, vec![1.0, -2.0, -3.0, 4.0, -5.0, 6.0, 7.0, -8.0]);
/// ```
#[must_use]
pub fn grade_involution(dimension: u32, coefficients: &[f64]) -> Vec<f64> {
    let total = 1usize << dimension;
    coefficients
        .iter()
        .enumerate()
        .take(total)
        .map(|(i, c)| if blade_grade(i) % 2 == 1 { -c } else { *c })
        .collect()
}

/// Compute the reverse (reversion) of a coefficient array.
///
/// Reversion negates grades 2,3 (mod 4), i.e., grade k is multiplied by (-1)^(k(k-1)/2).
///
/// # Examples
///
/// ```rust
/// use orlando_transducers::geometric_optics::reverse;
///
/// let mv = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// let rev = reverse(3, &mv);
/// // Grade 0: +1, Grade 1: +1, Grade 2: -1, Grade 3: -1
/// assert_eq!(rev, vec![1.0, 2.0, 3.0, -4.0, 5.0, -6.0, -7.0, -8.0]);
/// ```
#[must_use]
pub fn reverse(dimension: u32, coefficients: &[f64]) -> Vec<f64> {
    let total = 1usize << dimension;
    coefficients
        .iter()
        .enumerate()
        .take(total)
        .map(|(i, c)| {
            let k = blade_grade(i);
            // (-1)^(k(k-1)/2): sign pattern is +,+,-,-,+,+,-,-,...
            if k > 0 && (k * (k - 1) / 2) % 2 == 1 {
                -c
            } else {
                *c
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 3D algebra: 8 coefficients
    // Index: 0=scalar, 1=e1, 2=e2, 3=e12, 4=e3, 5=e13, 6=e23, 7=e123
    fn sample_3d() -> Vec<f64> {
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    }

    // ===== blade_grade =====

    #[test]
    fn test_blade_grade_scalar() {
        assert_eq!(blade_grade(0), 0);
    }

    #[test]
    fn test_blade_grade_vectors() {
        assert_eq!(blade_grade(0b001), 1); // e1
        assert_eq!(blade_grade(0b010), 1); // e2
        assert_eq!(blade_grade(0b100), 1); // e3
    }

    #[test]
    fn test_blade_grade_bivectors() {
        assert_eq!(blade_grade(0b011), 2); // e12
        assert_eq!(blade_grade(0b101), 2); // e13
        assert_eq!(blade_grade(0b110), 2); // e23
    }

    #[test]
    fn test_blade_grade_trivector() {
        assert_eq!(blade_grade(0b111), 3); // e123
    }

    // ===== blades_at_grade_count =====

    #[test]
    fn test_blades_at_grade_count_3d() {
        assert_eq!(blades_at_grade_count(3, 0), 1);
        assert_eq!(blades_at_grade_count(3, 1), 3);
        assert_eq!(blades_at_grade_count(3, 2), 3);
        assert_eq!(blades_at_grade_count(3, 3), 1);
        assert_eq!(blades_at_grade_count(3, 4), 0);
    }

    #[test]
    fn test_blades_at_grade_count_4d() {
        assert_eq!(blades_at_grade_count(4, 0), 1);
        assert_eq!(blades_at_grade_count(4, 1), 4);
        assert_eq!(blades_at_grade_count(4, 2), 6);
        assert_eq!(blades_at_grade_count(4, 3), 4);
        assert_eq!(blades_at_grade_count(4, 4), 1);
    }

    // ===== grade_indices =====

    #[test]
    fn test_grade_indices_3d() {
        assert_eq!(grade_indices(3, 0), vec![0]);
        assert_eq!(grade_indices(3, 1), vec![1, 2, 4]);
        assert_eq!(grade_indices(3, 2), vec![3, 5, 6]);
        assert_eq!(grade_indices(3, 3), vec![7]);
    }

    // ===== grade_extract =====

    #[test]
    fn test_grade_extract_scalar() {
        assert_eq!(grade_extract(3, 0, &sample_3d()), vec![1.0]);
    }

    #[test]
    fn test_grade_extract_vectors() {
        assert_eq!(grade_extract(3, 1, &sample_3d()), vec![2.0, 3.0, 5.0]);
    }

    #[test]
    fn test_grade_extract_bivectors() {
        assert_eq!(grade_extract(3, 2, &sample_3d()), vec![4.0, 6.0, 7.0]);
    }

    #[test]
    fn test_grade_extract_trivector() {
        assert_eq!(grade_extract(3, 3, &sample_3d()), vec![8.0]);
    }

    #[test]
    fn test_grade_extract_empty() {
        assert_eq!(grade_extract(3, 4, &sample_3d()), Vec::<f64>::new());
    }

    // ===== grade_project =====

    #[test]
    fn test_grade_project_vectors_only() {
        let projected = grade_project(3, 1, &sample_3d());
        assert_eq!(projected, vec![0.0, 2.0, 3.0, 0.0, 5.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_grade_project_scalar_only() {
        let projected = grade_project(3, 0, &sample_3d());
        assert_eq!(projected, vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_grade_project_idempotent() {
        // Projecting twice should give the same result
        let once = grade_project(3, 1, &sample_3d());
        let twice = grade_project(3, 1, &once);
        assert_eq!(once, twice);
    }

    // ===== grade_project_max =====

    #[test]
    fn test_grade_project_max() {
        let projected = grade_project_max(3, 1, &sample_3d());
        // Keep grades 0 and 1, zero grades 2 and 3
        assert_eq!(projected, vec![1.0, 2.0, 3.0, 0.0, 5.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_grade_project_max_all() {
        // max_grade >= dimension keeps everything
        let projected = grade_project_max(3, 3, &sample_3d());
        assert_eq!(projected, sample_3d());
    }

    // ===== grade_mask =====

    #[test]
    fn test_grade_mask_all_grades() {
        assert_eq!(grade_mask(3, &sample_3d()), 0b1111);
    }

    #[test]
    fn test_grade_mask_scalar_only() {
        let scalar = vec![5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(grade_mask(3, &scalar), 0b0001);
    }

    #[test]
    fn test_grade_mask_bivector_only() {
        let bv = vec![0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 0.0];
        assert_eq!(grade_mask(3, &bv), 0b0100);
    }

    #[test]
    fn test_grade_mask_zero() {
        let zero = vec![0.0; 8];
        assert_eq!(grade_mask(3, &zero), 0);
    }

    // ===== has_grade =====

    #[test]
    fn test_has_grade() {
        let vectors = vec![0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0];
        assert!(!has_grade(3, 0, &vectors));
        assert!(has_grade(3, 1, &vectors));
        assert!(!has_grade(3, 2, &vectors));
    }

    // ===== is_pure_grade =====

    #[test]
    fn test_is_pure_grade_vector() {
        let vector = vec![0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0];
        assert!(is_pure_grade(3, &vector));
    }

    #[test]
    fn test_is_pure_grade_mixed() {
        assert!(!is_pure_grade(3, &sample_3d()));
    }

    #[test]
    fn test_is_pure_grade_zero() {
        let zero = vec![0.0; 8];
        assert!(is_pure_grade(3, &zero));
    }

    // ===== component_get/set =====

    #[test]
    fn test_component_get() {
        let mv = sample_3d();
        assert_eq!(component_get(&mv, 0), Some(1.0));
        assert_eq!(component_get(&mv, 3), Some(4.0));
        assert_eq!(component_get(&mv, 10), None);
    }

    #[test]
    fn test_component_set() {
        let mv = sample_3d();
        let updated = component_set(&mv, 1, 99.0);
        assert_eq!(updated[1], 99.0);
        assert_eq!(updated[0], 1.0); // other coefficients unchanged
        assert_eq!(updated.len(), mv.len());
    }

    // ===== norm =====

    #[test]
    fn test_norm_345() {
        assert!((norm(&[3.0, 4.0]) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_norm_zero() {
        assert!((norm(&[0.0, 0.0]) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_norm_squared() {
        assert!((norm_squared(&[3.0, 4.0]) - 25.0).abs() < 1e-10);
    }

    // ===== normalize =====

    #[test]
    fn test_normalize() {
        let unit = normalize(&[3.0, 4.0]).unwrap();
        assert!((norm(&unit) - 1.0).abs() < 1e-10);
        assert!((unit[0] - 0.6).abs() < 1e-10);
        assert!((unit[1] - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_zero() {
        assert!(normalize(&[0.0, 0.0]).is_none());
    }

    // ===== grade_involution =====

    #[test]
    fn test_grade_involution() {
        let inv = grade_involution(3, &sample_3d());
        // grade 0 (idx 0): +1.0
        // grade 1 (idx 1,2,4): -2.0, -3.0, -5.0
        // grade 2 (idx 3,5,6): +4.0, +6.0, +7.0
        // grade 3 (idx 7): -8.0
        assert_eq!(inv, vec![1.0, -2.0, -3.0, 4.0, -5.0, 6.0, 7.0, -8.0]);
    }

    #[test]
    fn test_grade_involution_involution() {
        // Applying twice should return the original
        let mv = sample_3d();
        let double = grade_involution(3, &grade_involution(3, &mv));
        for (a, b) in mv.iter().zip(double.iter()) {
            assert!((a - b).abs() < 1e-10);
        }
    }

    // ===== reverse =====

    #[test]
    fn test_reverse() {
        let rev = reverse(3, &sample_3d());
        // Grade 0: sign +1, Grade 1: sign +1, Grade 2: sign -1, Grade 3: sign -1
        assert_eq!(rev, vec![1.0, 2.0, 3.0, -4.0, 5.0, -6.0, -7.0, -8.0]);
    }

    #[test]
    fn test_reverse_involution() {
        // Applying reverse twice should return the original
        let mv = sample_3d();
        let double = reverse(3, &reverse(3, &mv));
        for (a, b) in mv.iter().zip(double.iter()) {
            assert!((a - b).abs() < 1e-10);
        }
    }

    // ===== Optic-like composition tests =====

    #[test]
    fn test_extract_then_norm() {
        // Pipeline pattern: extract bivector part, then compute norm
        let mv = sample_3d();
        let bivectors = grade_extract(3, 2, &mv);
        let bv_norm = norm(&bivectors);
        // bivectors are [4.0, 6.0, 7.0], norm = sqrt(16+36+49) = sqrt(101)
        assert!((bv_norm - 101.0f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_project_preserves_grade_mask() {
        let mv = sample_3d();
        let projected = grade_project(3, 2, &mv);
        // Only grade 2 should remain
        assert_eq!(grade_mask(3, &projected), 0b0100);
    }

    #[test]
    fn test_grade_filter_pattern() {
        // Pattern: filter a collection of multivectors to only those with nonzero bivector part
        let mvs = [
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // scalar only
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0], // has bivector
            vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // vector only
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0], // has bivector
        ];

        let with_bivectors: Vec<_> = mvs.iter().filter(|mv| has_grade(3, 2, mv)).collect();

        assert_eq!(with_bivectors.len(), 2);
    }

    // ===== Property: binomial sums to 2^n =====

    #[test]
    fn test_binomial_sum() {
        for n in 0..8 {
            let sum: usize = (0..=n).map(|k| blades_at_grade_count(n, k)).sum();
            assert_eq!(sum, 1 << n);
        }
    }

    // ===== 2D algebra tests =====

    #[test]
    fn test_2d_algebra() {
        // Cl(2,0,0): 4 coefficients [scalar, e1, e2, e12]
        let mv = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(grade_indices(2, 0), vec![0]);
        assert_eq!(grade_indices(2, 1), vec![1, 2]);
        assert_eq!(grade_indices(2, 2), vec![3]);
        assert_eq!(grade_extract(2, 1, &mv), vec![2.0, 3.0]);
    }

    // ===== 4D algebra tests =====

    #[test]
    fn test_4d_grade_indices() {
        // Grade 2 in 4D: C(4,2) = 6 bivectors
        let indices = grade_indices(4, 2);
        assert_eq!(indices.len(), 6);
        // All should have exactly 2 bits set
        for idx in &indices {
            assert_eq!(blade_grade(*idx), 2);
        }
    }

    // Property tests
    #[cfg(not(target_arch = "wasm32"))]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        fn arbitrary_mv(dim: u32) -> impl Strategy<Value = Vec<f64>> {
            let size = 1usize << dim;
            proptest::collection::vec(-100.0f64..100.0, size)
        }

        proptest! {
            /// grade_project is idempotent
            #[test]
            fn prop_grade_project_idempotent(mv in arbitrary_mv(3), grade in 0u32..4) {
                let once = grade_project(3, grade, &mv);
                let twice = grade_project(3, grade, &once);
                for (a, b) in once.iter().zip(twice.iter()) {
                    prop_assert!((a - b).abs() < 1e-10);
                }
            }

            /// grade_extract length equals blades_at_grade_count
            #[test]
            fn prop_grade_extract_length(mv in arbitrary_mv(3), grade in 0u32..4) {
                let extracted = grade_extract(3, grade, &mv);
                prop_assert_eq!(extracted.len(), blades_at_grade_count(3, grade));
            }

            /// grade_mask bits correspond to non-empty grade_extract
            #[test]
            fn prop_grade_mask_consistent(mv in arbitrary_mv(3)) {
                let mask = grade_mask(3, &mv);
                for grade in 0u32..4 {
                    let extracted = grade_extract(3, grade, &mv);
                    let has_nonzero = extracted.iter().any(|c| *c != 0.0);
                    prop_assert_eq!(
                        mask & (1 << grade) != 0,
                        has_nonzero,
                        "grade {} mask mismatch", grade
                    );
                }
            }

            /// grade_involution is an involution (applying twice = identity)
            #[test]
            fn prop_grade_involution_involution(mv in arbitrary_mv(3)) {
                let double = grade_involution(3, &grade_involution(3, &mv));
                for (a, b) in mv.iter().zip(double.iter()) {
                    prop_assert!((a - b).abs() < 1e-10);
                }
            }

            /// reverse is an involution
            #[test]
            fn prop_reverse_involution(mv in arbitrary_mv(3)) {
                let double = reverse(3, &reverse(3, &mv));
                for (a, b) in mv.iter().zip(double.iter()) {
                    prop_assert!((a - b).abs() < 1e-10);
                }
            }

            /// norm is non-negative
            #[test]
            fn prop_norm_non_negative(mv in arbitrary_mv(3)) {
                prop_assert!(norm(&mv) >= 0.0);
            }

            /// normalize produces unit norm (when not zero)
            #[test]
            fn prop_normalize_unit(mv in arbitrary_mv(3)) {
                if let Some(unit) = normalize(&mv) {
                    prop_assert!((norm(&unit) - 1.0).abs() < 1e-8);
                }
            }
        }
    }
}
