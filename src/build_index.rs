use rand::seq::IndexedRandom;
use rinha::index::{VectorsData, l2_distance};
use std::sync::atomic::{AtomicUsize, Ordering};

const K: usize = 1024;

/// this function build the index doing K Means clustering
/// it should write the index to a binart compressed file.
pub fn main() {
    let data = VectorsData::load();
    let mut centroids = initialize_centroids(&data);

    println!("building index: {} vectors, K={K}", data.vectors.len());

    for iter in 0..20 {
        let mut clusters: Vec<Vec<_>> = vec![Vec::new(); K];

        for point in data.vectors.iter() {
            let mut closest_index = 0;
            let mut min_distance = l2_distance(point, &centroids[0]);
            // ckippy said it was better than a simple for. Ok then
            for (j, centroid) in centroids.iter().enumerate().skip(1) {
                let distance = l2_distance(point, centroid);
                if distance < min_distance {
                    min_distance = distance;
                    closest_index = j;
                }
            }

            clusters[closest_index].push(point);
        }
        let mut new_centroids = Vec::new();
        for (i, cluster) in clusters.iter().enumerate() {
            let new_centroid = calculate_centroid(cluster, &centroids[i]);
            new_centroids.push(new_centroid);
        }

        centroids = new_centroids;
        println!("  iter {}/20 done", iter + 1);
    }

    println!("done.");
}

// should pick K datapoints inside the data
fn initialize_centroids(data: &VectorsData) -> Vec<[f32; 14]> {
    let mut rng = rand::rng();
    let random = data.vectors.sample(&mut rng, K);

    let mut res = Vec::with_capacity(K);
    for r in random {
        res.push(*r); // clonning, but only happens on the first iteration
        // I'm clonning because it means the caller should own the data
    }
    res
}

fn calculate_centroid(vs: &Vec<&[f32; 14]>, fallback: &[f32; 14]) -> [f32; 14] {
    if vs.is_empty() {
        return *fallback;
    }
    let mut res = [0.0; 14];
    for i in 0..14 {
        let dimension_value: f32 = vs.iter().map(|s| s[i]).sum();
        res[i] = dimension_value / vs.len() as f32;
    }

    res
}
