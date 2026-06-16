import { browserStore, commandBarStore, type BrowserTab, type SearchEngine, type CommandResult, type Workspace, type BrowserState } from '$lib/stores/browser';
import { fuzzySearch, fuzzyScore, getLuckySearchUrl, type SearchableItem } from '$lib/utils/fuzzy';
import { isValidUrl, normalizeUrl } from '$lib/system/actions';
import { get } from 'svelte/store';

// =========================================================================
// Actions
// =========================================================================

export function navigateToUrl(url: string) {
	const normalized = normalizeUrl(url);
	browserStore.openTab(normalized, { activate: true });
	browserStore.addToHistory(normalized, normalized);
	commandBarStore.close();
}

export function executeSearch(engine: SearchEngine, searchQuery: string) {
	const url = engine.searchUrl.replace('%s', encodeURIComponent(searchQuery));
	browserStore.openTab(url, { activate: true });
	browserStore.addToHistory(url, `${engine.name}: ${searchQuery}`);
	commandBarStore.close();
}

export function searchPlates(searchQuery: string) {
	const url = `https://www.google.com/search?q=${encodeURIComponent(searchQuery)}`;
	browserStore.openTab(url, { activate: true });
	browserStore.addToHistory(url, `Search: ${searchQuery}`);
	commandBarStore.close();
}

// =========================================================================
// Built-in Commands
// =========================================================================

export const COMMANDS = [
	{ id: 'list-tabs', name: 'List Tabs', aliases: ['tabs', 'show tabs', 'all tabs'], icon: '📑', action: () => commandBarStore.setMode('tabs') },
	{ id: 'list-workspaces', name: 'List Workspaces', aliases: ['workspaces', 'spaces'], icon: '📁', action: () => commandBarStore.setMode('workspace') },
	{ id: 'new-tab', name: 'New Tab', aliases: ['open tab', 'create tab'], icon: '➕', action: () => { browserStore.openTab('about:blank'); commandBarStore.close(); } },
	{ id: 'close-tab', name: 'Close Tab', aliases: ['close', 'kill tab'], icon: '✕', action: () => {
		const state = get(browserStore);
		if (state.activeTabId) browserStore.closeTab(state.activeTabId);
		commandBarStore.close();
	}},
	{ id: 'new-workspace', name: 'New Workspace', aliases: ['create workspace', 'add workspace'], icon: '📂', action: () => {
		browserStore.createWorkspace('New Workspace');
		commandBarStore.close();
	}},
	{ id: 'bookmark', name: 'Bookmark Page', aliases: ['add bookmark', 'save'], icon: '⭐', action: () => {
		const state = get(browserStore);
		const tab = state.tabs.find(t => t.id === state.activeTabId);
		if (tab) browserStore.addBookmark(tab.url, tab.title);
		commandBarStore.close();
	}},
	{ id: 'history', name: 'View History', aliases: ['show history', 'recent'], icon: '🕐', action: () => commandBarStore.setMode('tabs') },
	{ id: 'settings', name: 'Settings', aliases: ['preferences', 'config'], icon: '⚙️', action: () => commandBarStore.close() },
];

export interface SuggestionContext {
	browser: BrowserState;
	tabCounts: Record<string, number>;
}

// =========================================================================
// Suggestion Computation
// =========================================================================

export function findMatchingEngines(q: string, searchEngines: SearchEngine[]): SearchEngine[] {
	if (!q.trim()) return [];
	const lower = q.toLowerCase();
	return searchEngines.filter(e =>
		e.keyword.toLowerCase().startsWith(lower) ||
		e.name.toLowerCase().startsWith(lower)
	);
}

export function computeSuggestions(
	q: string,
	m: 'default' | 'tabs' | 'workspace' | 'search-engine',
	engine: SearchEngine | null,
	ctx: SuggestionContext
): CommandResult[] {
	// In search engine mode
	if (m === 'search-engine' && engine) {
		if (!q.trim()) {
			return [{
				id: 'search-prompt',
				type: 'search-engine',
				title: `Type to search ${engine.name}...`,
				subtitle: engine.searchUrl.replace('%s', '...'),
				icon: engine.icon,
				action: () => {},
				score: 1000
			}];
		}
		return [{
			id: 'search',
			type: 'search-engine',
			title: `Search ${engine.name} for "${q}"`,
			subtitle: engine.searchUrl.replace('%s', encodeURIComponent(q)),
			icon: engine.icon,
			url: engine.searchUrl.replace('%s', encodeURIComponent(q)),
			action: () => executeSearch(engine, q),
			score: 1000,
			hint: '↵'
		}];
	}

	// In tabs mode
	if (m === 'tabs') {
		return getTabSuggestions(q, ctx);
	}

	// In workspace mode
	if (m === 'workspace') {
		return getWorkspaceSuggestions(q, ctx);
	}

	// Default mode - combine everything as suggestions
	return getAllSuggestions(q, ctx);
}

