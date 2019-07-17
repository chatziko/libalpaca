//! Provides functions to sample objects' count and size from a
//! probability distribution.
use std::{str,fs};
use rand::Rng;
use rand_distr::{LogNormal,Normal,Exp,Poisson,Binomial,Gamma,OpenClosed01,Distribution};

// Number of tries per sample. If no sampled number satisfies a specified
// threshold after `SAMPLE_LIMIT` tries the sampling function returns Err.
const SAMPLE_LIMIT: usize = 30;

/// A struct containing the 3 distributions needed for the probabilistic version.
/// Some fields contain "None", depending on whether a known distribution or a 
/// distribution file is given.
pub struct Distributions {
    /// Known distribution for html size
    pub dist_html: Option<String>,
    /// Known distribution for objects' number
    pub dist_obj_num: Option<String>,
    /// Known distribution for objects' size
    pub dist_obj_size: Option<String>,
    /// Vectors of probabilities and values
    /// taken from distributions files.
    pub dist_html_values: Option<Vec<usize>>,
    pub dist_html_probs: Option<Vec<f64>>,
    pub dist_obj_num_values: Option<Vec<usize>>,
    pub dist_obj_num_probs: Option<Vec<f64>>,
    pub dist_obj_size_values: Option<Vec<usize>>,
    pub dist_obj_size_probs: Option<Vec<f64>>,
}

impl Distributions {
    /// Construct a Distributions object.
    pub fn from(dist_html: &str, dist_obj_num: &str, dist_obj_size: &str, root: &str) -> Result<Distributions, ()> {
        let mut dists = Distributions {
            dist_html: None,
            dist_obj_num: None,
            dist_obj_size: None,
            dist_html_values: None,
            dist_html_probs: None,
            dist_obj_num_values: None,
            dist_obj_num_probs: None,
            dist_obj_size_values: None,
            dist_obj_size_probs: None,
        };

        // Parse html size distribution.
        if parse_given_dist(&mut dists,dist_html,root,1).is_err() {
            return Err(());
        }

        // Parse objects number distribution.
        if parse_given_dist(&mut dists,dist_obj_num,root,2).is_err() {
            return Err(());
        }

        // Parse objects size distribution.
        if parse_given_dist(&mut dists,dist_obj_size,root,3).is_err() {
            return Err(());
        }

        Ok(dists)
    }
}

/// Parses a given distribution and decides if its a known one or a file. Updates
/// the Distribtuions object accordingly.
fn parse_given_dist(obj: &mut Distributions,dist: &str, root: &str, n: usize) -> Result<(), ()>{
    if dist.ends_with(".dist") { // A distribution file has been given
        let mut absolute = String::from(root);
        absolute.push_str(dist);
        match fs::read_to_string(absolute) {
            Ok(data) => {
                let all_values: Vec<f64> = data.split_whitespace().map(|s| s.parse().unwrap()).collect(); // Contents of the file.
                if all_values.len() % 2 != 0 { // There has to be a 1-1 match between values and probabilities.
                    return Err(());
                }

                // Construct the 2 vectors containing the values and probabilities
                let mut values: Vec<usize> = Vec::with_capacity(all_values.len()/2);
                let mut probs: Vec<f64> = Vec::with_capacity(all_values.len()/2);
                for i in 0..all_values.len() {
                    if i % 2 == 0 {
                        values.push(all_values[i] as usize);
                    }
                    else {
                        probs.push(all_values[i]);
                    }
                }

                // Update the Distributions object to contain the vectors.
                if n == 1 {
                    obj.dist_html_values = Some(values);
                    obj.dist_html_probs = Some(probs);
                } else if n == 2 {
                    obj.dist_obj_num_values = Some(values);
                    obj.dist_obj_num_probs = Some(probs);
                } else if n == 3 {
                    obj.dist_obj_size_values = Some(values);
                    obj.dist_obj_size_probs = Some(probs);
                } else {
                    return Err(());
                }
            },
            Err(_) => return Err(())
        }
    }
    else { // A known distribution and its parameters have been given.
        if n == 1 {
            obj.dist_html = Some(String::from(dist));
        } else if n == 2 {
            obj.dist_obj_num = Some(String::from(dist));
        }
        else if n == 3 {
            obj.dist_obj_size = Some(String::from(dist));
        } else {
            return Err(());
        }
    }

    Ok(())
}


