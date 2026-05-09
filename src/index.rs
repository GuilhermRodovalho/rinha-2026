use std::{
    error::Error,
    io::{BufReader, Read},
    sync::Arc,
};

static INDEX_BYTES: &[u8] = include_bytes!("../data/raw_data.bin");

pub struct Index {
    vectors: Vec<[f32; 14]>,
    labels: Vec<bool>,
}

impl Index {
    pub fn load() -> Arc<Self> {
        let bytes = INDEX_BYTES;

        let mut reader = BufReader::new(bytes);
        let mut magic = [0; 9];
        reader.read_exact(&mut magic).unwrap();
        if &magic != b"rinha2026" {
            panic!("invalid binary format");
        }
        let len = read_u32(&mut reader) as usize;
        let dimension = read_u32(&mut reader) as usize;
        if dimension != 14 {
            panic!("arrays with dimensions != 14 {}", dimension);
        }

        println!("length dos dados {}", len);

        let mut vectors = Vec::with_capacity(len);
        for _ in 0..len {
            let curr = std::array::from_fn(|_| read_f32(&mut reader));
            vectors.push(curr);
        }

        let mut labels = Vec::with_capacity(len);
        for i in 0..len {
            let is_fraud = read_bool(&mut reader)
                .unwrap_or_else(|_| panic!("error reading boolean at index {}", i));
            labels.push(is_fraud);
        }

        Arc::new(Index { vectors, labels })
    }

    pub fn knn_fraud_ratio(&self, query: &[f32; 14], k: usize) -> f32 {
        let mut top: Vec<(f32, bool)> = Vec::with_capacity(k + 1);

        for (vec, &fraud) in self.vectors.iter().zip(&self.labels) {
            let distance = l2_distance(query, vec);
            if top.len() < k || distance < top.last().unwrap().0 {
                top.push((distance, fraud));
                top.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                top.truncate(k);
            }
        }

        (top.iter().filter(|&&(_, b)| b).count() as f32) / (k as f32)
    }
}

fn l2_distance(q: &[f32; 14], v: &[f32; 14]) -> f32 {
    let mut s = 0f32;
    for i in 0..14 {
        let d = q[i] - v[i];
        s += d * d;
    }
    s
}

// dava pra ser genérica sobre o tipo de retorno, mas dá trabalho
fn read_u32<R: Read>(reader: &mut R) -> u32 {
    let mut data = [0u8; 4];
    reader.read_exact(&mut data).unwrap();
    u32::from_le_bytes(data)
}

fn read_f32<R: Read>(reader: &mut R) -> f32 {
    let mut data = [0u8; 4];
    reader.read_exact(&mut data).unwrap();
    f32::from_le_bytes(data)
}

fn read_bool<R: Read>(reader: &mut R) -> std::io::Result<bool> {
    let mut data = [0u8; 1];
    reader.read_exact(&mut data)?;
    Ok(data[0] != 0)
}
