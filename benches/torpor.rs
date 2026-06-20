use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use torpor::timelock::Solver;
use torpor::timelock::server::Trapdoor;
use torpor::{Difficulty, Wide};

const MODULUS_HEX: &str = concat!(
    "af64b6097b3ee918c3039c70b630308d03f90112a044ea15c6516c254bdeb03e",
    "3fb592eac5db1db254bef997a4d1369b4b5a3d1e3b1aa455b73b56f85b529602",
    "5b4bb4722b190cc12c43cdc71aba66d75413549220b738cc09a339f5d5a0d944",
    "be2bd78028b4ee3417a66249b0ae5a90b55a9cb138bd0c8cf6a2c26c88bc38fc",
    "f308ba799ab8878522eff0a94bc833917f8f210362feafe955026a74a546ee3a",
    "cc2fbde41a925abcba3a3dde10522ea9abd3ba378e1aad597ea94c86bce859cc",
    "75c01265549d658628a6d9320ad35423b8983b0e9273e4e95c9c1875e959d1f2",
    "42c216af76b85a66568e1c0fffbc2187b80e88358a2a2ad997deb97cfd1dedbb",
);
const BASE_HEX: &str = concat!(
    "15c810fab4c3136524819be34e8e83eeff642fa1d16546bbd98edfa52e8f639d",
    "1907064f076ad7a11fb25b61680568198df808f2a00d69117187aba037f3eaf3",
    "7eee9f8fd9ae22301de5f15525bbbb053936876cc9eff1221f0dcc27cee07b52",
    "ee6f3994e1f11d6ed3c117a01191c98532d6008c318e88c4d860ba17cd4fe87a",
    "36e9b77d07148982908d25e3ed828287aeb8cc8150df3e7c17afc84c3923a6f3",
    "ac7d427f54e158a3f533ff2b01e0c6aaa01d1ccad6cb745c9d978f8b92204e15",
    "c3fcf511c057e5245fa93d04e14eb65a3185ba6aa74e878e76b6739e5cc91fdf",
    "69c78f8f5d36450c40e9ceb3dde5bed9b9577b9d12a207dad9b50d3cc5d226e8",
);

fn client_squaring(c: &mut Criterion) {
    let modulus = Wide::from_hex(MODULUS_HEX).unwrap();
    let base = Wide::from_hex(BASE_HEX).unwrap();
    const SQUARINGS: u64 = 4096;

    let mut group = c.benchmark_group("client");
    group.throughput(Throughput::Elements(SQUARINGS));
    group.bench_function("modular_squaring", |b| {
        b.iter(|| {
            Solver::solve(black_box(&modulus), black_box(&base), Difficulty(SQUARINGS)).unwrap()
        });
    });
    group.finish();
}

fn server_issue(c: &mut Criterion) {
    let trapdoor = Trapdoor::generate(2048);

    let mut group = c.benchmark_group("server");
    group.bench_function("new_challenge", |b| {
        b.iter(|| black_box(trapdoor.new_challenge(Difficulty(1_000_000))));
    });
    group.finish();
}

criterion_group!(benches, client_squaring, server_issue);
criterion_main!(benches);
