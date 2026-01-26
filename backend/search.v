module main

import vweb
import json

// Search integration for Soliloquy command bar
// Supports web search, command execution, and Cupboard retrieval

struct SearchCard {
pub mut:
	id          string
	title       string
	snippet     string
	url         string
	source      string
	image_url   string
	card_type   string // 'web', 'cupboard', 'command', 'browser'
	metadata    map[string]string
}

struct SearchResponse {
pub mut:
	query       string
	cards       []SearchCard
	suggestions []string
	took_ms     int
}

struct CommandResult {
pub mut:
	command     string
	output      string
	success     bool
	action      string // 'execute', 'search', 'navigate'
}

fn (mut app App) search_web(query string) ![]SearchCard {
	// TODO: Integrate with Perplexity API or SearXNG instance
	// For now, return mock results
	
	mut cards := []SearchCard{}
	
	// Mock web search results with carousel layout
	cards << SearchCard{
		id: 'web_1'
		title: 'Search results for: ${query}'
		snippet: 'This is a placeholder for web search integration. Connect to Perplexity or SearXNG for real results.'
		url: 'https://www.perplexity.ai/search?q=${query}'
		source: 'Perplexity'
		card_type: 'web'
		image_url: ''
		metadata: {}
	}
	
	return cards
}

fn (mut app App) search_cupboard(user_id string, query string) ![]SearchCard {
	memories := app.cupboard_retrieve(user_id, query, 5) or {
		return []SearchCard{}
	}
	
	mut cards := []SearchCard{}
	
	for memory in memories {
		cards << SearchCard{
			id: memory.id
			title: 'Memory: ${memory.source}'
			snippet: memory.content
			url: ''
			source: 'Cupboard'
			card_type: 'cupboard'
			image_url: ''
			metadata: memory.metadata
		}
	}
	
	return cards
}

fn (mut app App) parse_command(query string) !CommandResult {
	// Detect if query is a command or URL
	
	if query.starts_with('http://') || query.starts_with('https://') {
		return CommandResult{
			command: query
			output: 'Navigate to ${query}'
			success: true
			action: 'navigate'
		}
	}
	
	if query.starts_with('/') || query.starts_with('>') {
		// Plates-style command
		cmd := query.trim_left('/>').trim_space()
		return CommandResult{
			command: cmd
			output: 'Execute command: ${cmd}'
			success: true
			action: 'execute'
		}
	}
	
	// Default: search
	return CommandResult{
		command: query
		output: 'Search for: ${query}'
		success: true
		action: 'search'
	}
}

struct SearchRequest {
	query string
	limit int
}

@['/api/search'; post]
pub fn (mut app App) search() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(SearchRequest, app.req.data) or {
		return app.server_error_msg('Invalid payload')
	}
	
	// Parse query to determine intent
	cmd_result := app.parse_command(payload.query) or {
		return app.server_error_msg('Failed to parse command')
	}
	
	mut cards := []SearchCard{}
	
	match cmd_result.action {
		'navigate' {
			// Return navigation card
			cards << SearchCard{
				id: 'nav_1'
				title: 'Navigate to URL'
				snippet: cmd_result.command
				url: cmd_result.command
				source: 'Browser'
				card_type: 'browser'
				image_url: ''
				metadata: {}
			}
		}
		'execute' {
			// Return command execution card
			cards << SearchCard{
				id: 'cmd_1'
				title: 'Execute Command'
				snippet: cmd_result.output
				url: ''
				source: 'Plates'
				card_type: 'command'
				image_url: ''
				metadata: {
					'command': cmd_result.command
				}
			}
		}
		else {
			// Search mode: combine Cupboard + Web results
			h_cupboard := spawn app.search_cupboard(session.user_id, payload.query)
			h_web := spawn app.search_web(payload.query)

			cupboard_cards := h_cupboard.wait() or { []SearchCard{} }
			web_cards := h_web.wait() or { []SearchCard{} }

			cards << cupboard_cards
			cards << web_cards
		}
	}
	
	response := SearchResponse{
		query: payload.query
		cards: cards
		suggestions: []
		took_ms: 0
	}
	
	return app.json(response)
}

@['/api/search/suggestions'; post]
pub fn (mut app App) search_suggestions() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	_ := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	payload := json.decode(SearchRequest, app.req.data) or {
		return app.server_error_msg('Invalid payload')
	}
	
	// TODO: Return search suggestions based on Cupboard + recent searches
	suggestions := [
		payload.query + ' in soliloquy',
		payload.query + ' documentation',
		payload.query + ' tutorial'
	]
	
	return app.json({
		'suggestions': suggestions
	})
}
