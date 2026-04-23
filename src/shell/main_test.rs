//! Simplified main for testing V8 integration without platform dependencies

mod js_engine;
mod servo_embedder;
mod v8_runtime;

#[cfg(test)]
mod integration_tests;

use log::{info, error, debug};
use servo_embedder::{ServoEmbedder, InputEvent};
use std::env;

fn main() {
    // Initialize simple logger
    env_logger::init();
    
    info!("Soliloquy Shell starting (V8 Integration Test)...");

    // Initialize Servo embedder
    let mut embedder = match ServoEmbedder::new() {
        Ok(embedder) => {
            info!("Servo embedder initialized successfully");
            embedder
        }
        Err(e) => {
            error!("Failed to initialize Servo embedder: {}", e);
            return;
        }
    };

    // Load initial URL
    match embedder.load_url("https://example.com") {
        Ok(_) => info!("Initial URL loaded successfully"),
        Err(e) => error!("Failed to load initial URL: {}", e),
    }

    // Test V8 execution
    match embedder.execute_js("console.log('V8 is working in Soliloquy!'); 'V8 Test Success'") {
        Ok(result) => info!("V8 test result: {}", result),
        Err(e) => error!("V8 test failed: {}", e),
    }

    // Simulate some input events for testing
    embedder.handle_input(InputEvent::Touch { x: 100.0, y: 200.0 });
    embedder.handle_input(InputEvent::Key { code: 13 }); // Enter key

    // Present a frame
    match embedder.present() {
        Ok(_) => debug!("Frame presented successfully"),
        Err(e) => error!("Failed to present frame: {}", e),
    }

    // Print embedder state
    info!("Embedder state: {:?}", embedder.get_state());
    if let Some(url) = embedder.get_current_url() {
        info!("Current URL: {}", url);
    }
    
    if let Some(webview_info) = embedder.get_webview_info() {
        info!("Webview info: {:?}", webview_info);
    }

    info!("Soliloquy Shell V8 integration test completed successfully!");
    
    // Run some additional tests if requested
    if env::args().any(|arg| arg == "--test-js") {
        run_js_tests(&mut embedder);
    }
}

fn run_js_tests(embedder: &mut ServoEmbedder) {
    info!("Running JavaScript tests...");
    
    let test_cases = vec![
        ("Basic arithmetic", "2 + 3 * 4", "14"),
        ("String manipulation", "'Hello, ' + 'World!'", "Hello, World!"),
        ("Array operations", "[1, 2, 3].map(x => x * 2).join(',')", "2,4,6"),
        ("Object creation", "JSON.stringify({name: 'Soliloquy', version: '1.0'})", "\"name\":\"Soliloquy\",\"version\":\"1.0\"}"),
        ("Function definition", "(function(x) { return x * x; })(5)", "25"),
    ];
    
    for (name, script, expected_contains) in test_cases {
        match embedder.execute_js(script) {
            Ok(result) => {
                if result.contains(expected_contains) {
                    info!("✅ {}: {}", name, result);
                } else {
                    error!("❌ {}: Expected '{}', got '{}'", name, expected_contains, result);
                }
            }
            Err(e) => {
                error!("❌ {}: {}", name, e);
            }
        }
    }
}
