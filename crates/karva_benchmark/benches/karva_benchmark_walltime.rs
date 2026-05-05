use divan::{Bencher, bench};

#[bench(sample_size = 2, sample_count = 1)]
fn karva_benchmark(bencher: Bencher) {
    karva_benchmark::bench(bencher);
}

fn main() {
    divan::main();
}