/// Samples the html target size.
pub fn sample_html_size<R: Rng>(rng: &mut R, dists: &Distributions, ge: usize) -> Result<usize, ()> {
    // Decide if a known distribution or a file was given.
    if !dists.dist_html.is_none() { // Known distribution.
        let dist;
        match dists.dist_html {
            Some(ref r) => dist = r,
            None => return Err(())
        }
        match sample_from_distribution(rng, dist, ge, 1) {
            Ok(sampled_nums) => return Ok(sampled_nums[0]),
            Err(_) => return Err(())
        }
    } else if !dists.dist_html_values.is_none() && !dists.dist_html_probs.is_none() { // File.
        let values;
        match dists.dist_html_values {
            Some(ref r) => values = r,
            None => return Err(())
        }
        let probs;
        match dists.dist_html_probs {
            Some(ref r) => probs = r,
            None => return Err(())
        }
        match sample_from_file(rng, values, probs, ge, 1) {
            Ok(sampled_nums) => return Ok(sampled_nums[0]),
            Err(_) => return Err(())
        }
    } else {
        return Err(());
    }
}

/// Samples the number of objects.
pub fn sample_object_num<R: Rng>(rng: &mut R, dists: &Distributions, ge: usize) -> Result<usize, ()> {
    // Decide if a known distribution or a file was given.
    if !dists.dist_obj_num.is_none() { // Known distribution.
        let dist;
        match dists.dist_obj_num {
            Some(ref r) => dist = r,
            None => return Err(())
        }
        match sample_from_distribution(rng, dist, ge, 1) {
            Ok(sampled_nums) => return Ok(sampled_nums[0]),
            Err(_) => return Err(())
        }
    } else if !dists.dist_obj_num_values.is_none() && !dists.dist_obj_num_probs.is_none() { // File.
        let values;
        match dists.dist_obj_num_values {
            Some(ref r) => values = r,
            None => return Err(())
        }
        let probs;
        match dists.dist_obj_num_probs {
            Some(ref r) => probs = r,
            None => return Err(())
        }
        match sample_from_file(rng, values, probs, ge, 1) {
            Ok(sampled_nums) => return Ok(sampled_nums[0]),
            Err(_) => return Err(())
        }
    } else {
        return Err(());
    }
}

/// Samples the objects' sizes..
pub fn sample_object_sizes<R: Rng>(rng: &mut R, dists: &Distributions, n: usize) -> Result<Vec<usize>, ()> {
    // Decide if a known distribution or a file was given.
    if !dists.dist_obj_size.is_none() { // Known distribution.
        let dist;
        match dists.dist_obj_size {
            Some(ref r) => dist = r,
            None => return Err(())
        }
        match sample_from_distribution(rng, dist, 1, n) {
            Ok(sampled_nums) => return Ok(sampled_nums),
            Err(_) => return Err(())
        }
    } else if !dists.dist_obj_size_values.is_none() && !dists.dist_obj_size_probs.is_none() { // File.
        let values;
        match dists.dist_obj_size_values {
            Some(ref r) => values = r,
            None => return Err(())
        }
        let probs;
        match dists.dist_obj_size_probs {
            Some(ref r) => probs = r,
            None => return Err(())
        }
        match sample_from_file(rng, values, probs, 1, n) {
            Ok(sampled_nums) => return Ok(sampled_nums),
            Err(_) => return Err(())
        }
    } else {
        return Err(());
    }
}

