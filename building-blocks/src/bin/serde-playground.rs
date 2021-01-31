use std::{fs::File, io::BufReader, io::Read, iter, str};

use bson::{doc, Document};
use rand::{
    distributions::{Distribution, Standard},
    seq::SliceRandom,
    thread_rng, Rng,
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
enum Axis {
    X,
    Y,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
struct Move {
    direction: Axis,
    distance: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BsonSucks {
    values: Vec<Move>,
}

impl Distribution<Axis> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Axis {
        *[Axis::X, Axis::Y].choose(rng).unwrap()
    }
}

impl Distribution<Move> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Move {
        Move {
            direction: rng.gen(),
            distance: rng.gen(),
        }
    }
}

fn main() {
    let a = Move {
        direction: Axis::X,
        distance: 42,
    };

    println!("Move -> JSON -> Move");
    let mut f = File::create("/tmp/serde-playground.json").unwrap();
    serde_json::to_writer(&mut f, &a).unwrap();
    let f = File::open("/tmp/serde-playground.json").unwrap();
    let b: Move = serde_json::from_reader(BufReader::new(f)).unwrap();
    println!("a: {:?}\nb: {:?}\n", a, b);

    println!("Move -> RON -> Move");
    let mut buf: Vec<u8> = vec![];
    ron::ser::to_writer(&mut buf, &a).unwrap();
    println!("ron: {}", str::from_utf8(&buf).unwrap());
    let b: Move = ron::de::from_reader(&*buf).unwrap();
    println!("a: {:?}\nb: {:?}\n", a, b);

    println!("Vec<Move> -> BSON -> Vec<Move>");
    let mut rng = thread_rng();
    let a: Vec<Move> = iter::repeat_with(|| rng.gen()).take(1000).collect();
    let mut buf: Vec<u8> = vec![];
    for v in &a {
        let doc: Document = bson::to_document(v).unwrap();
        doc.to_writer(&mut buf).unwrap();
    }

    let mut reader = BufReader::new(&*buf);
    let b: Vec<Move> = iter::repeat_with(|| {
        Document::from_reader(&mut reader)
            .ok()
            .and_then(|d| bson::from_document(d).ok())
    })
    .take_while(Option::is_some)
    .flatten()
    .collect();
    println!("a: {:?}\nb: {:?}", &a[..3], &b[..3]);
}
