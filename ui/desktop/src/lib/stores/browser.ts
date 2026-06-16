/**
 * Browser State Management for Soliloquy
 * 
 * No traditional tabs - everything controlled via command bar
 * - Workspaces for organization
 * - Fuzzy search across tabs and content
 * - Search engine integration with Tab-to-search
 */

import { writable, derived, get } from 'svelte/store';

// ============================================================================
// Types
// ============================================================================

export interface BrowserTab {
	id: string;
	url: string;
	title: string;
	favicon?: string;
	/** Indexed page content for search */
	contentSnippets: string[];
	/** Last access timestamp */
	lastAccessed: number;
	/** Creation timestamp */
	createdAt: number;
	/** Is this tab currently loading? */
	loading: boolean;
	/** Is this tab pinned? */
	pinned: boolean;
	/** Workspace this tab belongs to */
	workspaceId: string;
	/** Preview image (screenshot) */
	preview?: string;
}

export interface Workspace {
	id: string;
	name: string;
	icon: string;
	color: string;
	createdAt: number;
	/** Order in the workspace list */
	order: number;
}

export interface SearchEngine {
	id: string;
	name: string;
	keyword: string;
	searchUrl: string;
	suggestUrl?: string;
	icon?: string;
}

export interface CommandResult {
	id: string;
	type: 'tab' | 'workspace' | 'search-engine' | 'url' | 'command' | 'history' | 'bookmark';
	title: string;
	subtitle: string;
	icon?: string;
	url?: string;
	action: () => void;
	/** Relevance score for sorting (higher = more relevant) */
	score: number;
	/** Matched text ranges for highlighting */
	matches?: Array<{ start: number; end: number; field: 'title' | 'subtitle' }>;
	/** Keyboard hint */
	hint?: string;
}

export interface BrowserState {
	tabs: BrowserTab[];
	workspaces: Workspace[];
	activeTabId: string | null;
	activeWorkspaceId: string;
	searchEngines: SearchEngine[];
	history: HistoryEntry[];
	bookmarks: Bookmark[];
}

export interface HistoryEntry {
	url: string;
	title: string;
	visitedAt: number;
	visitCount: number;
}

export interface Bookmark {
	id: string;
	url: string;
	title: string;
	folderId?: string;
	createdAt: number;
}

// ============================================================================
// Default Data
// ============================================================================

const DEFAULT_WORKSPACES: Workspace[] = [
	{ id: 'default', name: 'Personal', icon: '🏠', color: '#3B82F6', createdAt: Date.now(), order: 0 },
	{ id: 'work', name: 'Work', icon: '💼', color: '#10B981', createdAt: Date.now(), order: 1 },
	{ id: 'research', name: 'Research', icon: '🔬', color: '#8B5CF6', createdAt: Date.now(), order: 2 },
];

const DEFAULT_SEARCH_ENGINES: SearchEngine[] = [
	{
		id: 'google',
		name: 'Google',
		keyword: 'google',
		searchUrl: 'https://www.google.com/search?q=%s',
		suggestUrl: 'https://suggestqueries.google.com/complete/search?client=firefox&q=%s',
		icon: '🔍'
	},
	{
		id: 'duckduckgo',
		name: 'DuckDuckGo',
		keyword: 'ddg',
		searchUrl: 'https://duckduckgo.com/?q=%s',
		icon: '🦆'
	},
	{
		id: 'youtube',
		name: 'YouTube',
		keyword: 'yt',
		searchUrl: 'https://www.youtube.com/results?search_query=%s',
		icon: '▶️'
	},
	{
		id: 'github',
		name: 'GitHub',
		keyword: 'gh',
		searchUrl: 'https://github.com/search?q=%s',
		icon: '🐙'
	},
	{
		id: 'wikipedia',
		name: 'Wikipedia',
		keyword: 'wiki',
		searchUrl: 'https://en.wikipedia.org/wiki/Special:Search?search=%s',
		icon: '📚'
	},
	{
		id: 'amazon',
		name: 'Amazon',
		keyword: 'amz',
		searchUrl: 'https://www.amazon.com/s?k=%s',
		icon: '📦'
	},
	{
		id: 'reddit',
		name: 'Reddit',
		keyword: 'r',
		searchUrl: 'https://www.reddit.com/search/?q=%s',
		icon: '🔴'
	},
	{
		id: 'twitter',
		name: 'X/Twitter',
		keyword: 'x',
		searchUrl: 'https://twitter.com/search?q=%s',
		icon: '𝕏'
	},
	{
		id: 'maps',
		name: 'Google Maps',
		keyword: 'maps',
		searchUrl: 'https://www.google.com/maps/search/%s',
		icon: '🗺️'
	},
	{
		id: 'npm',
		name: 'npm',
		keyword: 'npm',
		searchUrl: 'https://www.npmjs.com/search?q=%s',
		icon: '📦'
	},
];

