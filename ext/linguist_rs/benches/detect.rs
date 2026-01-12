use std::fs::read_to_string;

use criterion::{Criterion, criterion_group, criterion_main};
use linguist::fixtures::{self, Sample};

fn detect_benchmark(c: &mut Criterion) {
    let fixtures = fixtures();
    c.bench_function("detect", |b| {
        b.iter(|| {
            for fixture in &fixtures {
                let language =
                    linguist::detect(&fixture.sample.path.to_string_lossy(), &fixture.content);
                assert!(language.is_some());
            }
        })
    });
    eprintln!("{} fixtures", fixtures.len());
}

fn fixtures() -> Vec<Fixture> {
    fixtures::samples()
        .into_iter()
        .map(|sample| {
            let content = read_to_string(&sample.path).unwrap();
            Fixture { sample, content }
        })
        .collect()
}

struct Fixture {
    sample: Sample,
    content: String,
}

criterion_group!(benches, detect_benchmark);
criterion_main!(benches);
