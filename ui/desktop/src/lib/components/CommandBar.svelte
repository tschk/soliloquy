<script lang="ts">
	import { fade, fly } from 'svelte/transition';
	import { onMount, tick } from 'svelte';
	import { 
		browserStore, 
		commandBarStore, 
		workspaceTabs,
		activeWorkspace,
		workspaceTabCounts,
		type BrowserTab,
		type SearchEngine,
		type CommandResult,
		type Workspace
	} from '$lib/stores/browser';
	import { 
		getLuckySearchUrl
	} from '$lib/utils/fuzzy';
	import { isValidUrl, normalizeUrl } from '$lib/system/actions';
	import {
		computeSuggestions,
		findMatchingEngines,
		navigateToUrl,
		searchPlates
	} from './suggestions';


	// =========================================================================
	// State
	// =========================================================================
	
	let dialogRef: HTMLDivElement | null = null;
	let inputRef: HTMLInputElement | null = null;
	let resultsRef: HTMLDivElement | null = null;
	
	$: ({ open, query, selectedIndex, mode, selectedEngine } = $commandBarStore);
	$: browser = $browserStore;
	$: currentWorkspace = $activeWorkspace;
	$: allTabs = browser.tabs;
	$: tabCounts = $workspaceTabCounts;
	
	// All suggestions powered by one function
	$: suggestions = computeSuggestions(query, mode, selectedEngine, { browser, tabCounts });
	$: isUrl = isValidUrl(query.trim());
	
	// Detect if query starts with a search engine keyword
	$: matchingEngines = findMatchingEngines(query, browser.searchEngines);

	// =========================================================================
	// Actions
	// =========================================================================

	function executeSuggestion(suggestion: CommandResult) {
		suggestion.action();
	}
	
	function selectTab(tab: BrowserTab) {
		browserStore.activateTab(tab.id);
		commandBarStore.close();
	}
	
	function closeTabFromList(tabId: string, event: Event) {
		event.stopPropagation();
		browserStore.closeTab(tabId);
	}

	// =========================================================================
	// Helpers
	// =========================================================================
	
	function getIconBgClass(type: string): string {
		switch (type) {
			case 'tab': return 'bg-white/5';
			case 'search-engine': return 'bg-blue-500/20';
			case 'workspace': return 'bg-purple-500/20';
			case 'bookmark': return 'bg-amber-500/20';
			case 'history': return 'bg-gray-500/20';
			case 'url': return 'bg-green-500/20';
			case 'command': return 'bg-cyan-500/20';
			default: return 'bg-white/5';
		}
	}
	
	function getBadgeClasses(type: string): string {
		switch (type) {
			case 'tab': return 'bg-white/5 text-white/40';
			case 'search-engine': return 'bg-blue-500/20 text-blue-300';
			case 'workspace': return 'bg-purple-500/20 text-purple-300';
			case 'bookmark': return 'bg-amber-500/20 text-amber-300';
			case 'history': return 'bg-gray-500/20 text-gray-300';
			case 'url': return 'bg-green-500/20 text-green-300';
			case 'command': return 'bg-cyan-500/20 text-cyan-300';
			default: return 'bg-white/5 text-white/40';
		}
	}
	
	// =========================================================================
	// Keyboard Handling
	// =========================================================================
	
	function handleKeyDown(event: KeyboardEvent) {
		if (!open) return;
		
		switch (event.key) {
			case 'Escape':
				event.preventDefault();
				if (mode !== 'default' || selectedEngine) {
					commandBarStore.setMode('default');
					commandBarStore.setSearchEngine(null);
				} else {
					commandBarStore.close();
				}
				break;
				
			case 'Tab':
				event.preventDefault();
				// If selected suggestion is a search engine, activate it
				const selected = suggestions[selectedIndex];
				if (selected?.type === 'search-engine' && selected.id.startsWith('engine-')) {
					const engineId = selected.id.replace('engine-', '');
					const engine = browser.searchEngines.find(e => e.id === engineId);
					if (engine) commandBarStore.setSearchEngine(engine);
				}
				break;
				
			case 'ArrowDown':
				event.preventDefault();
				commandBarStore.setSelectedIndex(Math.min(selectedIndex + 1, suggestions.length - 1));
				scrollToSelected();
				break;
				
			case 'ArrowUp':
				event.preventDefault();
				commandBarStore.setSelectedIndex(Math.max(selectedIndex - 1, 0));
				scrollToSelected();
				break;
				
			case 'Enter':
				event.preventDefault();
				if (event.shiftKey && query.trim()) {
					navigateToUrl(getLuckySearchUrl(query.trim()));
				} else if (suggestions.length > 0) {
					executeSuggestion(suggestions[selectedIndex]);
				} else if (query.trim()) {
					searchPlates(query.trim());
				}
				break;
				
			case 'Backspace':
				if (!query && selectedEngine) {
					commandBarStore.setSearchEngine(null);
					commandBarStore.setMode('default');
				}
				break;
		}
	}
	
	async function scrollToSelected() {
		await tick();
		const selected = resultsRef?.querySelector('[data-selected="true"]');
		selected?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
	}
	
	// =========================================================================
	// Lifecycle
	// =========================================================================
	
	onMount(() => {
		const globalKeyHandler = (event: KeyboardEvent) => {
			if ((event.metaKey || event.ctrlKey) && (event.key === 'k' || event.key === '\\')) {
				event.preventDefault();
				commandBarStore.toggle();
			}
		};
		window.addEventListener('keydown', globalKeyHandler);
		return () => window.removeEventListener('keydown', globalKeyHandler);
	});
	
	$: if (open) {
		tick().then(() => inputRef?.focus());
	}
	
	function handleClickOutside(event: PointerEvent) {
		if (!open || !dialogRef) return;
		if (!dialogRef.contains(event.target as Node)) {
			commandBarStore.close();
		}
	}
