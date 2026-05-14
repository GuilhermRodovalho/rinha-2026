use std::{
    io::{BufReader, Read},
    sync::Arc,
};

use flate2::read::GzDecoder;

// ── raw data loader — usado apenas pelo build_index ──────────────────────────
// Lê do filesystem em vez de include_bytes! para não embutir 163MB no binário do servidor.

#[derive(Debug)]
pub struct VectorsData {
    pub vectors: Vec<[f32; 14]>,
    pub labels: Vec<bool>,
}

impl VectorsData {
    pub fn load() -> Arc<Self> {
        let bytes = std::fs::read("data/raw_data.bin")
            .expect("data/raw_data.bin not found — run from project root");

        let mut reader = BufReader::new(bytes.as_slice());
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

        Arc::new(VectorsData { vectors, labels })
    }
}

pub fn l2_distance(q: &[f32; 14], v: &[f32; 14]) -> f32 {
    let mut s = 0f32;
    for i in 0..14 {
        let d = q[i] - v[i];
        s += d * d;
    }
    s
}

// ── IVF index — carregado pelo servidor ──────────────────────────────────────

static IVF_BYTES: &[u8] = include_bytes!("../data/index.bin.gz");

const NPROBE: usize = 8;
const NPROBE_WIDE: usize = 24;

// Vetores guardados como i16 (× SCALE), dequantizados na hora do cálculo.
// 3M × 14 × 2 bytes = ~84 MB, versus ~168 MB em f32.
const SCALE: f32 = 10_000.0;

pub struct Index {
    centroids: Vec<[f32; 14]>,
    offsets: Vec<u32>,
    vectors: Vec<[i16; 14]>,
    labels: Vec<u8>,
}

impl Index {
    pub fn load() -> Arc<Self> {
        let decoder = GzDecoder::new(IVF_BYTES);
        let mut r = BufReader::new(decoder);

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"rivf", "invalid index format");

        let n = read_u32(&mut r) as usize;
        let k = read_u32(&mut r) as usize;
        let d = read_u32(&mut r) as usize;
        assert_eq!(d, 14);

        let mut centroids = Vec::with_capacity(k);
        for _ in 0..k {
            centroids.push(std::array::from_fn(|_| read_f32(&mut r)));
        }

        let mut offsets = Vec::with_capacity(k + 1);
        for _ in 0..=k {
            offsets.push(read_u32(&mut r));
        }

        let mut vectors: Vec<[i16; 14]> = Vec::with_capacity(n);
        for _ in 0..n {
            let v: [i16; 14] = std::array::from_fn(|_| {
                let x = read_f32(&mut r);
                (x * SCALE).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
            });
            vectors.push(v);
        }

        let mut labels = Vec::with_capacity(n);
        let mut buf = [0u8; 1];
        for _ in 0..n {
            r.read_exact(&mut buf).unwrap();
            labels.push(buf[0]);
        }

        println!("IVF index loaded: n={n}, k={k}");
        Arc::new(Index {
            centroids,
            offsets,
            vectors,
            labels,
        })
    }

    pub fn knn_fraud_ratio(&self, query: &[f32; 14], k: usize) -> f32 {
        let (fraud_count, total) = self.scan(query, k, NPROBE);
        if fraud_count == 2 || fraud_count == 3 {
            let (fc, t) = self.scan(query, k, NPROBE_WIDE);
            fc as f32 / t.max(1) as f32
        } else {
            fraud_count as f32 / total.max(1) as f32
        }
    }

    fn scan(&self, query: &[f32; 14], k: usize, nprobe: usize) -> (usize, usize) {
        let mut dists: Vec<(f32, usize)> = self
            .centroids
            .iter()
            .enumerate()
            .map(|(i, c)| (l2_distance(query, c), i))
            .collect();
        dists.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let mut top: Vec<(f32, u8)> = Vec::with_capacity(k + 1);

        for &(_, ci) in dists.iter().take(nprobe) {
            let start = self.offsets[ci] as usize;
            let end = self.offsets[ci + 1] as usize;
            for vi in start..end {
                let dist = l2_distance_i16(query, &self.vectors[vi]);
                let worst = top.last().map(|x| x.0).unwrap_or(f32::MAX);
                if top.len() < k || dist < worst {
                    top.push((dist, self.labels[vi]));
                    top.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                    top.truncate(k);
                }
            }
        }

        let fraud_count = top.iter().filter(|&&(_, l)| l != 0).count();
        (fraud_count, top.len())
    }
}

fn l2_distance_i16(q: &[f32; 14], v: &[i16; 14]) -> f32 {
    let inv = 1.0 / SCALE;
    let mut s = 0f32;
    for i in 0..14 {
        let d = q[i] - v[i] as f32 * inv;
        s += d * d;
    }
    s
}

// ── helpers ───────────────────────────────────────────────────────────────────

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
