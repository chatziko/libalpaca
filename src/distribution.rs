//! Provides functions to sample objects' count and size from a
//! probability distribution.
use std::{str, fs};
use rand::Rng;
use rand_distr;
use rand_distr::Distribution;
use aux::*;

// Number of tries per sample. If no sampled number satisfies a specified
// threshold after `SAMPLE_LIMIT` tries the sampling function returns Err.
const SAMPLE_LIMIT: usize = 30;

// Probability distribution
pub struct Dist {
    pub name: String,
    pub params: Vec<f64>,           // For known distributions these are the params (eg mean, lambda, etc). For custom, these are the probabilities
    pub values: Option<Vec<usize>>,   // Only for custom, the values
}

/// A struct containing the 3 distributions needed for the probabilistic version.
pub struct Distributions {
    pub html: Dist,         // html dist
    pub obj_num: Dist,      // number of objects
    pub obj_size: Dist,     // size of object
}

impl Distributions {
    /// Construct a Distributions object.
    pub fn from(
        dist_html: &str,
        dist_obj_num: &str,
        dist_obj_size: &str,
    ) -> Result<Distributions, String> {
        // Parse html size distribution.
        return Ok(Distributions {
            html: parse_given_dist(dist_html)?,
            obj_num: parse_given_dist(dist_obj_num)?,
            obj_size: parse_given_dist(dist_obj_size)?,
        })
    }
}

/// Parses a given distribution and decides if its a known one or a file. Updates
/// the Distribtuions object accordingly.
fn parse_given_dist(dist: &str) -> Result<Dist,String> {

    if dist.ends_with(".dist") {
        // A distribution file has been given

        let res = stringify_error(fs::read_to_string(dist.clone()));
        if res.is_err() {
            eprint!("libalpaca: cannot open {}: \n", dist);
        }
        let data = res?;

        // Construct the 2 vectors containing the values and probabilities
        let mut values: Vec<usize> = Vec::new();
        let mut probs: Vec<f64> = Vec::new();
        for line in data.lines() {
            let l = String::from(line);
            let v:Vec<&str> = l.split_whitespace().collect();
            if v.len() != 2 {
                return Err(format!("invalid dist file {}, line {}", dist, line));
            }
            values.push(v[0].parse().unwrap());
            probs.push(v[1].parse().unwrap());
        }

        return Ok(Dist {
            name: String::from("custom"),
            params: probs,
            values: Some(values),
        });

    } else {

        let tokens: Vec<&str> = dist.split("/").collect();
        if tokens.len() != 2 {
            return Err(format!("invalid distribution {}", dist));
        }

        let name = tokens[0];
        let params: Vec<f64> = tokens[1].split(",").map(|s| s.parse().unwrap()).collect(); // Distributions parameters

        let params_needed = match name {
            "Normal" => 2,
            "LogNormal" => 2,
            "Exp" => 1,
            "Poisson" => 1,
            "Binomial" => 2,
            "Gamma" => 2,
            _ => return Err(format!("invalid distribution {}", dist)),
        };

        // A known distribution and its parameters have been given.
        if params.len() != params_needed {
            return Err(format!("{} distribution requires {} params, {} given", name, params_needed, params.len()));
        }

        return Ok(Dist {
            name: String::from(name),
            params: params,
            values: None,
        });
    }
}

pub fn sample_ge_many(dist:&Dist, lower_bound:usize, samples:usize) -> Result<Vec<usize>,String> {
    let mut vec: Vec<usize> = Vec::new();
    for _ in 0..samples {
        vec.push(sample_ge(dist, lower_bound)?);
    }
    Ok(vec)
}

/// Samples a value greater or equal than the given one
pub fn sample_ge(dist:&Dist, lower_bound:usize) -> Result<usize,String> {
    for _ in 0..SAMPLE_LIMIT {
        let sampled_num = sample(dist);
        if sampled_num >= lower_bound {
            return Ok(sampled_num);
        }
    }
    Err(format!("SAMPLE_LIMIT={} reached for distribution {}", SAMPLE_LIMIT, dist.name))
}

