module main

// Display detection for Soliloquy/Zircon
// Uses real Zircon scenic FIDL bindings for display enumeration
// When no display is connected, runs as Cupboard sync server only (headless mode)

import vweb

// Import real Zircon scenic bindings
// Uses third_party/zircon_v/scenic for native FIDL display detection
import scenic

// Display info wrapper for API compatibility
struct DisplayInfo {
pub mut:
	available      bool
	width          int
	height         int
	refresh_hz     int
	width_mm       int
	height_mm      int
	dpr            f32
	name           string
	connection     string
	is_primary     bool
}

// Detect display using native Zircon bindings
pub fn detect_display() DisplayInfo {
	$if fuchsia ? {
		return detect_display_zircon()
	} $else {
		// Non-Fuchsia platforms: display detection not applicable
		return DisplayInfo{
			available: false
			width: 0
			height: 0
			refresh_hz: 0
			width_mm: 0
			height_mm: 0
			dpr: 1.0
			name: 'not-fuchsia'
			connection: 'none'
			is_primary: false
		}
	}
}

// Query Zircon scenic for display information
fn detect_display_zircon() DisplayInfo {
	println('🔍 Querying Zircon scenic for displays...')
	
	// Use real FIDL bindings to detect displays
	result := scenic.detect_displays()
	
	match result.query_result {
		.success {
			if result.displays.len > 0 {
				primary := result.displays[0]
				println('🖥️  Display detected: ${primary.format()}')
				
				connection_str := match primary.connection {
					.none { 'none' }
					.hdmi { 'hdmi' }
					.dp { 'displayport' }
					.dsi { 'dsi' }
					.edp { 'edp' }
					.lvds { 'lvds' }
					.internal { 'internal' }
					.virtual_ { 'virtual' }
				}
				
				return DisplayInfo{
					available: primary.state == .active || primary.state == .connected
					width: int(primary.metrics.extent_in_px_width)
					height: int(primary.metrics.extent_in_px_height)
					refresh_hz: int(primary.metrics.max_refresh_rate_mhz / 1000)
					width_mm: int(primary.metrics.extent_in_mm_width)
					height_mm: int(primary.metrics.extent_in_mm_height)
					dpr: primary.metrics.recommended_dpr_x
					name: primary.name
					connection: connection_str
					is_primary: primary.is_primary
				}
			}
			println('⚠️  No displays found (headless mode)')
			return no_display_info()
		}
		.no_displays {
			println('⚠️  No displays connected (headless mode)')
			return no_display_info()
		}
		.service_unavailable {
			println('⚠️  Scenic service unavailable - assuming headless')
			return no_display_info()
		}
		.permission_denied {
			println('❌ Permission denied accessing display service')
			return no_display_info()
		}
		.internal_error {
			println('❌ Internal error querying displays: ${result.error_message}')
			return no_display_info()
		}
	}
}

// Create info struct for headless mode
fn no_display_info() DisplayInfo {
	return DisplayInfo{
		available: false
		width: 0
		height: 0
		refresh_hz: 0
		width_mm: 0
		height_mm: 0
		dpr: 1.0
		name: 'none'
		connection: 'none'
		is_primary: false
	}
}

// Get hostname for network display
fn get_hostname() string {
	$if fuchsia ? {
		return 'soliloquy'
	} $else {
		return 'localhost'
	}
}

// Check if running in headless mode
pub fn (mut app App) check_headless_mode() bool {
	display := detect_display()
	
	if !display.available {
		println('🚫 No display detected - running in headless Cupboard server mode')
		println('📡 Devices can sync to this server at http://${get_hostname()}:3030')
		return true
	}
	
	println('🖥️  Display available: ${display.name} (${display.width}x${display.height}@${display.refresh_hz}Hz)')
	return false
}

// API endpoint: Get display information
@['/api/display/info'; get]
pub fn (mut app App) display_info() vweb.Result {
	display := detect_display()
	
	return app.json({
		'available': display.available.str()
		'width': display.width.str()
		'height': display.height.str()
		'refresh_rate_hz': display.refresh_hz.str()
		'physical_width_mm': display.width_mm.str()
		'physical_height_mm': display.height_mm.str()
		'dpr': display.dpr.str()
		'name': display.name
		'connection': display.connection
		'is_primary': display.is_primary.str()
		'mode': if display.available { 'desktop' } else { 'headless' }
	})
}

// API endpoint: Get all displays
@['/api/display/list'; get]
pub fn (mut app App) display_list() vweb.Result {
	$if fuchsia ? {
		result := scenic.detect_displays()
		
		if result.query_result != .success {
			return app.json({
				'displays': '[]'
				'count': '0'
				'error': result.error_message
			})
		}
		
		mut displays := []map[string]string{}
		
		for display in result.displays {
			displays << {
				'id': display.id.str()
				'name': display.name
				'width': display.metrics.extent_in_px_width.str()
				'height': display.metrics.extent_in_px_height.str()
				'is_primary': display.is_primary.str()
			}
		}
		
		return app.json(displays)
	} $else {
		return app.json({
			'displays': '[]'
			'count': '0'
			'error': 'not on fuchsia platform'
		})
	}
}