/// This function samples values from a given distribution
fn sample_from_distribution<R: Rng>(rng: &mut R, dist: &str, ge: usize, n: usize) -> Result<Vec<usize>, ()> {
    let tokens: Vec<&str> = dist.split("/").collect();
    if tokens.len() != 2 {
        return Err(());
    }

    let dist_kind = tokens[0]; // Name of the distribution
    let dist_params: Vec<f64> = tokens[1].split(",").map(|s| s.parse().unwrap()).collect(); // Distributions parameters
    let mut sampled_nums: Vec<usize> = Vec::with_capacity(n); // Vector with sampled numbers
    let mut sampled;

    if dist_kind == "Normal" { // Normal Distribution
        if dist_params.len() != 2 {
            return Err(());
        }
        let dist;        
        match Normal::new(dist_params[0],dist_params[1]) {
            Ok(d) => dist = d,
            Err(_) => return Err(())
        }
        // Sample n numbers
        for _ in 0..n {
            sampled = false;
            for _ in 0..SAMPLE_LIMIT {
                let sampled_num = dist.sample(rng) as usize;
                if sampled_num >= ge {
                    sampled_nums.push(sampled_num);
                    sampled = true;
                    break;
                }
            }  
            if sampled == false { // No number was found for an iteration
                return Err(());
            }
        }
    } else if dist_kind == "LogNormal" { // LogNormal Distribution
        if dist_params.len() != 2 {
            return Err(());
        }
        let dist;
        match LogNormal::new(dist_params[0],dist_params[1]) {
            Ok(d) => dist = d,
            Err(_) => return Err(())
        }
        // Sample n numbers
        for _ in 0..n {
            sampled = false;
            for _ in 0..SAMPLE_LIMIT {
                let sampled_num = dist.sample(rng) as usize;
                if sampled_num >= ge {
                    sampled_nums.push(sampled_num);
                    sampled = true;
                    break;
                }
            }  
            if sampled == false { // No number was found for an iteration
                return Err(());
            }
        }
    } else if dist_kind == "Exp" { // Exponential Distribution
        if dist_params.len() != 1 {
            return Err(());
        }
        let dist;
        match Exp::new(dist_params[0]) {
            Ok(d) => dist = d,
            Err(_) => return Err(())
        }
        // Sample n numbers
        for _ in 0..n {
            sampled = false;
            for _ in 0..SAMPLE_LIMIT {
                let sampled_num = dist.sample(rng) as usize;
                if sampled_num >= ge {
                    sampled_nums.push(sampled_num);
                    sampled = true;
                    break;
                }
            }  
            if sampled == false { // No number was found for an iteration
                return Err(());
            }
        }
    } else if dist_kind == "Poisson" { // Poisson Distribution
        if dist_params.len() != 1 {
            return Err(());
        }
        let dist;
        match Poisson::new(dist_params[0]) {
            Ok(d) => dist = d,
            Err(_) => return Err(())
        }
        // Sample n numbers
        for _ in 0..n {
            sampled = false;
            for _ in 0..SAMPLE_LIMIT {
                let sampled_num: u64 = dist.sample(rng);
                if sampled_num as usize >= ge {
                    sampled_nums.push(sampled_num as usize);
                    sampled = true;
                    break;
                }
            }  
            if sampled == false { // No number was found for an iteration
                return Err(());
            }
        }
    } else if dist_kind == "Binomial" { // Binomial Distribution
        if dist_params.len() != 2 {
            return Err(());
        }
        let dist;
        match Binomial::new(dist_params[0] as u64,dist_params[1]) {
            Ok(d) => dist = d,
            Err(_) => return Err(())
        }
        // Sample n numbers
        for _ in 0..n {
            sampled = false;
            for _ in 0..SAMPLE_LIMIT {
                let sampled_num = dist.sample(rng) as usize;
                if sampled_num >= ge {
                    sampled_nums.push(sampled_num);
                    sampled = true;
                    break;
                }
            }  
            if sampled == false { // No number was found for an iteration
                return Err(());
            }
        }
    } else if dist_kind == "Gamma" { // Gamma Distribution
        if dist_params.len() != 2 {
            return Err(());
        }
        let dist;
        match Gamma::new(dist_params[0],dist_params[1]) {
            Ok(d) => dist = d,
            Err(_) => return Err(())
        }
        // Sample n numbers
        for _ in 0..n {
            sampled = false;
            for _ in 0..SAMPLE_LIMIT {
                let sampled_num = dist.sample(rng) as usize;
                if sampled_num >= ge {
                    sampled_nums.push(sampled_num);
                    sampled = true;
                    break;
                }
            }  
            if sampled == false { // No number was found for an iteration
                return Err(());
            }
        }
    }
    else {
        return Err(());
    }

    Ok(sampled_nums)
}

/// This function samples a value using vectors of values and probabilities
/// which have already been created after parsing a distribution file. 
fn sample_from_file<R: Rng>(rng: &mut R, values: &Vec<usize>, probs: &Vec<f64>, ge: usize, n: usize) -> Result<Vec<usize>, ()> {     
    let mut sampled_nums: Vec<usize> = Vec::with_capacity(n); // Vector with sampled numbers
    let mut sampled;
    // Sample n numbers
    for _ in 0..n {
        sampled = false;
        // Try to sample a number greater or equal to ge `SAMPLE_LIMIT` times
        for _ in 0..SAMPLE_LIMIT {
            let probability: f64 = rng.sample(OpenClosed01);
            let mut sum = 0.0;
            let mut sampled_num = values[values.len()-1];
            
            // Sample a value from the given distribution
            for i in 0..values.len() {
                sum += probs[i];
                if sum >= probability {
                    sampled_num = values[i];
                    break;
                }
            }
            if sampled_num >= ge {
                sampled_nums.push(sampled_num);
                sampled = true;
                break;
            }
        }
        if sampled == false { // No number was found for an iteration
            return Err(());
        }
    }

    Ok(sampled_nums)

}
