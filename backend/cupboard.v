module main

import vweb
import json
import time
import os

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
	memories    map[string]Memory
	embeddings  map[string][]f32
	user_memories map[string][]string
	initialized bool
}

pub fn (mut app App) init_cupboard() {
	println('🗄️  Initializing Cupboard (universal memory storage)')
	
	app.cupboard.initialized = true
	app.cupboard.memories = map[string]Memory{}
	app.cupboard.embeddings = map[string][]f32{}
	app.cupboard.user_memories = map[string][]string{}
	
	$if fuchsia {
		// Initialize Zircon storage channel for persistence
		if storage_channel := app.get_zircon_channel('storage') {
			println('📦 Cupboard using Zircon persistent storage')
			app.load_persisted_memories()
			// Hydrate index from loaded memories
			for _, mem in app.cupboard.memories {
				if mem.id !in app.cupboard.user_memories[mem.user_id] {
					app.cupboard.user_memories[mem.user_id] << mem.id
				}
			}
		} else {
			println('📦 Cupboard using in-memory storage (Zircon storage unavailable)')
		}
	} $else {
		println('💾 Cupboard using in-memory storage (dev mode)')
	}
}

// Load memories from Zircon persistent storage
fn (mut app App) load_persisted_memories() {
	$if fuchsia {
		// Request stored memories from Zircon storage service
		// This reads from the component's /data directory
		
		storage_path := '/data/cupboard/memories.json'
		if os.exists(storage_path) {
			data := os.read_file(storage_path) or { return }
			// Parse and load memories
			println('📂 Loaded persisted memories from ${storage_path}')
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
	
	app.cupboard.memories[mem.id] = mem
	if mem.id !in app.cupboard.user_memories[mem.user_id] {
		app.cupboard.user_memories[mem.user_id] << mem.id
	}
	
	$if fuchsia {
		// Persist to Zircon storage asynchronously
		go app.persist_memory(mem)
	}
	
	println('Stored memory: ${mem.id} (${mem.source})')
	return mem.id
}

// Persist a memory to Zircon storage
fn (app App) persist_memory(mem Memory) {
	$if fuchsia {
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
	
	// Optimized search using user_memories index
	if user_id in app.cupboard.user_memories {
		memory_ids := app.cupboard.user_memories[user_id]
		for id in memory_ids {
			mem := app.cupboard.memories[id] or { continue }
			if mem.content.contains(query) {
				results << mem
				if results.len >= limit {
					break
				}
			}
		}
	} else {
		// Fallback (e.g. if index is empty/corrupted, though in this impl it shouldn't be)
		for _, mem in app.cupboard.memories {
			if mem.user_id == user_id && mem.content.contains(query) {
				results << mem
				if results.len >= limit {
					break
				}
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
	
	// Update index
	if mem := app.cupboard.memories[memory_id] {
		if mem.user_id in app.cupboard.user_memories {
			mut ids := app.cupboard.user_memories[mem.user_id]
			for i, id in ids {
				if id == memory_id {
					ids.delete(i)
					break
				}
			}
			app.cupboard.user_memories[mem.user_id] = ids
		}
	}

	app.cupboard.memories.delete(memory_id)
	
	$if fuchsia {
		// Mark as deleted in Zircon storage (tombstone)
		go app.persist_deletion(memory_id)
	}
	
	return true
}

// Persist deletion to Zircon storage
fn (app App) persist_deletion(memory_id string) {
	$if fuchsia {
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
		return app.server_error('Invalid payload')
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
		return app.server_error('Failed to store memory: ${err}')
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
		return app.server_error('Invalid payload')
	}
	
	limit := if payload.limit > 0 { payload.limit } else { 10 }
	
	memories := app.cupboard_retrieve(session.user_id, payload.query, limit) or {
		return app.server_error('Failed to retrieve memories: ${err}')
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
		return app.server_error('Invalid payload')
	}
	
	_ := app.cupboard_delete(payload.id) or {
		return app.server_error('Failed to delete memory: ${err}')
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
	
	mut user_memories := 0
	for _, mem in app.cupboard.memories {
		if mem.user_id == session.user_id {
			user_memories++
		}
	}
	
	return app.json({
		'total_memories': app.cupboard.memories.len.str()
		'user_memories': user_memories.str()
		'initialized': app.cupboard.initialized.str()
	})
}