fn sample(dist:&Dist) -> usize {

   match dist.name.as_str() {
        "Normal" => {
            let d = rand_distr::Normal::new(dist.params[0], dist.params[1]).unwrap();
            d.sample(&mut rand::thread_rng()) as usize
        },
        "LogNormal" => {
            let d = rand_distr::LogNormal::new(dist.params[0], dist.params[1]).unwrap();
            d.sample(&mut rand::thread_rng()) as usize
        },
        "Exp" => {
            let d = rand_distr::Exp::new(dist.params[0]).unwrap();
            d.sample(&mut rand::thread_rng()) as usize
        },
        // "Poisson" => {
        //     let d = Poisson::new(dist.params[0]).unwrap();
        //     return Ok(d.sample(&mut rand::thread_rng()) as usize);
        // },
        "Binomial" => {
            let d = rand_distr::Binomial::new(dist.params[0] as u64, dist.params[1]).unwrap();
            d.sample(&mut rand::thread_rng()) as usize
        },
        "Gamma" => {
            let d = rand_distr::Gamma::new(dist.params[0], dist.params[1]).unwrap();
            d.sample(&mut rand::thread_rng()) as usize
        },
        "custom" => {
            let probability: f64 = rand::thread_rng().sample(rand_distr::OpenClosed01);
            let mut sum = 0.0;
            let values = dist.values.as_ref().unwrap();
            let mut sampled_num = values[values.len() - 1];

            // Sample a value from the given distribution
            for i in 0..values.len() {
                sum += dist.params[i];
                if sum >= probability {
                    sampled_num = values[i];
                    break;
                }
            }
            sampled_num
        },
        _ => panic!("not possible"),
    }
}

// Samples the html target size.
// pub fn sample_html_size<R: Rng>(
//     rng: &mut R,
//     dists: &Distributions,
//     ge: usize,
// ) -> Result<usize, ()> {
//     // Decide if a known distribution or a file was given.
//     if !dists.dist_html.is_none() {
//         // Known distribution.
//         let dist;
//         match dists.dist_html {
//             Some(ref r) => dist = r,
//             None => return Err(()),
//         }
//         match sample_from_distribution(rng, dist, ge, 1) {
//             Ok(sampled_nums) => return Ok(sampled_nums[0]),
//             Err(_) => return Err(()),
//         }
//     } else if !dists.dist_html_values.is_none() && !dists.dist_html_probs.is_none() {
//         // File.
//         let values;
//         match dists.dist_html_values {
//             Some(ref r) => values = r,
//             None => return Err(()),
//         }
//         let probs;
//         match dists.dist_html_probs {
//             Some(ref r) => probs = r,
//             None => return Err(()),
//         }
//         match sample_from_file(rng, values, probs, ge, 1) {
//             Ok(sampled_nums) => return Ok(sampled_nums[0]),
//             Err(_) => return Err(()),
//         }
//     } else {
//         return Err(());
//     }
// }

// /// Samples the number of objects.
// pub fn sample_object_num<R: Rng>(
//     rng: &mut R,
//     dists: &Distributions,
//     ge: usize,
// ) -> Result<usize, ()> {
//     // Decide if a known distribution or a file was given.
//     if !dists.dist_obj_num.is_none() {
//         // Known distribution.
//         let dist;
//         match dists.dist_obj_num {
//             Some(ref r) => dist = r,
//             None => return Err(()),
//         }
//         match sample_from_distribution(rng, dist, ge, 1) {
//             Ok(sampled_nums) => return Ok(sampled_nums[0]),
//             Err(_) => return Err(()),
//         }
//     } else if !dists.dist_obj_num_values.is_none() && !dists.dist_obj_num_probs.is_none() {
//         // File.
//         let values;
//         match dists.dist_obj_num_values {
//             Some(ref r) => values = r,
//             None => return Err(()),
//         }
//         let probs;
//         match dists.dist_obj_num_probs {
//             Some(ref r) => probs = r,
//             None => return Err(()),
//         }
//         match sample_from_file(rng, values, probs, ge, 1) {
//             Ok(sampled_nums) => return Ok(sampled_nums[0]),
//             Err(_) => return Err(()),
//         }
//     } else {
//         return Err(());
//     }
// }

// /// Samples the objects' sizes..
// pub fn sample_object_sizes<R: Rng>(
//     rng: &mut R,
//     dists: &Distributions,
//     n: usize,
// ) -> Result<Vec<usize>, ()> {
//     // Decide if a known distribution or a file was given.
//     if !dists.dist_obj_size.is_none() {
//         // Known distribution.
//         let dist;
//         match dists.dist_obj_size {
//             Some(ref r) => dist = r,
//             None => return Err(()),
//         }
//         match sample_from_distribution(rng, dist, 1, n) {
//             Ok(sampled_nums) => return Ok(sampled_nums),
//             Err(_) => return Err(()),
//         }
//     } else if !dists.dist_obj_size_values.is_none() && !dists.dist_obj_size_probs.is_none() {
//         // File.
//         let values;
//         match dists.dist_obj_size_values {
//             Some(ref r) => values = r,
//             None => return Err(()),
//         }
//         let probs;
//         match dists.dist_obj_size_probs {
//             Some(ref r) => probs = r,
//             None => return Err(()),
//         }
//         match sample_from_file(rng, values, probs, 1, n) {
//             Ok(sampled_nums) => return Ok(sampled_nums),
//             Err(_) => return Err(()),
//         }
//     } else {
//         return Err(());
//     }
// }

