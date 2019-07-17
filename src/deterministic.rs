//! Provides functions to sample objects' count and size
//!	using the ALPaCA's deterministic way.
use rand::Rng;
use rand_distr::{Uniform,Distribution};

/// Returns the next multiple of "num" which is greater
/// or equal than "min".
pub fn get_multiple(num: usize, min: usize) -> usize {
	let mut count = num;

	while count < min {
		count += num;
	}

	count
}

/// Returns a vector of target sizes for the fake objects. Sizes have
/// to be a multiple of "obj_size" and smaller than "max_obj_size".
/// They are sampled uniformly.
pub fn get_multiples_in_range<R: Rng>(rng: &mut R,obj_size: usize, max_obj_size: usize, n:usize) -> Result<Vec<usize>, ()> {
	if (obj_size > max_obj_size) || (max_obj_size % obj_size != 0){
		return Err(());
	}

	let mut sizes: Vec<usize> = Vec::with_capacity(n); // Vector of target sizes.

	let max = max_obj_size/obj_size + 1;
	let between = Uniform::from(1..max);

    for _ in 0..n {
        let num: usize = between.sample(rng);
        sizes.push(num*obj_size);
    }

    Ok(sizes)
}
