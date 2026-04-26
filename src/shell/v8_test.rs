//! Simple V8 integration test without platform dependencies

mod js_engine;
mod v8_runtime;

use log::{info, error};

fn main() {
    // Initialize simple logger
    env_logger::init();
    
    info!("V8 Integration Test starting...");

    // Test V8 runtime creation
    match v8_runtime::V8Runtime::new() {
        Ok(mut runtime) => {
            info!("✅ V8 runtime created successfully");
            
            // Test basic script execution
            test_basic_scripts(&mut runtime);
            
            // Test arithmetic
            test_arithmetic(&mut runtime);
            
            // Test functions
            test_functions(&mut runtime);
            
            info!("✅ All V8 tests passed!");
        }
        Err(e) => {
            error!("❌ Failed to create V8 runtime: {}", e);
            return;
        }
    }
}

fn test_basic_scripts(runtime: &mut v8_runtime::V8Runtime) {
    info!("Testing basic script execution...");
    
    let test_cases = vec![
        ("String literal", "'Hello, V8!'", "Hello, V8!"),
        ("Number literal", "42", "42"),
        ("Boolean true", "true", "true"),
        ("Boolean false", "false", "false"),
        ("Null value", "null", "null"),
    ];
    
    for (name, script, expected) in test_cases {
        match runtime.execute_script(script) {
            Ok(result) => {
                if result == expected {
                    info!("✅ {}: {}", name, result);
                } else {
                    error!("❌ {}: Expected '{}', got '{}'", name, expected, result);
                }
            }
            Err(e) => {
                error!("❌ {}: {}", name, e);
            }
        }
    }
}

fn test_arithmetic(runtime: &mut v8_runtime::V8Runtime) {
    info!("Testing arithmetic operations...");
    
    let test_cases = vec![
        ("Addition", "2 + 3", "5"),
        ("Subtraction", "10 - 4", "6"),
        ("Multiplication", "6 * 7", "42"),
        ("Division", "20 / 4", "5"),
        ("Modulo", "17 % 5", "2"),
        ("Complex", "(2 + 3) * 4 - 6 / 2", "16"),
    ];
    
    for (name, script, expected) in test_cases {
        match runtime.execute_script(script) {
            Ok(result) => {
                if result == expected {
                    info!("✅ {}: {}", name, result);
                } else {
                    error!("❌ {}: Expected '{}', got '{}'", name, expected, result);
                }
            }
            Err(e) => {
                error!("❌ {}: {}", name, e);
            }
        }
    }
}

fn test_functions(runtime: &mut v8_runtime::V8Runtime) {
    info!("Testing function definitions and calls...");
    
    // Test function definition and call
    match runtime.execute_script(
        r#"
        function add(a, b) {
            return a + b;
        }
        add(5, 7);
        "#
    ) {
        Ok(result) => {
            if result == "12" {
                info!("✅ Function definition and call: {}", result);
            } else {
                error!("❌ Function test: Expected '12', got '{}'", result);
            }
        }
        Err(e) => {
            error!("❌ Function test: {}", e);
        }
    }
    
    // Test arrow function
    match runtime.execute_script(
        r#"
        const multiply = (x, y) => x * y;
        multiply(4, 6);
        "#
    ) {
        Ok(result) => {
            if result == "24" {
                info!("✅ Arrow function: {}", result);
            } else {
                error!("❌ Arrow function test: Expected '24', got '{}'", result);
            }
        }
        Err(e) => {
            error!("❌ Arrow function test: {}", e);
        }
    }
    
    // Test object creation
    match runtime.execute_script(
        r#"
        const person = {
            name: 'Soliloquy',
            version: '1.0',
            greet: function() {
                return 'Hello from ' + this.name + ' v' + this.version;
            }
        };
        person.greet();
        "#
    ) {
        Ok(result) => {
            if result.contains("Hello from Soliloquy") {
                info!("✅ Object method call: {}", result);
            } else {
                error!("❌ Object method test: Unexpected result '{}'", result);
            }
        }
        Err(e) => {
            error!("❌ Object method test: {}", e);
        }
    }
}