// /// This function samples values from a given distribution
// fn sample_from_distribution<R: Rng>(
//     rng: &mut R,
//     dist: &str,
//     ge: usize,
//     n: usize,
// ) -> Result<Vec<usize>, ()> {
//     let tokens: Vec<&str> = dist.split("/").collect();
//     if tokens.len() != 2 {
//         return Err(());
//     }

//     let dist_kind = tokens[0]; // Name of the distribution
//     let dist_params: Vec<f64> = tokens[1].split(",").map(|s| s.parse().unwrap()).collect(); // Distributions parameters
//     let mut sampled_nums: Vec<usize> = Vec::with_capacity(n); // Vector with sampled numbers
//     let mut sampled;

//     if dist_kind == "Normal" {
//         // Normal Distribution
//         if dist_params.len() != 2 {
//             return Err(());
//         }
//         let dist;
//         match Normal::new(dist_params[0], dist_params[1]) {
//             Ok(d) => dist = d,
//             Err(_) => return Err(()),
//         }
//         // Sample n numbers
//         for _ in 0..n {
//             sampled = false;
//             for _ in 0..SAMPLE_LIMIT {
//                 let sampled_num = dist.sample(rng) as usize;
//                 if sampled_num >= ge {
//                     sampled_nums.push(sampled_num);
//                     sampled = true;
//                     break;
//                 }
//             }
//             if sampled == false {
//                 // No number was found for an iteration
//                 eprint!(
//                     "libalpaca: sample_from_distribution: {}: SAMLPE_LIMIT={} reached\n",
//                     dist_kind,
//                     SAMPLE_LIMIT
//                 );
//                 return Err(());
//             }
//         }
//     } else if dist_kind == "LogNormal" {
//         // LogNormal Distribution
//         if dist_params.len() != 2 {
//             return Err(());
//         }
//         let dist;
//         match LogNormal::new(dist_params[0], dist_params[1]) {
//             Ok(d) => dist = d,
//             Err(_) => return Err(()),
//         }
//         // Sample n numbers
//         for _ in 0..n {
//             sampled = false;
//             for _ in 0..SAMPLE_LIMIT {
//                 let sampled_num = dist.sample(rng) as usize;
//                 if sampled_num >= ge {
//                     sampled_nums.push(sampled_num);
//                     sampled = true;
//                     break;
//                 }
//             }
//             if sampled == false {
//                 // No number was found for an iteration
//                 eprint!(
//                     "libalpaca: sample_from_distribution: {}: SAMLPE_LIMIT={} reached\n",
//                     dist_kind,
//                     SAMPLE_LIMIT
//                 );
//                 return Err(());
//             }
//         }
//     } else if dist_kind == "Exp" {
//         // Exponential Distribution
//         if dist_params.len() != 1 {
//             return Err(());
//         }
//         let dist;
//         match Exp::new(dist_params[0]) {
//             Ok(d) => dist = d,
//             Err(_) => return Err(()),
//         }
//         // Sample n numbers
//         for _ in 0..n {
//             sampled = false;
//             for _ in 0..SAMPLE_LIMIT {
//                 let sampled_num = dist.sample(rng) as usize;
//                 if sampled_num >= ge {
//                     sampled_nums.push(sampled_num);
//                     sampled = true;
//                     break;
//                 }
//             }
//             if sampled == false {
//                 // No number was found for an iteration
//                 eprint!(
//                     "libalpaca: sample_from_distribution: {}: SAMLPE_LIMIT={} reached\n",
//                     dist_kind,
//                     SAMPLE_LIMIT
//                 );
//                 return Err(());
//             }
//         }
//     } else if dist_kind == "Poisson" {
//         // Poisson Distribution
//         if dist_params.len() != 1 {
//             return Err(());
//         }
//         let dist;
//         match Poisson::new(dist_params[0]) {
//             Ok(d) => dist = d,
//             Err(_) => return Err(()),
//         }
//         // Sample n numbers
//         for _ in 0..n {
//             sampled = false;
//             for _ in 0..SAMPLE_LIMIT {
//                 let sampled_num: u64 = dist.sample(rng);
//                 if sampled_num as usize >= ge {
//                     sampled_nums.push(sampled_num as usize);
//                     sampled = true;
//                     break;
//                 }
//             }
//             if sampled == false {
//                 // No number was found for an iteration
//                 eprint!(
//                     "libalpaca: sample_from_distribution: {}: SAMLPE_LIMIT={} reached\n",
//                     dist_kind,
//                     SAMPLE_LIMIT
//                 );
//                 return Err(());
//             }
//         }
//     } else if dist_kind == "Binomial" {
//         // Binomial Distribution
//         if dist_params.len() != 2 {
//             return Err(());
//         }
//         let dist;
//         match Binomial::new(dist_params[0] as u64, dist_params[1]) {
//             Ok(d) => dist = d,
//             Err(_) => return Err(()),
//         }
//         // Sample n numbers
//         for _ in 0..n {
//             sampled = false;
//             for _ in 0..SAMPLE_LIMIT {
//                 let sampled_num = dist.sample(rng) as usize;
//                 if sampled_num >= ge {
//                     sampled_nums.push(sampled_num);
//                     sampled = true;
//                     break;
//                 }
//             }
//             if sampled == false {
//                 // No number was found for an iteration
//                 eprint!(
//                     "libalpaca: sample_from_distribution: {}: SAMLPE_LIMIT={} reached\n",
//                     dist_kind,
//                     SAMPLE_LIMIT
//                 );
//                 return Err(());
//             }
//         }
//     } else if dist_kind == "Gamma" {
//         // Gamma Distribution
//         if dist_params.len() != 2 {
//             return Err(());
//         }
//         let dist;
//         match Gamma::new(dist_params[0], dist_params[1]) {
//             Ok(d) => dist = d,
//             Err(_) => return Err(()),
//         }
//         // Sample n numbers
//         for _ in 0..n {
//             sampled = false;
//             for _ in 0..SAMPLE_LIMIT {
//                 let sampled_num = dist.sample(rng) as usize;
//                 if sampled_num >= ge {
//                     sampled_nums.push(sampled_num);
//                     sampled = true;
//                     break;
//                 }
//             }
//             if sampled == false {
//                 // No number was found for an iteration
//                 eprint!(
//                     "libalpaca: sample_from_distribution: {}: SAMLPE_LIMIT={} reached\n",
//                     dist_kind,
//                     SAMPLE_LIMIT
//                 );
//                 return Err(());
//             }
//         }
//     } else {
//         return Err(());
//     }

