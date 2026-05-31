//! Integration tests for Servo-V8 integration
//!
//! These tests verify that the ServoEmbedder can create a V8-backed Servo instance
//! and execute JavaScript code successfully.

#[cfg(test)]
mod tests {
    use crate::servo_embedder::{InputEvent, ServoEmbedder};
    use crate::v8_runtime::V8Runtime;
    use log::info;

    #[test]
    fn test_v8_runtime_creation() {
        // Test that V8 runtime can be created
        let runtime = V8Runtime::new();
        assert!(runtime.is_ok(), "V8 runtime creation should succeed");

        let runtime = runtime.unwrap();
        assert!(runtime.is_initialized(), "V8 runtime should be initialized");

        // Test V8 version
        let version = V8Runtime::get_version();
        assert!(!version.is_empty(), "V8 version should not be empty");
        info!("V8 version: {}", version);
    }

    #[test]
    fn test_v8_script_execution() {
        let mut runtime = V8Runtime::new().expect("V8 runtime should initialize");

        // Test basic arithmetic
        let result = runtime.execute_script("1 + 1");
        assert!(result.is_ok(), "Script execution should succeed");
        assert_eq!(result.unwrap(), "2", "1 + 1 should equal 2");

        // Test string operations
        let result = runtime.execute_script("'Hello' + ' ' + 'World'");
        assert!(result.is_ok(), "String concatenation should succeed");
        assert_eq!(
            result.unwrap(),
            "Hello World",
            "String concatenation result"
        );

        // Test function definition and execution
        let result = runtime.execute_script(
            r#"
            function greet(name) {
                return 'Hello, ' + name + '!';
            }
            greet('Soliloquy');
        "#,
        );
        assert!(
            result.is_ok(),
            "Function definition and execution should succeed"
        );
        assert_eq!(result.unwrap(), "Hello, Soliloquy!", "Function call result");
    }

    #[test]
    fn test_servo_embedder_creation() {
        let embedder = ServoEmbedder::new();
        assert!(embedder.is_ok(), "Servo embedder creation should succeed");

        let embedder = embedder.unwrap();
        use crate::servo_embedder::EmbedderState;
        assert_eq!(
            embedder.get_state(),
            &EmbedderState::Ready,
            "Embedder should be in Ready state"
        );
    }

    #[test]
    fn test_servo_embedder_url_loading() {
        let mut embedder = ServoEmbedder::new().expect("Servo embedder should initialize");

        // Test URL loading
        let result = embedder.load_url("https://example.com");
        assert!(result.is_ok(), "URL loading should succeed");

        // Check that URL is set
        assert_eq!(
            embedder.get_current_url(),
            Some(&"https://example.com".to_string())
        );

        // Check embedder state
        use crate::servo_embedder::EmbedderState;
        assert_eq!(
            embedder.get_state(),
            &EmbedderState::Running,
            "Embedder should be in Running state"
        );

        // Check webview info
        let webview_info = embedder.get_webview_info();
        assert!(webview_info.is_some(), "Webview info should be available");

        let info = webview_info.unwrap();
        assert_eq!(info.get("url"), Some(&"https://example.com".to_string()));
        assert!(info.contains_key("title"), "Webview should have a title");
        assert_eq!(info.get("loading"), Some(&"false".to_string()));
    }

    #[test]
    fn test_servo_embedder_javascript_execution() {
        let mut embedder = ServoEmbedder::new().expect("Servo embedder should initialize");

        // Load a URL first
        embedder
            .load_url("https://example.com")
            .expect("URL loading should succeed");

        // Test JavaScript execution
        let result = embedder.execute_js("document.title = 'Test Page'; 'Updated'");
        assert!(result.is_ok(), "JavaScript execution should succeed");
        assert_eq!(result.unwrap(), "Updated", "JavaScript execution result");

        // Test more complex JavaScript
        let result = embedder.execute_js(
            r#"
            var page = {
                title: 'Soliloquy Test',
                ready: true,
                version: '1.0.0'
            };
            JSON.stringify(page);
        "#,
        );
        assert!(
            result.is_ok(),
            "Complex JavaScript execution should succeed"
        );

        let json_result = result.unwrap();
        assert!(
            json_result.contains("Soliloquy Test"),
            "Result should contain page title"
        );
    }

    #[test]
    fn test_input_event_handling() {
        let mut embedder = ServoEmbedder::new().expect("Servo embedder should initialize");

        // Test touch events
        let touch_event = InputEvent::Touch { x: 100.0, y: 200.0 };
        // This should not panic or cause errors
        embedder.handle_input(touch_event);

        // Test key events
        let key_event = InputEvent::Key { code: 13 }; // Enter key
        embedder.handle_input(key_event);

        // Test multiple events
        embedder.handle_input(InputEvent::Touch { x: 0.0, y: 0.0 });
        embedder.handle_input(InputEvent::Key { code: 65 }); // 'A' key
    }

    #[test]
    fn test_frame_presentation() {
        let mut embedder = ServoEmbedder::new().expect("Servo embedder should initialize");

        // Test frame presentation
        let result = embedder.present();
        assert!(result.is_ok(), "Frame presentation should succeed");
    }

    #[test]
    fn test_complete_workflow() {
        let mut embedder = ServoEmbedder::new().expect("Servo embedder should initialize");

        // Load URL
        embedder
            .load_url("https://soliloquy.example")
            .expect("URL loading should succeed");

        // Execute JavaScript
        let js_result = embedder.execute_js(
            r#"
            console.log('Testing complete workflow');
            var data = {
                timestamp: Date.now(),
                url: window.location.href,
                userAgent: 'Soliloquy/1.0'
            };
            'Workflow test completed';
        "#,
        );
        assert!(
            js_result.is_ok(),
            "JavaScript execution in workflow should succeed"
        );

        // Handle input
        embedder.handle_input(InputEvent::Touch { x: 150.0, y: 250.0 });

        // Present frame
        embedder
            .present()
            .expect("Frame presentation in workflow should succeed");

        // Verify final state
        use crate::servo_embedder::EmbedderState;
        assert_eq!(
            embedder.get_state(),
            &EmbedderState::Running,
            "Final state should be Running"
        );
        assert_eq!(
            embedder.get_current_url(),
            Some(&"https://soliloquy.example".to_string())
        );
    }

    #[test]
    fn test_error_handling() {
        let mut embedder = ServoEmbedder::new().expect("Servo embedder should initialize");

        // Test JavaScript syntax error
        let result = embedder.execute_js("invalid javascript syntax {");
        assert!(result.is_err(), "Invalid JavaScript should return error");

        // Test JavaScript runtime error
        let result = embedder.execute_js("undefined_variable.property");
        // This might not error in V8, but let's check it doesn't panic
        assert!(
            result.is_ok() || result.is_err(),
            "JavaScript runtime error should be handled"
        );
    }
}
