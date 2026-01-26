module main

import vweb
import json
import time
$if fuchsia ? {
	import os
}

// Cupboard - Universal memory storage for Soliloquy
// Stores user memories, pickups, clipboard history, and universal context
// Uses Zircon for persistent storage when running on Fuchsia

struct Memory {
pub mut:
	id          string
	user_id     string
	content     string
	embedding   []f32
	metadata    map[string]string
	created_at  i64
	updated_at  i64
	tags        []string
	source      string // 'user', 'search', 'clipboard', 'pickup'
}

struct SearchResult {
pub mut:
	memory     Memory
	similarity f32
	rank       int
}

struct CupboardContext {
mut:
	memories           map[string]Memory
	embeddings         map[string][]f32
	user_memory_counts map[string]int
	initialized        bool
}

pub fn (mut app App) init_cupboard() {
	println('🗄️  Initializing Cupboard (universal memory storage)')
	
	app.cupboard.initialized = true
	app.cupboard.memories = map[string]Memory{}
	app.cupboard.embeddings = map[string][]f32{}
	app.cupboard.user_memory_counts = map[string]int{}
	
	$if fuchsia ? {
		// Initialize Zircon storage channel for persistence
		if storage_channel := app.get_zircon_channel('storage') {
			println('📦 Cupboard using Zircon persistent storage')
			app.load_persisted_memories()
		} else {
			println('📦 Cupboard using in-memory storage (Zircon storage unavailable)')
		}
	} $else {
		println('💾 Cupboard using in-memory storage (dev mode)')
	}
}

// Load memories from Zircon persistent storage
fn (mut app App) load_persisted_memories() {
	$if fuchsia ? {
		// Request stored memories from Zircon storage service
		// This reads from the component's /data directory
		
		mut storage_path := '/data/cupboard/memories.jsonl'
		if !os.exists(storage_path) {
			storage_path = '/data/cupboard/memories.json'
		}

		if os.exists(storage_path) {
			data := os.read_file(storage_path) or { return }
			lines := data.split('\n')

			mut loaded_count := 0
			for line in lines {
				if line.trim_space() == '' {
					continue
				}

				mem := json.decode(Memory, line) or {
					eprintln('Failed to decode memory: ${err}')
					continue
				}

				app.cupboard.memories[mem.id] = mem
				app.cupboard.user_memory_counts[mem.user_id]++
				loaded_count++
			}

			println('📂 Loaded ${loaded_count} persisted memories from ${storage_path}')
		}
	}
}

// Store a memory to Cupboard
fn (mut app App) cupboard_store(memory Memory) !string {
	if !app.cupboard.initialized {
		return error('Cupboard not initialized')
	}
	
	mut mem := memory
	mem.id = time.now().unix().str() + '_' + mem.user_id
	mem.created_at = time.now().unix()
	mem.updated_at = time.now().unix()
	
	if mem.id !in app.cupboard.memories {
		app.cupboard.user_memory_counts[mem.user_id]++
	}
	app.cupboard.memories[mem.id] = mem
	
	$if fuchsia ? {
		// Persist to Zircon storage asynchronously
		go app.persist_memory(mem)
	}
	
	println('Stored memory: ${mem.id} (${mem.source})')
	return mem.id
}

// Persist a memory to Zircon storage
fn (app App) persist_memory(mem Memory) {
	$if fuchsia ? {
		// Ensure storage directory exists
		storage_dir := '/data/cupboard'
		if !os.is_dir(storage_dir) {
			os.mkdir_all(storage_dir) or { return }
		}
		
		// Write memory to individual file (append to memories.jsonl)
		storage_path := '${storage_dir}/memories.jsonl'
		mem_json := json.encode(mem)
		os.write_file(storage_path, mem_json + '\n') or { return }
	}
}

// Retrieve memories from Cupboard
fn (mut app App) cupboard_retrieve(user_id string, query string, limit int) ![]Memory {
	if !app.cupboard.initialized {
		return error('Cupboard not initialized')
	}
	
	mut results := []Memory{}
	
	// Simple keyword search for now (TODO: implement vector similarity search)
	for _, mem in app.cupboard.memories {
		if mem.user_id == user_id && mem.content.contains(query) {
			results << mem
			if results.len >= limit {
				break
			}
		}
	}
	
	return results
}

// Delete a memory from Cupboard
fn (mut app App) cupboard_delete(memory_id string) !bool {
	if !app.cupboard.initialized {
		return error('Cupboard not initialized')
	}
	
	if mem := app.cupboard.memories[memory_id] {
		app.cupboard.user_memory_counts[mem.user_id]--
	}

	app.cupboard.memories.delete(memory_id)
	
	$if fuchsia ? {
		// Mark as deleted in Zircon storage (tombstone)
		go app.persist_deletion(memory_id)
	}
	
	return true
}

// Persist deletion to Zircon storage
fn (app App) persist_deletion(memory_id string) {
	$if fuchsia ? {
		storage_path := '/data/cupboard/deletions.log'
		entry := '${time.now().unix()}:${memory_id}\n'
		
		os.write_file(storage_path, entry) or { return }
	}
}

// API endpoints for Cupboard

struct MemoryStoreRequest {
	content  string
	metadata map[string]string
	tags     []string
	source   string
}

@['/api/cupboard/store'; post]
pub fn (mut app App) cupboard_store_endpoint() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(MemoryStoreRequest, app.req.data) or {
		app.set_status(500, 'Invalid payload')
		return app.text('Invalid payload')
	}
	
	memory := Memory{
		user_id: session.user_id
		content: payload.content
		metadata: payload.metadata
		tags: payload.tags
		source: payload.source
		embedding: []
	}
	
	memory_id := app.cupboard_store(memory) or {
		app.set_status(500, 'Failed to store memory: ${err}')
		return app.text('Failed to store memory: ${err}')
	}
	
	return app.json({
		'id': memory_id
		'status': 'ok'
	})
}

struct MemoryRetrieveRequest {
	query string
	limit int
}

@['/api/cupboard/retrieve'; post]
pub fn (mut app App) cupboard_retrieve_endpoint() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(MemoryRetrieveRequest, app.req.data) or {
		app.set_status(500, 'Invalid payload')
		return app.text('Invalid payload')
	}
	
	limit := if payload.limit > 0 { payload.limit } else { 10 }
	
	memories := app.cupboard_retrieve(session.user_id, payload.query, limit) or {
		app.set_status(500, 'Failed to retrieve memories: ${err}')
		return app.text('Failed to retrieve memories: ${err}')
	}
	
	return app.json(memories)
}

struct MemoryDeleteRequest {
	id string
}

@['/api/cupboard/delete'; post]
pub fn (mut app App) cupboard_delete_endpoint() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	_ := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(MemoryDeleteRequest, app.req.data) or {
		app.set_status(500, 'Invalid payload')
		return app.text('Invalid payload')
	}
	
	_ := app.cupboard_delete(payload.id) or {
		app.set_status(500, 'Failed to delete memory: ${err}')
		return app.text('Failed to delete memory: ${err}')
	}
	
	return app.json({
		'status': 'ok'
	})
}

@['/api/cupboard/stats'; get]
pub fn (mut app App) cupboard_stats() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	user_memories := app.cupboard.user_memory_counts[session.user_id]
	
	return app.json({
		'total_memories': app.cupboard.memories.len.str()
		'user_memories': user_memories.str()
		'initialized': app.cupboard.initialized.str()
	})
}
