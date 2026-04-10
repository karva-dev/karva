use divan::{Bencher, bench};

#[bench(sample_size = 2, sample_count = 2)]
fn karva_benchmark(bencher: Bencher) {
    karva_benchmark::bench(bencher);
}

fn main() {
    karva_benchmark::run_karva(&karva_benchmark::setup_project());
    divan::main();
}