function getAllSuggestions(q: string, ctx: SuggestionContext): CommandResult[] {
	const suggestions: CommandResult[] = [];
	const trimmed = q.trim().toLowerCase();
	const { browser } = ctx;

	// 1. COMMANDS - these should match first (e.g., "list t" -> "List Tabs")
	for (const cmd of COMMANDS) {
		const nameMatch = fuzzyScore(trimmed, cmd.name.toLowerCase());
		const aliasScores = cmd.aliases.map(a => fuzzyScore(trimmed, a).score);
		const bestScore = Math.max(nameMatch.score, ...aliasScores);

		if (!trimmed || bestScore > 20) {
			suggestions.push({
				id: `cmd-${cmd.id}`,
				type: 'command',
				title: cmd.name,
				subtitle: `Command`,
				icon: cmd.icon,
				action: cmd.action,
				score: trimmed ? bestScore + 500 : 50 // Boost commands when matching
			});
		}
	}

	// 2. SEARCH ENGINES - show "Press Tab to search X" for any partial match
	for (const engine of browser.searchEngines) {
		const keywordMatch = fuzzyScore(trimmed, engine.keyword.toLowerCase());
		const nameMatch = fuzzyScore(trimmed, engine.name.toLowerCase());
		const bestScore = Math.max(keywordMatch.score, nameMatch.score);

		// Show if query starts with keyword or name, or fuzzy matches
		const startsWithKeyword = engine.keyword.toLowerCase().startsWith(trimmed);
		const startsWithName = engine.name.toLowerCase().startsWith(trimmed);

		if (trimmed && (startsWithKeyword || startsWithName || bestScore > 30)) {
			suggestions.push({
				id: `engine-${engine.id}`,
				type: 'search-engine',
				title: `Search ${engine.name}`,
				subtitle: `Press Tab to search ${engine.name}`,
				icon: engine.icon,
				action: () => commandBarStore.setSearchEngine(engine),
				score: startsWithKeyword ? 900 : (startsWithName ? 850 : bestScore + 400),
				hint: 'Tab'
			});
		}
	}

	// 3. URL - if it looks like a URL
	if (isValidUrl(q.trim())) {
		suggestions.push({
			id: 'navigate-url',
			type: 'url',
			title: `Go to ${q.trim()}`,
			subtitle: 'Navigate to URL',
			icon: '🌐',
			url: normalizeUrl(q.trim()),
			action: () => navigateToUrl(q.trim()),
			score: 1000,
			hint: '↵'
		});
	}

	// 4. TAB SEARCH - fuzzy search open tabs
	if (trimmed) {
		const tabItems: SearchableItem[] = browser.tabs.map(tab => ({
			id: tab.id,
			title: tab.title,
			url: tab.url,
			content: tab.contentSnippets.join(' '),
			timestamp: tab.lastAccessed
		}));

		const matches = fuzzySearch(trimmed, tabItems, {
			minScore: 15,
			maxResults: 5,
			boostRecent: true
		});

		const tabsById = new Map(browser.tabs.map(t => [t.id, t]));
		for (const match of matches) {
			const tab = tabsById.get((match.item as SearchableItem).id)!;
			suggestions.push(createTabSuggestion(tab, match.score, ctx));
		}
	}

	// 5. HISTORY SEARCH
	if (trimmed) {
		const historyItems: SearchableItem[] = browser.history.slice(0, 50).map(h => ({
			id: h.url,
			title: h.title,
			url: h.url,
			timestamp: h.visitedAt
		}));

		const matches = fuzzySearch(trimmed, historyItems, {
			minScore: 25,
			maxResults: 3,
			boostRecent: true
		});

		for (const match of matches) {
			const item = match.item as SearchableItem;
			suggestions.push({
				id: `history-${item.id}`,
				type: 'history',
				title: item.title || item.url,
				subtitle: item.url,
				icon: '🕐',
				url: item.url,
				action: () => navigateToUrl(item.url),
				score: match.score * 0.5
			});
		}
	}

	// 6. BOOKMARK SEARCH
	if (trimmed) {
		const bookmarkItems: SearchableItem[] = browser.bookmarks.map(b => ({
			id: b.id,
			title: b.title,
			url: b.url,
			timestamp: b.createdAt
		}));

		const matches = fuzzySearch(trimmed, bookmarkItems, {
			minScore: 25,
			maxResults: 3
		});

		for (const match of matches) {
			const item = match.item as SearchableItem;
			suggestions.push({
				id: `bookmark-${item.id}`,
				type: 'bookmark',
				title: item.title,
				subtitle: item.url,
				icon: '⭐',
				url: item.url,
				action: () => navigateToUrl(item.url),
				score: match.score * 0.6
			});
		}
	}

	// 7. DEFAULT SEARCH - fallback to search Plates/Google
	if (trimmed) {
		suggestions.push({
			id: 'search-plates',
			type: 'search-engine',
			title: `Search for "${q.trim()}"`,
			subtitle: 'Search with Plates',
			icon: '🔍',
			action: () => searchPlates(q.trim()),
			score: 5,
			hint: '↵'
		});

		suggestions.push({
			id: 'search-lucky',
			type: 'command',
			title: `I'm Feeling Lucky: "${q.trim()}"`,
			subtitle: 'Open first result directly',
			icon: '🍀',
			action: () => navigateToUrl(getLuckySearchUrl(q.trim())),
			score: 4,
			hint: '⇧↵'
		});
	}

	// Sort by score and limit
	return suggestions.sort((a, b) => b.score - a.score).slice(0, 12);
}