// ============================================================================
// Action Creators
// ============================================================================

function createTabActions(update: (updater: (state: BrowserState) => BrowserState) => void, subscribe: (run: (value: BrowserState) => void) => () => void) {
	return {
		openTab: (url: string, options?: { workspaceId?: string; activate?: boolean }) => {
			const tabId = crypto.randomUUID();
			const workspaceId = options?.workspaceId ?? get({ subscribe }).activeWorkspaceId;
			
			update(state => {
				const newTab: BrowserTab = {
					id: tabId,
					url,
					title: url,
					contentSnippets: [],
					lastAccessed: Date.now(),
					createdAt: Date.now(),
					loading: true,
					pinned: false,
					workspaceId,
				};
				
				return {
					...state,
					tabs: [...state.tabs, newTab],
					activeTabId: options?.activate !== false ? tabId : state.activeTabId,
				};
			});
			
			return tabId;
		},
		
		closeTab: (tabId: string) => {
			update(state => {
				const tabs = state.tabs.filter(t => t.id !== tabId);
				let activeTabId = state.activeTabId;
				
				// If we closed the active tab, activate another one
				if (activeTabId === tabId) {
					const workspaceTabs = tabs.filter(t => t.workspaceId === state.activeWorkspaceId);
					activeTabId = workspaceTabs.length > 0 
						? workspaceTabs[workspaceTabs.length - 1].id 
						: null;
				}
				
				return { ...state, tabs, activeTabId };
			});
		},
		
		activateTab: (tabId: string) => {
			update(state => {
				const tab = state.tabs.find(t => t.id === tabId);
				if (!tab) return state;
				
				return {
					...state,
					activeTabId: tabId,
					activeWorkspaceId: tab.workspaceId,
					tabs: state.tabs.map(t => 
						t.id === tabId 
							? { ...t, lastAccessed: Date.now() }
							: t
					),
				};
			});
		},
		
		updateTab: (tabId: string, updates: Partial<BrowserTab>) => {
			update(state => ({
				...state,
				tabs: state.tabs.map(t => 
					t.id === tabId ? { ...t, ...updates } : t
				),
			}));
		},
		
		moveTabToWorkspace: (tabId: string, workspaceId: string) => {
			update(state => ({
				...state,
				tabs: state.tabs.map(t => 
					t.id === tabId ? { ...t, workspaceId } : t
				),
			}));
		},
		
		pinTab: (tabId: string, pinned = true) => {
			update(state => ({
				...state,
				tabs: state.tabs.map(t => 
					t.id === tabId ? { ...t, pinned } : t
				),
			}));
		}
	};
}

function createWorkspaceActions(update: (updater: (state: BrowserState) => BrowserState) => void) {
	return {
		createWorkspace: (name: string, icon = '📁', color = '#6B7280') => {
			const id = crypto.randomUUID();
			update(state => ({
				...state,
				workspaces: [...state.workspaces, {
					id,
					name,
					icon,
					color,
					createdAt: Date.now(),
					order: state.workspaces.length,
				}],
			}));
			return id;
		},
		
		deleteWorkspace: (workspaceId: string) => {
			if (workspaceId === 'default') return; // Can't delete default
			
			update(state => {
				// Move tabs to default workspace
				const tabs = state.tabs.map(t => 
					t.workspaceId === workspaceId 
						? { ...t, workspaceId: 'default' }
						: t
				);
				
				return {
					...state,
					tabs,
					workspaces: state.workspaces.filter(w => w.id !== workspaceId),
					activeWorkspaceId: state.activeWorkspaceId === workspaceId 
						? 'default' 
						: state.activeWorkspaceId,
				};
			});
		},
		
		switchWorkspace: (workspaceId: string) => {
			update(state => {
				const workspaceTabs = state.tabs.filter(t => t.workspaceId === workspaceId);
				const mostRecent = workspaceTabs.sort((a, b) => b.lastAccessed - a.lastAccessed)[0];
				
				return {
					...state,
					activeWorkspaceId: workspaceId,
					activeTabId: mostRecent?.id ?? state.activeTabId,
				};
			});
		},
		
		updateWorkspace: (workspaceId: string, updates: Partial<Workspace>) => {
			update(state => ({
				...state,
				workspaces: state.workspaces.map(w => 
					w.id === workspaceId ? { ...w, ...updates } : w
				),
			}));
		}
	};
}

