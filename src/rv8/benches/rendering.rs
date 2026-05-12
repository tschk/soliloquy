use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rv8::servo_embed::ServoEmbedder;
use tokio::runtime::Runtime;

fn rendering_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("servo_html_parsing", |b| {
        b.to_async(&rt).iter(|| async {
            let config = rv8::servo_embed::ServoConfig::default();
            let mut embedder = ServoEmbedder::new(config).await.unwrap();
            black_box(
                embedder
                    .navigate("data:text/html,<html><body><h1>Test</h1></body></html>")
                    .await
                    .unwrap(),
            );
        });
    });

    c.bench_function("servo_js_execution", |b| {
        b.to_async(&rt).iter(|| async {
            let config = rv8::servo_embed::ServoConfig::default();
            let embedder = ServoEmbedder::new(config).await.unwrap();
            black_box(embedder.execute_script("1 + 1").await.unwrap());
        });
    });

    c.bench_function("servo_dom_query", |b| {
        b.to_async(&rt).iter(|| async {
            let config = rv8::servo_embed::ServoConfig::default();
            let mut embedder = ServoEmbedder::new(config).await.unwrap();
            embedder
                .navigate("data:text/html,<html><body><div id='test'>content</div></body></html>")
                .await
                .unwrap();
            // Simulate DOM query
            black_box(
                embedder
                    .execute_script("document.getElementById('test').innerHTML")
                    .await
                    .unwrap(),
            );
        });
    });
}

criterion_group!(benches, rendering_benchmark);
criterion_main!(benches);
