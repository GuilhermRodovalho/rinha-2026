use std::{
    fs::File,
    io::{BufWriter, Write},
};

use flate2::{Compression, write::GzEncoder};
use rand::seq::IndexedRandom;
use rinha::index::{VectorsData, l2_distance};

const K: usize = 2048;
const MAX_ITERS: usize = 20;
const CONVERGENCE_EPS: f32 = 1e-4;

/// this function build the index doing K Means clustering
/// it should write the index to a binart compressed file.
pub fn main() {
    let data = VectorsData::load();
    let mut centroids = initialize_centroids(&data);
    let mut assignments = Vec::new();

    let n = data.vectors.len();
    let n_threads = std::thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(4);
    println!("building index: {n} vectors, K={K}, threads={n_threads}");

    for iter in 0..MAX_ITERS {
        // assign em paralelo: cada thread calcula o centróide mais próximo para seu chunk
        assignments = vec![0u16; n];
        let chunk = (n + n_threads - 1) / n_threads;
        let centroids_ref = &centroids;
        std::thread::scope(|s| {
            for (v_chunk, a_chunk) in data
                .vectors
                .chunks(chunk)
                .zip(assignments.chunks_mut(chunk))
            {
                s.spawn(move || {
                    for (v, a) in v_chunk.iter().zip(a_chunk.iter_mut()) {
                        let mut closest_index = 0u16;
                        let mut min_distance = l2_distance(v, &centroids_ref[0]);
                        for (j, centroid) in centroids_ref.iter().enumerate().skip(1) {
                            let distance = l2_distance(v, centroid);
                            if distance < min_distance {
                                min_distance = distance;
                                closest_index = j as u16;
                            }
                        }
                        *a = closest_index;
                    }
                });
            }
        });

        // monta clusters a partir dos assignments (single-threaded, rápido)
        let mut clusters: Vec<Vec<&[f32; 14]>> = vec![Vec::new(); K];
        for (point, &a) in data.vectors.iter().zip(assignments.iter()) {
            clusters[a as usize].push(point);
        }

        let old_centroids = centroids.clone();
        let mut new_centroids = Vec::new();
        for (i, cluster) in clusters.iter().enumerate() {
            let new_centroid = calculate_centroid(cluster, &old_centroids[i]);
            new_centroids.push(new_centroid);
        }
        centroids = new_centroids;

        let max_movement = centroids
            .iter()
            .zip(old_centroids.iter())
            .map(|(new, old)| l2_distance(new, old).sqrt())
            .fold(0.0f32, f32::max);

        println!("  iter {}/{MAX_ITERS} done (max movement: {max_movement:.6})", iter + 1);

        if max_movement < CONVERGENCE_EPS {
            println!("  converged early at iter {}", iter + 1);
            break;
        }
    }

    write_data_to_file(&data, &centroids, &assignments);

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

fn write_data_to_file(data: &VectorsData, centroids: &[[f32; 14]], assignments: &[u16]) {
    let file = File::create("data/index.bin.gz").expect("Failed to create index file");
    let writer = BufWriter::new(file);
    let mut writer = GzEncoder::new(writer, Compression::best());

    writer.write_all(b"rivf").unwrap(); // magic
    let mut clusters_indices = vec![Vec::new(); K];
    for (i, &a) in assignments.iter().enumerate() {
        clusters_indices[a as usize].push(i);
    }
    let mut offsets = vec![0u32; K + 1];
    for i in 0..K {
        offsets[i + 1] = offsets[i] + clusters_indices[i].len() as u32;
    }

    writer
        .write_all(&(data.vectors.len() as u32).to_le_bytes())
        .unwrap();
    writer.write_all(&(K as u32).to_le_bytes()).unwrap();
    writer.write_all(&14u32.to_le_bytes()).unwrap();
    for c in centroids {
        for &x in c.iter() {
            writer.write_all(&x.to_le_bytes()).unwrap();
        }
    }

    for o in &offsets {
        writer.write_all(&o.to_le_bytes()).unwrap();
    }

    for indices in &clusters_indices {
        for &vi in indices {
            for &x in data.vectors[vi].iter() {
                writer.write_all(&x.to_le_bytes()).unwrap();
            }
        }
    }

    for indices in &clusters_indices {
        for &vi in indices {
            writer.write_all(&[data.labels[vi] as u8]).unwrap();
        }
    }

    writer.flush().unwrap();
    writer.finish().unwrap();
    println!("index writter on data/index.bin.gz");
}