function createHistoryActions(update: (updater: (state: BrowserState) => BrowserState) => void) {
	return {
		addToHistory: (url: string, title: string) => {
			update(state => {
				const existing = state.history.find(h => h.url === url);
				if (existing) {
					return {
						...state,
						history: state.history.map(h => 
							h.url === url 
								? { ...h, title, visitedAt: Date.now(), visitCount: h.visitCount + 1 }
								: h
						),
					};
				}
				
				return {
					...state,
					history: [
						{ url, title, visitedAt: Date.now(), visitCount: 1 },
						...state.history.slice(0, 999), // Keep last 1000
					],
				};
			});
		},
		
		clearHistory: () => {
			update(state => ({ ...state, history: [] }));
		}
	};
}

function createBookmarkActions(update: (updater: (state: BrowserState) => BrowserState) => void) {
	return {
		addBookmark: (url: string, title: string, folderId?: string) => {
			const id = crypto.randomUUID();
			update(state => ({
				...state,
				bookmarks: [...state.bookmarks, {
					id,
					url,
					title,
					folderId,
					createdAt: Date.now(),
				}],
			}));
			return id;
		},
		
		removeBookmark: (bookmarkId: string) => {
			update(state => ({
				...state,
				bookmarks: state.bookmarks.filter(b => b.id !== bookmarkId),
			}));
		}
	};
}

// ============================================================================
// Stores
// ============================================================================

const INITIAL_BROWSER_STATE: BrowserState = {
	tabs: [],
	workspaces: DEFAULT_WORKSPACES,
	activeTabId: null,
	activeWorkspaceId: 'default',
	searchEngines: DEFAULT_SEARCH_ENGINES,
	history: [],
	bookmarks: [],
};

function createBrowserStore() {
	const { subscribe, set, update } = writable<BrowserState>(INITIAL_BROWSER_STATE);

	return {
		subscribe,
		...createTabActions(update, subscribe),
		...createWorkspaceActions(update),
		...createHistoryActions(update),
		...createBookmarkActions(update),
		reset: () => set(INITIAL_BROWSER_STATE),
	};
}

export const browserStore = createBrowserStore();

// ============================================================================
// Derived Stores
// ============================================================================

/** Tabs in the current workspace */
export const workspaceTabs = derived(browserStore, $browser => 
	$browser.tabs
		.filter(t => t.workspaceId === $browser.activeWorkspaceId)
		.sort((a, b) => {
			// Pinned tabs first, then by last accessed
			if (a.pinned && !b.pinned) return -1;
			if (!a.pinned && b.pinned) return 1;
			return b.lastAccessed - a.lastAccessed;
		})
);

/** Currently active tab */
export const activeTab = derived(browserStore, $browser => 
	$browser.tabs.find(t => t.id === $browser.activeTabId) ?? null
);

/** Current workspace */
export const activeWorkspace = derived(browserStore, $browser => 
	$browser.workspaces.find(w => w.id === $browser.activeWorkspaceId) ?? $browser.workspaces[0]
);

/** Tab count per workspace */
export const workspaceTabCounts = derived(browserStore, $browser => {
	const counts: Record<string, number> = {};
	for (const workspace of $browser.workspaces) {
		counts[workspace.id] = $browser.tabs.filter(t => t.workspaceId === workspace.id).length;
	}
	return counts;
});

// ============================================================================
// Command Bar State
// ============================================================================

export interface CommandBarState {
	open: boolean;
	query: string;
	selectedIndex: number;
	mode: 'default' | 'search-engine' | 'workspace' | 'tabs';
	selectedEngine: SearchEngine | null;
}

function createCommandBarStore() {
	const { subscribe, set, update } = writable<CommandBarState>({
		open: false,
		query: '',
		selectedIndex: 0,
		mode: 'default',
		selectedEngine: null,
	});

	return {
		subscribe,
		
		open: () => update(state => ({ ...state, open: true, query: '', selectedIndex: 0 })),
		close: () => update(state => ({ ...state, open: false, query: '', selectedIndex: 0, mode: 'default', selectedEngine: null })),
		toggle: () => update(state => ({ ...state, open: !state.open, query: state.open ? '' : state.query })),
		
		setQuery: (query: string) => update(state => ({ ...state, query, selectedIndex: 0 })),
		setSelectedIndex: (index: number) => update(state => ({ ...state, selectedIndex: index })),
		
		setMode: (mode: CommandBarState['mode']) => update(state => ({ ...state, mode })),
		setSearchEngine: (engine: SearchEngine | null) => update(state => ({ 
			...state, 
			selectedEngine: engine,
			mode: engine ? 'search-engine' : 'default',
			query: '', // Clear query when entering search engine mode
		})),
		
		reset: () => set({
			open: false,
			query: '',
			selectedIndex: 0,
			mode: 'default',
			selectedEngine: null,
		}),
	};
}

export const commandBarStore = createCommandBarStore();
