use std::{
    fmt, fs,
    io::{BufReader, BufWriter, Write},
};

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProccessedTransactionData {
    pub vector: [f32; 14],
    pub label: String,
}

struct SenderVisitor {
    vectors: Vec<[f32; 14]>,
    labels: Vec<u8>,
}

impl<'de> serde::de::Visitor<'de> for SenderVisitor {
    type Value = (Vec<[f32; 14]>, Vec<u8>);

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "an array of transaction data")
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        mut self,
        mut seq: A,
    ) -> Result<Self::Value, A::Error> {
        while let Some(item) = seq.next_element::<ProccessedTransactionData>()? {
            self.vectors.push(item.vector);
            self.labels.push(if item.label == "fraud" { 1 } else { 0 });
        }
        Ok((self.vectors, self.labels))
    }
}

fn main() {
    let (vecs, labels) = load_data();

    write_raw_bin(vecs, labels);
}

fn load_data() -> (Vec<[f32; 14]>, Vec<u8>) {
    let file = fs::File::open("data/references.json").expect("couldn't find file references.json");
    let reader = BufReader::new(file);
    let mut deserializer = serde_json::Deserializer::from_reader(reader);

    deserializer
        .deserialize_seq(SenderVisitor {
            vectors: Vec::with_capacity(3000000),
            labels: Vec::with_capacity(3000000),
        })
        .expect("error reding the data")
}

fn write_raw_bin(vecs: Vec<[f32; 14]>, labels: Vec<u8>) {
    let out_path = "data/raw_data.bin";
    let file = fs::File::create(out_path).expect("couldn't create raw_data.bin");

    let mut writer = BufWriter::new(file);
    writer.write_all(b"rinha2026").unwrap();
    let len = vecs.len() as u32;
    writer.write_all(&len.to_le_bytes()).unwrap();
    // dimensões dos arrays
    writer.write_all(&u32::to_le_bytes(14)).unwrap();

    for v in vecs {
        for f in v {
            writer.write_all(&f.to_le_bytes()).unwrap();
        }
    }

    // teoricamente, poderia ser um único bit ao invés de um byte
    writer.write_all(&labels).unwrap();
    writer.flush().unwrap();
}