function getTabSuggestions(q: string, ctx: SuggestionContext): CommandResult[] {
	const trimmed = q.trim().toLowerCase();
	const { browser } = ctx;

	if (!trimmed) {
		return browser.tabs
			.sort((a, b) => b.lastAccessed - a.lastAccessed)
			.map((tab, i) => createTabSuggestion(tab, 100 - i, ctx));
	}

	const items: SearchableItem[] = browser.tabs.map(tab => ({
		id: tab.id,
		title: tab.title,
		url: tab.url,
		content: tab.contentSnippets.join(' '),
		timestamp: tab.lastAccessed
	}));

	const matches = fuzzySearch(trimmed, items, {
		minScore: 5,
		maxResults: 20,
		boostRecent: true
	});

	const tabsById = new Map(browser.tabs.map(t => [t.id, t]));
	return matches.map(match => {
		const tab = tabsById.get((match.item as SearchableItem).id)!;
		return createTabSuggestion(tab, match.score, ctx);
	});
}

function getWorkspaceSuggestions(q: string, ctx: SuggestionContext): CommandResult[] {
	const trimmed = q.trim().toLowerCase();
	const { browser } = ctx;

	return browser.workspaces
		.map(ws => {
			const score = trimmed ? fuzzyScore(trimmed, ws.name.toLowerCase()).score : 100;
			return { ws, score };
		})
		.filter(({ score }) => !trimmed || score > 0)
		.sort((a, b) => b.score - a.score)
		.map(({ ws, score }) => createWorkspaceSuggestion(ws, score, ctx));
}

function createTabSuggestion(tab: BrowserTab, score: number, ctx: SuggestionContext): CommandResult {
	const { browser } = ctx;
	const ws = browser.workspaces.find(w => w.id === tab.workspaceId);
	return {
		id: `tab-${tab.id}`,
		type: 'tab',
		title: tab.title || 'Untitled',
		subtitle: `${ws?.icon || '📁'} ${ws?.name || 'Personal'} • ${tab.url}`,
		icon: tab.favicon || '🌐',
		url: tab.url,
		action: () => { browserStore.activateTab(tab.id); commandBarStore.close(); },
		score
	};
}

function createWorkspaceSuggestion(ws: Workspace, score: number, ctx: SuggestionContext): CommandResult {
	const { tabCounts } = ctx;
	const count = tabCounts[ws.id] || 0;
	return {
		id: `workspace-${ws.id}`,
		type: 'workspace',
		title: `${ws.icon} ${ws.name}`,
		subtitle: `${count} tab${count !== 1 ? 's' : ''}`,
		action: () => { browserStore.switchWorkspace(ws.id); commandBarStore.close(); },
		score
	};
}
