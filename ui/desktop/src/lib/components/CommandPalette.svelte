<script lang="ts">
import { fade } from 'svelte/transition';
import { createEventDispatcher, onMount } from 'svelte';
import type { CommandSuggestion } from '$lib/system/actions';
import { isValidUrl, setPendingNavigation } from '$lib/system/actions';

type PaletteEvents = {
close: void;
select: { suggestion: CommandSuggestion };
};

export let open = false;
export let suggestions: CommandSuggestion[] = [];

const dispatch = createEventDispatcher<PaletteEvents>();
let query = '';
let dialogRef: HTMLDivElement | null = null;
let inputRef: HTMLInputElement | null = null;

$: isUrlQuery = isValidUrl(query.trim());

$: preparedSuggestions = suggestions.map((suggestion) => ({
	suggestion,
	lowerTitle: suggestion.title.toLowerCase(),
	lowerDescription: suggestion.description.toLowerCase()
}));

$: filteredSuggestions = (() => {
	const trimmedQuery = query.trim();
	if (!trimmedQuery) return suggestions;

	const needle = trimmedQuery.toLowerCase();
	return preparedSuggestions
		.filter(({ lowerTitle, lowerDescription }) => {
			return lowerTitle.includes(needle) || lowerDescription.includes(needle);
		})
		.map(({ suggestion }) => suggestion);
})();

function closePalette() {
query = '';
dispatch('close');
}

function handleSelect(suggestion: CommandSuggestion) {
dispatch('select', { suggestion });
}

function handleNavigateToUrl() {
if (!isUrlQuery) return;
const url = query.trim();
setPendingNavigation(url);
dispatch('select', { 
	suggestion: { 
		title: `Go to ${url}`, 
		description: 'Navigate to URL', 
		action: 'navigate.url' 
	} 
});
}

function handleKeyDown(event: KeyboardEvent) {
if (event.key === 'Enter' && isUrlQuery) {
	event.preventDefault();
	handleNavigateToUrl();
}
}

onMount(() => {
const keyHandler = (event: KeyboardEvent) => {
if (event.key === 'Escape' && open) {
closePalette();
}
};

const pointerHandler = (event: PointerEvent) => {
if (!open || !dialogRef) return;
if (!dialogRef.contains(event.target as Node)) {
closePalette();
}
};

const pointerOptions: AddEventListenerOptions = { capture: true };

window.addEventListener('keydown', keyHandler);
window.addEventListener('pointerdown', pointerHandler, pointerOptions);
return () => {
window.removeEventListener('keydown', keyHandler);
window.removeEventListener('pointerdown', pointerHandler, pointerOptions);
};
});
</script>

{#if open}
<div class="fixed inset-0 z-50 flex items-start justify-center bg-black/80 px-4 backdrop-blur" transition:fade={{ duration: 160 }}>
<div
class="glass-panel w-full max-w-2xl mt-28 overflow-hidden"
role="dialog"
aria-modal="true"
aria-label="Soliloquy command palette"
tabindex="-1"
bind:this={dialogRef}
on:introend={() => dialogRef?.focus()}
>
<div class="flex items-center gap-3 border-b border-white/10 px-6 py-4">
<span class="text-xs font-semibold uppercase tracking-[0.3em] text-white/60">⌘ \</span>
<input
type="text"
placeholder="Enter a URL or search commands..."
bind:value={query}
bind:this={inputRef}
on:keydown={handleKeyDown}
class="flex-1 bg-transparent text-lg text-white placeholder-white/40 focus:outline-none"
/>
<button
type="button"
class="text-xs font-semibold uppercase tracking-[0.3em] text-white/50 transition hover:text-white"
on:click={closePalette}
>
Esc
</button>
</div>
<div class="max-h-80 overflow-y-auto">
{#if isUrlQuery}
<button
type="button"
class="w-full border-b border-white/5 px-6 py-5 text-left transition bg-white/5 hover:bg-white/10"
on:click={handleNavigateToUrl}
>
<div class="flex items-center justify-between gap-4">
<div>
<p class="text-base font-semibold text-white">Go to {query.trim()}</p>
<p class="text-sm text-white/60">Navigate to this URL</p>
</div>
<span class="text-xs font-semibold uppercase tracking-[0.3em] text-white/50">
↵ Enter
</span>
</div>
</button>
{/if}
{#if filteredSuggestions.length === 0 && !isUrlQuery}
<div class="px-6 py-10 text-center text-sm text-white/50">
No matches. Try another query.
</div>
{:else}
{#each filteredSuggestions as suggestion}
<button
type="button"
class="w-full border-b border-white/5 px-6 py-5 text-left transition hover:bg-white/5"
on:click={() => handleSelect(suggestion)}
>
<div class="flex items-center justify-between gap-4">
<div>
<p class="text-base font-semibold text-white">{suggestion.title}</p>
<p class="text-sm text-white/60">{suggestion.description}</p>
</div>
{#if suggestion.shortcut}
<span class="text-xs font-semibold uppercase tracking-[0.3em] text-white/50">
{suggestion.shortcut}
</span>
{/if}
</div>
</button>
{/each}
{/if}
</div>
</div>
</div>
{/if}
