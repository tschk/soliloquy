<script lang="ts">
import '../app.css';
import { onMount } from 'svelte';
import { page } from '$app/stores';
import CommandPalette from '$lib/components/CommandPalette.svelte';
import { defaultCommandSuggestions, runSystemAction, type CommandSuggestion } from '$lib/system/actions';

const COMMAND_DISABLED_ROOTS = ['/onboarding'];
let commandPaletteOpen = false;
let suggestions: CommandSuggestion[] = defaultCommandSuggestions;

function isCommandsDisabled(pathname: string): boolean {
return COMMAND_DISABLED_ROOTS.some((root) =>
	root === pathname || (root !== '/' && pathname.startsWith(`${root}`))
);
}

$: currentPath = $page.url.pathname;
$: commandsDisabled = isCommandsDisabled(currentPath);
$: showCommandButton = !commandsDisabled && currentPath !== '/';

function toggleCommandPalette(force?: boolean) {
if (commandsDisabled) return;
commandPaletteOpen = force ?? !commandPaletteOpen;
}

async function handleCommandSelect(event: CustomEvent<{ suggestion: CommandSuggestion }>) {
const { suggestion } = event.detail;
await runSystemAction(suggestion.action);
toggleCommandPalette(false);
}

onMount(() => {
const keyHandler = (event: KeyboardEvent) => {
if (commandsDisabled) return;
if ((event.metaKey || event.ctrlKey) && event.key === '\\') {
event.preventDefault();
toggleCommandPalette();
}
};

const externalToggle = () => {
if (commandsDisabled) return;
toggleCommandPalette();
};

window.addEventListener('keydown', keyHandler);
window.addEventListener('soliloquy:command:toggle', externalToggle as EventListener);

return () => {
window.removeEventListener('keydown', keyHandler);
window.removeEventListener('soliloquy:command:toggle', externalToggle as EventListener);
};
});
</script>

<div class="relative min-h-screen bg-black text-white">
<slot />

{#if showCommandButton}
<button
class="command-button fixed bottom-6 left-6 z-40"
aria-label="Open command palette"
on:click={() => toggleCommandPalette()}
>
<span class="text-xs font-semibold uppercase tracking-[0.4em] text-white/60">Command</span>
<span class="text-white">⌘ \</span>
</button>
{/if}

<CommandPalette
open={commandPaletteOpen && !commandsDisabled}
suggestions={suggestions}
on:close={() => toggleCommandPalette(false)}
on:select={handleCommandSelect}
/>
</div>