</script>

<svelte:window on:keydown={handleKeyDown} on:pointerdown={handleClickOutside} />

{#if open}
	<div 
		class="fixed inset-0 z-50 flex items-start justify-center bg-black/80 px-4 backdrop-blur-sm"
		transition:fade={{ duration: 150 }}
	>
		<div
			class="glass-panel mt-20 w-full max-w-2xl overflow-hidden shadow-2xl"
			role="dialog"
			aria-modal="true"
			aria-label="Command bar"
			tabindex="-1"
			bind:this={dialogRef}
			transition:fly={{ y: -20, duration: 200 }}
		>
			<!-- Input Area -->
			<div class="flex items-center gap-3 border-b border-white/10 px-5 py-4">
				<!-- Mode indicator -->
				{#if selectedEngine}
					<button
						type="button"
						class="flex items-center gap-1.5 rounded-md bg-white/10 px-2.5 py-1 text-sm font-medium text-white transition hover:bg-white/20"
						on:click={() => {
							commandBarStore.setSearchEngine(null);
							commandBarStore.setMode('default');
						}}
					>
						<span>{selectedEngine.icon}</span>
						<span>{selectedEngine.name}</span>
						<span class="ml-1 text-white/50">×</span>
					</button>
				{:else if mode === 'tabs'}
					<button
						type="button"
						class="flex items-center gap-1.5 rounded-md bg-blue-500/20 px-2.5 py-1 text-sm font-medium text-blue-300 transition hover:bg-blue-500/30"
						on:click={() => commandBarStore.setMode('default')}
					>
						<span>📑</span>
						<span>Tabs</span>
						<span class="ml-1 text-blue-300/50">×</span>
					</button>
				{:else if mode === 'workspace'}
					<button
						type="button"
						class="flex items-center gap-1.5 rounded-md bg-purple-500/20 px-2.5 py-1 text-sm font-medium text-purple-300 transition hover:bg-purple-500/30"
						on:click={() => commandBarStore.setMode('default')}
					>
						<span>📁</span>
						<span>Workspaces</span>
						<span class="ml-1 text-purple-300/50">×</span>
					</button>
				{:else}
					<span class="text-xs font-semibold uppercase tracking-[0.25em] text-white/40">⌘K</span>
				{/if}
				
				<input
					type="text"
					placeholder={selectedEngine 
						? `Search ${selectedEngine.name}...` 
						: mode === 'tabs'
							? 'Search tabs...'
							: 'Search tabs, history, or enter URL...'
					}
					value={query}
					on:input={(e) => commandBarStore.setQuery(e.currentTarget.value)}
					bind:this={inputRef}
					class="flex-1 bg-transparent text-lg text-white placeholder-white/40 focus:outline-none"
				/>
				
				<button
					type="button"
					class="rounded px-2 py-1 text-xs font-semibold uppercase tracking-wider text-white/40 transition hover:bg-white/10 hover:text-white/60"
					on:click={() => commandBarStore.close()}
				>
					Esc
				</button>
			</div>
			
			<!-- Main Content: Tab List + Suggestions -->
			<div class="flex max-h-[60vh]">
				<!-- Left: Vertical Tab List -->
				<div class="w-48 shrink-0 overflow-y-auto border-r border-white/10 bg-white/[0.02]">
					<div class="px-3 py-2">
						<p class="text-[10px] font-semibold uppercase tracking-wider text-white/40">Open Tabs</p>
					</div>
					{#if allTabs.length === 0}
						<div class="px-3 py-4 text-center text-xs text-white/30">No tabs open</div>
					{:else}
						{#each allTabs.slice(0, 15) as tab (tab.id)}
							{@const isActive = tab.id === browser.activeTabId}
							<button
								type="button"
								class="group flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition {isActive ? 'bg-white/10 text-white' : 'text-white/60 hover:bg-white/5 hover:text-white'}"
								on:click={() => selectTab(tab)}
							>
								<span class="shrink-0">{tab.favicon || '🌐'}</span>
								<span class="flex-1 truncate">{tab.title || 'Untitled'}</span>
								<span
									role="button"
									tabindex="0"
									class="shrink-0 opacity-0 transition hover:text-red-400 group-hover:opacity-100"
									on:click={(e) => closeTabFromList(tab.id, e)}
									on:keydown={(e) => e.key === 'Enter' && closeTabFromList(tab.id, e)}
								>×</span>
							</button>
						{/each}
						{#if allTabs.length > 15}
							<div class="px-3 py-2 text-center text-[10px] text-white/30">
								+{allTabs.length - 15} more
							</div>
						{/if}
					{/if}
				</div>
				
				<!-- Right: Suggestions -->
				<div class="flex-1 overflow-y-auto" bind:this={resultsRef}>
					{#if suggestions.length === 0}
						<div class="px-5 py-10 text-center">
							<p class="text-sm text-white/50">
								{#if query.trim()}
									No matches. Press Enter to search.
								{:else}
									Type a command, search, or URL...
								{/if}
							</p>
						</div>
					{:else}
						{#each suggestions as suggestion, i (suggestion.id)}
							<button
								type="button"
								class="group flex w-full items-center gap-3 border-b border-white/5 px-4 py-3 text-left transition {i === selectedIndex ? 'bg-white/10' : 'hover:bg-white/5'}"
								data-selected={i === selectedIndex}
								on:click={() => executeSuggestion(suggestion)}
								on:mouseenter={() => commandBarStore.setSelectedIndex(i)}
							>
								<span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-base {getIconBgClass(suggestion.type)}">
									{suggestion.icon || '🔗'}
								</span>
								
								<div class="min-w-0 flex-1">
									<p class="truncate text-sm font-medium text-white">{suggestion.title}</p>
									<p class="truncate text-xs text-white/50">{suggestion.subtitle}</p>
								</div>
								
								<span class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium uppercase {getBadgeClasses(suggestion.type)}">
									{suggestion.type}
								</span>
								
								{#if suggestion.hint && i === selectedIndex}
									<span class="shrink-0 text-xs font-medium text-white/40">{suggestion.hint}</span>
								{/if}
							</button>
						{/each}
					{/if}
				</div>
			</div>
			
			<!-- Footer -->
			<div class="flex items-center justify-between border-t border-white/10 bg-white/5 px-4 py-2 text-[10px] text-white/40">
				<div class="flex items-center gap-3">
					<span><kbd class="rounded bg-white/10 px-1 py-0.5 font-mono">↑↓</kbd> Navigate</span>
					<span><kbd class="rounded bg-white/10 px-1 py-0.5 font-mono">↵</kbd> Open</span>
					<span><kbd class="rounded bg-white/10 px-1 py-0.5 font-mono">Tab</kbd> Search</span>
					<span><kbd class="rounded bg-white/10 px-1 py-0.5 font-mono">⇧↵</kbd> Lucky</span>
				</div>
				<div class="flex items-center gap-2">
					<span>{currentWorkspace.icon} {currentWorkspace.name}</span>
					<span>•</span>
					<span>{browser.tabs.length} tabs</span>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.glass-panel {
		background: rgba(20, 20, 25, 0.95);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		box-shadow: 
			0 25px 50px -12px rgba(0, 0, 0, 0.6),
			0 0 0 1px rgba(255, 255, 255, 0.05) inset;
	}
	
	kbd {
		font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, 'Courier New', monospace;
	}
</style>
