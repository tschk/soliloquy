module main

import vweb
$if fuchsia ? {
	import os
}

const (
	port = 3030
)

// Application mode
enum AppMode {
	desktop   // Full desktop mode with Servo + V8
	headless  // Cupboard sync server only (no display)
}

pub struct App {
	vweb.Context
pub mut:
	config          Config
	sessions        map[string]Session
	zircon          ZirconContext
	cupboard        CupboardContext
	device_registry DeviceRegistry
	mode            AppMode
}

pub struct Session {
pub mut:
	user_id  string
	email    string
	name     string
	picture  string
}

pub struct User {
pub mut:
	id                   string
	email                string
	name                 string
	picture              string
	onboarding_complete  bool
}

fn main() {
	println('🚀 Starting Soliloquy Backend (V)')
	
	config := load_config()
	
	mut app := &App{
		config: config
		sessions: map[string]Session{}
		zircon: ZirconContext{
			enabled: false
			channels: map[string]ZirconChannel{}
		}
		cupboard: CupboardContext{
			initialized: false
			memories: map[string]Memory{}
			embeddings: map[string][]f32{}
			user_memory_counts: map[string]int{}
		}
		device_registry: DeviceRegistry{
			devices: map[string]DeviceInfo{}
		}
		mode: .headless  // Default to headless, will update after detection
	}
	
	// Initialize Zircon IPC bridge (if on Fuchsia)
	app.init_zircon()
	
	// Check if display is available (determines headless vs desktop mode)
	is_headless := app.check_headless_mode()
	
	if is_headless {
		app.mode = .headless
		println('📦 Running as Cupboard sync server only')
		println('🚫 Servo + V8 desktop will not start (no display)')
		println('')
		println('  This server provides:')
		println('  - Memory storage and retrieval')
		println('  - Device sync (push/pull)')
		println('  - REST API for all Cupboard operations')
		println('')
	} else {
		app.mode = .desktop
		println('🖥️  Desktop mode enabled (display detected)')
		println('💻 Signaling launcher to start Servo + V8')
		
		// Signal launcher that display is available
		// The launcher process reads this to determine if it should start Servo
		signal_desktop_ready()
	}
	
	// Initialize Cupboard memory storage (always runs)
	app.init_cupboard()
	
	// Initialize device sync registry (always runs)
	app.init_sync()
	
	println('')
	println('🌐 Backend API listening on http://localhost:${port}')
	println('📊 Health check: http://localhost:${port}/health')
	println('📋 Mode: ${if is_headless { "headless" } else { "desktop" }}')
	
	vweb.run(app, port)
}

// Helper for returning server errors with a message
pub fn (mut app App) server_error_msg(msg string) vweb.Result {
	app.set_status(500, 'Internal Server Error')
	return app.text(msg)
}

// Signal to launcher that desktop mode is ready
fn signal_desktop_ready() {
	$if fuchsia ? {
		// Write signal file that launcher watches
		os.write_file('/tmp/soliloquy_desktop_ready', 'ready') or {}
	}
}

// Health check endpoint
@['/health'; get]
pub fn (mut app App) health() vweb.Result {
	return app.text('ok')
}

// Mode endpoint - returns current operating mode
@['/api/mode'; get]
pub fn (mut app App) mode_endpoint() vweb.Result {
	mode_str := match app.mode {
		.desktop { 'desktop' }
		.headless { 'headless' }
	}
	
	return app.json({
		'mode': mode_str
		'cupboard_enabled': 'true'
		'sync_enabled': 'true'
		'servo_enabled': if app.mode == .desktop { 'true' } else { 'false' }
		'zircon_enabled': app.zircon.enabled.str()
	})
}
