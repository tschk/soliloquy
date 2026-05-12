use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rv8::js::JsEngine;
use tokio::runtime::Runtime;

fn js_execution_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("js_simple_arithmetic", |b| {
        b.to_async(&rt).iter(|| async {
            let mut engine = JsEngine::new().unwrap();
            black_box(engine.execute_to_string("1 + 1").unwrap());
        });
    });

    c.bench_function("js_function_call", |b| {
        b.to_async(&rt).iter(|| async {
            let mut engine = JsEngine::new().unwrap();
            black_box(
                engine
                    .execute_to_string(
                        "
                function test() { return 42; }
                test();
            ",
                    )
                    .unwrap(),
            );
        });
    });

    c.bench_function("js_dom_manipulation", |b| {
        b.to_async(&rt).iter(|| async {
            let mut engine = JsEngine::new().unwrap();
            black_box(
                engine
                    .execute_to_string(
                        "
                var div = { innerHTML: 'test' };
                div.innerHTML = 'updated';
                div.innerHTML;
            ",
                    )
                    .unwrap(),
            );
        });
    });
}

criterion_group!(benches, js_execution_benchmark);
criterion_main!(benches);