//     Ok(sampled_nums)
// }

// /// This function samples a value using vectors of values and probabilities
// /// which have already been created after parsing a distribution file.
// fn sample_from_file<R: Rng>(
//     rng: &mut R,
//     values: &Vec<usize>,
//     probs: &Vec<f64>,
//     ge: usize,
//     n: usize,
// ) -> Result<Vec<usize>, ()> {
//     let mut sampled_nums: Vec<usize> = Vec::with_capacity(n); // Vector with sampled numbers
//     let mut sampled;
//     // Sample n numbers
//     for _ in 0..n {
//         sampled = false;
//         // Try to sample a number greater or equal to ge `SAMPLE_LIMIT` times
//         for _ in 0..SAMPLE_LIMIT {
//             let probability: f64 = rng.sample(OpenClosed01);
//             let mut sum = 0.0;
//             let mut sampled_num = values[values.len() - 1];

//             // Sample a value from the given distribution
//             for i in 0..values.len() {
//                 sum += probs[i];
//                 if sum >= probability {
//                     sampled_num = values[i];
//                     break;
//                 }
//             }
//             if sampled_num >= ge {
//                 sampled_nums.push(sampled_num);
//                 eprint!("libalpaca: sampled {}\n", sampled_num);
//                 sampled = true;
//                 break;
//             }
//         }
//         if sampled == false {
//             // No number was found for an iteration
//             eprint!(
//                 "libalpaca: sample_from_file: SAMLPE_LIMIT={} reached\n",
//                 SAMPLE_LIMIT
//             );
//             return Err(());
//         }
//     }

//     Ok(sampled_nums)

// }

///// Resolves the absolute path of a distribution file.
// fn resolve_file_path(root: &str, dist: &str) -> String {
// 	let relative = String::from(dist);

// 	// Resolve the dots in the path so far
// 	let components: Vec<&str> = relative.split("/").collect(); 	// Original components of the path

// 	let mut normalized: Vec<String> = Vec::with_capacity(components.len()); // Stack to be used for the normalization

// 	for comp in components {
// 		if comp == "." || comp == "" {continue;}
// 		else if comp == ".." {
// 			if !normalized.is_empty() {
// 				normalized.pop();
// 			}
// 		}
// 		else {
// 			normalized.push("/".to_string()+comp);
// 		}
// 	}

// 	let mut absolute: String = normalized.into_iter().collect(); // String with the resolved relative path

// 	absolute.insert_str(0,root); // Make the above path absolute by adding the root

// 	absolute
// }
