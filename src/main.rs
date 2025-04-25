mod vector;
mod matrix;

use std::io;
use std::cmp::Ordering; // compare, ordering
use rand::Rng;
use vector::Vector;

fn main() {
    println!("input values to construct vector");
    
    let mut input = String::new();
    
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let vals: Vec<f32> = input
        .split_whitespace()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();

    println!("input vector {:?}", vals);

    let input_vector = Vector::new_from_vec(vals);
    
    println!("parsed input vector: {:?}", input_vector);
}