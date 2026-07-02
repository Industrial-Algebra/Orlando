//! Provenance example: trace which inputs survive a lossy pipeline.
//!
//! Run with: cargo run --example provenance_demo --target x86_64-unknown-linux-gnu
//!
//! Demonstrates Layer B: for lossy pipelines (which have no true inverse),
//! `trace` records which input index produced each output — the "inverse of
//! Filter" is a kept-mask.

use orlando_transducers::collectors::to_vec;
use orlando_transducers::provenance::trace;
use orlando_transducers::transforms::{Filter, Map};
use orlando_transducers::transducer::Transducer;

fn main() {
    // Keep even numbers, then double them.
    let pipeline = Filter::new(|x: &i32| x % 2 == 0).compose(Map::new(|x: i32| x * 2));

    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

    // Run it the ordinary way.
    let outputs = to_vec(&pipeline, data.clone());
    println!("input   : {:?}", data);
    println!("output  : {:?}", outputs);

    // Run it with provenance.
    let (outputs2, trace) = trace(&pipeline, data.clone());
    assert_eq!(outputs, outputs2);

    println!("\nprovenance:");
    println!("  sources (output -> input index): {:?}", trace.sources);
    println!("  kept_mask : {:?}", trace.kept_mask(data.len()));
    println!("  dropped   : {:?}", trace.dropped_indices(data.len()));

    // The kept-mask is the practical "inverse" of Filter: it tells you exactly
    // which inputs survived, information the forward pass destroyed.
    println!("\n✓ kept_mask reconstructs which inputs survived the filter");
}
