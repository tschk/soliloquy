module main

import vweb
import json
import time

// Sync endpoints for headless Cupboard server
// Allows devices to sync memories, clipboard, and pickups

struct SyncRequest {
	device_id   string
	device_name string
	timestamp   i64
	items       []SyncItem
}

struct SyncItem {
	id         string
	item_type  string // 'memory', 'clipboard', 'pickup', 'note'
	content    string
	metadata   map[string]string
	created_at i64
}

struct SyncResponse {
pub mut:
	synced_count int
	new_items    []SyncItem
	timestamp    i64
}

struct DeviceInfo {
pub mut:
	id          string
	name        string
	last_seen   i64
	sync_count  int
	memory_count int
}

struct DeviceRegistry {
mut:
	devices map[string]DeviceInfo
}

pub fn (mut app App) init_sync() {
	println('📡 Initializing device sync registry')
	app.device_registry.devices = map[string]DeviceInfo{}
}

// Register or update a device
fn (mut app App) register_device(device_id string, device_name string) {
	mut device := app.device_registry.devices[device_id] or {
		DeviceInfo{
			id: device_id
			name: device_name
			last_seen: time.now().unix()
			sync_count: 0
			memory_count: 0
		}
	}
	
	device.last_seen = time.now().unix()
	device.sync_count++
	
	app.device_registry.devices[device_id] = device
}

// Sync endpoint for devices to push their data
@['/api/sync/push'; post]
pub fn (mut app App) sync_push() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(SyncRequest, app.req.data) or {
		return app.server_error('Invalid sync request')
	}
	
	// Register device
	app.register_device(payload.device_id, payload.device_name)
	
	mut synced := 0
	
	// Process sync items
	for item in payload.items {
		match item.item_type {
			'memory' {
				memory := Memory{
					user_id: session.user_id
					content: item.content
					metadata: item.metadata
					tags: []
					source: 'sync:${payload.device_name}'
					embedding: []
				}
				
				_ := app.cupboard_store(memory) or {
					eprintln('Failed to store synced memory: ${err}')
					continue
				}
				
				synced++
			}
			'clipboard' {
				// Store clipboard as memory
				memory := Memory{
					user_id: session.user_id
					content: item.content
					metadata: item.metadata
					tags: ['clipboard']
					source: 'clipboard:${payload.device_name}'
					embedding: []
				}
				
				_ := app.cupboard_store(memory) or {
					eprintln('Failed to store clipboard: ${err}')
					continue
				}
				
				synced++
			}
			else {
				println('Unknown sync item type: ${item.item_type}')
			}
		}
	}
	
	// Update device memory count
	mut device := app.device_registry.devices[payload.device_id] or {
		return app.server_error('Device not found')
	}
	device.memory_count += synced
	app.device_registry.devices[payload.device_id] = device
	
	println('📤 Synced ${synced} items from ${payload.device_name}')
	
	response := SyncResponse{
		synced_count: synced
		new_items: []
		timestamp: time.now().unix()
	}
	
	return app.json(response)
}

// Sync endpoint for devices to pull new data
@['/api/sync/pull'; post]
pub fn (mut app App) sync_pull() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(SyncRequest, app.req.data) or {
		return app.server_error('Invalid sync request')
	}
	
	// Register device
	app.register_device(payload.device_id, payload.device_name)
	
	// Get memories created after the client's last sync
	mut new_items := []SyncItem{}
	
	for _, memory in app.cupboard.memories {
		if memory.user_id == session.user_id && memory.created_at > payload.timestamp {
			new_items << SyncItem{
				id: memory.id
				item_type: 'memory'
				content: memory.content
				metadata: memory.metadata
				created_at: memory.created_at
			}
		}
	}
	
	println('📥 Sending ${new_items.len} new items to ${payload.device_name}')
	
	response := SyncResponse{
		synced_count: 0
		new_items: new_items
		timestamp: time.now().unix()
	}
	
	return app.json(response)
}

// Get list of registered devices
@['/api/sync/devices'; get]
pub fn (mut app App) sync_devices() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	_ := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	mut devices := []DeviceInfo{}
	for _, device in app.device_registry.devices {
		devices << device
	}
	
	return app.json(devices)
}

// Health check endpoint that includes sync status
@['/api/sync/status'; get]
pub fn (mut app App) sync_status() vweb.Result {
	device_count := app.device_registry.devices.len
	
	display := detect_display()
	mode := if display.available { 'desktop' } else { 'headless' }
	
	return app.json({
		'mode': mode
		'devices_connected': device_count.str()
		'cupboard_memories': app.cupboard.memories.len.str()
		'timestamp': time.now().unix().str()
	})
}
