//! Reversible pipeline example: build, run, describe, and invert.
//!
//! Run with: cargo run --example inversion_demo --target x86_64-unknown-linux-gnu
//!
//! Demonstrates the invertible groupoid (Layer A): a pipeline built from
//! `IsoMap` stages inverts cleanly, recovering its original input.

use orlando_transducers::collectors::to_vec;
use orlando_transducers::invert::IsoMap;
use orlando_transducers::{Describable, Invertible};
use orlando_transducers::transducer::Transducer;

fn main() {
    // Two-step temperature transform: C -> F, then offset by 10.
    let to_f = IsoMap::new(
        |c: f64| c * 9.0 / 5.0 + 32.0,
        |f: f64| (f - 32.0) * 5.0 / 9.0,
    );
    let offset = IsoMap::new(|x: f64| x + 10.0, |y: f64| y - 10.0);

    let forward = to_f.compose(offset);

    let celsius = vec![0.0, 37.0, 100.0, -40.0];

    // 1. Run it forward.
    let stored = to_vec(&forward, celsius.clone());
    println!("forward describe : {:?}", forward.describe());
    println!("celsius          : {:?}", celsius);
    println!("stored (forward) : {:?}", stored);

    // 2. Invert and recover the original input.
    let inverse = forward.invert();
    let recovered = to_vec(&inverse, stored.clone());
    println!("inverse describe : {:?}", inverse.describe());
    println!("recovered        : {:?}", recovered);

    assert_eq!(recovered, celsius);
    println!("\n✓ round-trip succeeded — groupoid inversion recovered the input");
}
