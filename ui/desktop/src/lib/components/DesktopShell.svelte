<script lang="ts">
	import { fade } from 'svelte/transition';
	import { onMount } from 'svelte';
	import { clockDisplay, systemClock } from '$lib/stores/system';
	import { batteryStore, weatherStore } from '$lib/stores/device';
	import { runSystemAction, type SystemAction } from '$lib/system/actions';
	import { performSearch, type SearchCard } from '$lib/api/search';
	import SearchBar from '$lib/components/SearchBar.svelte';
	import SearchCarousel from '$lib/components/SearchCarousel.svelte';
	import TerminalPane from '$lib/components/TerminalPane.svelte';

	type NavFilter = { label: string; action: SystemAction };

	const navFilters: NavFilter[] = [
		{ label: 'FILES', action: 'files.open' },
		{ label: 'CHATS', action: 'sessions.resume' },
		{ label: 'TABS', action: 'tabs.restore' },
		{ label: 'TERM', action: 'terminal.open' }
	];

	const fallbackPickup = {
		label: 'Desktop ready',
		title: 'Soliloquy booted into Servo',
		source: 'wlroots desktop',
		description: 'Use the command bar to search, navigate, or open the terminal.',
		actionUrl: undefined
	};

	const dateFormatter = new Intl.DateTimeFormat('en-US', {
		weekday: 'long',
		month: 'long',
		day: 'numeric'
	});

	let commandQuery = '';
	let searchLoading = false;
	let searchCards: SearchCard[] = [];
	let terminalOpen = false;
	let dayStamp = '';
	let heroStatus = '';

	$: dayStamp = dateFormatter.format($systemClock);
	$: heroStatus = `${$clockDisplay.time} ⋅ ${dayStamp}`;

	async function handleSearch(event: CustomEvent<string>) {
		const query = event.detail;
		searchLoading = true;
		searchCards = [];

		const response = await performSearch(query);
		searchLoading = false;

		if (response) {
			searchCards = response.cards;
		}
	}

	function handleSearchInput(_event: CustomEvent<string>) {
		// Reserved for future live suggestions.
	}

	function handleCardClick(card: SearchCard) {
		if (card.card_type === 'browser' && card.url) {
			window.open(card.url, '_blank', 'noopener');
		} else if (card.card_type === 'command') {
			console.info('[command]', card.metadata.command);
		} else if (card.card_type === 'cupboard') {
			console.info('[cupboard]', card.id);
		}
	}

	function toggleTerminal() {
		terminalOpen = !terminalOpen;
	}

	onMount(() => {
		const toggleTerminalListener = () => {
			toggleTerminal();
		};

		window.addEventListener('soliloquy:terminal:toggle', toggleTerminalListener);
		return () => window.removeEventListener('soliloquy:terminal:toggle', toggleTerminalListener);
	});
</script>

<main class="min-h-screen bg-black text-white">
	<header class="fixed right-6 top-5 z-10 flex items-center gap-5 text-sm sm:right-10">
		<span class="font-semibold">{heroStatus}</span>
		<span class="text-white/55">{$weatherStore.emoji} {$weatherStore.temp}°</span>
		<span class="text-white/55">{$batteryStore.level}%</span>
	</header>

	<aside class="fixed inset-y-0 left-0 z-10 flex w-28 flex-col justify-between px-5 py-6 text-white/50 transition hover:text-white/80">
		<nav class="grid gap-2 text-xs font-semibold uppercase">
			{#each navFilters as filter}
				<button
					type="button"
					class="text-left transition hover:text-white"
					on:click={() => runSystemAction(filter.action)}
				>
					{filter.label}
				</button>
			{/each}
		</nav>
		<button
			type="button"
			class="text-left text-xs font-semibold uppercase text-white/50 transition hover:text-white"
			on:click={toggleTerminal}
		>
			Terminal
		</button>
	</aside>

	<section class="grid min-h-screen place-items-center px-8 py-20 pl-32" transition:fade={{ duration: 220 }}>
		<div class="w-full max-w-5xl">
			<p class="mb-4 text-xl font-semibold text-white/90">Good day, Soliloquy</p>
			<SearchBar bind:value={commandQuery} loading={searchLoading} on:submit={handleSearch} on:input={handleSearchInput} />
			<SearchCarousel cards={searchCards} onCardClick={handleCardClick} />
		</div>
	</section>

	<aside class="fixed bottom-6 right-6 max-w-xs space-y-2 text-right text-white" transition:fade={{ delay: 120, duration: 240 }}>
		<p class="text-xs font-semibold uppercase text-white/45">{fallbackPickup.label}</p>
		<h3 class="text-xl font-semibold text-white/90">{fallbackPickup.title}</h3>
		{#if fallbackPickup.description}
			<p class="text-sm text-white/50">{fallbackPickup.description}</p>
		{/if}
		<p class="text-sm text-white/45">{fallbackPickup.source ?? 'Desktop mode'}</p>
		<button
			type="button"
			class="text-xs font-semibold uppercase text-white/60 underline underline-offset-4"
			on:click={toggleTerminal}
		>
			Open terminal
		</button>
	</aside>

	<TerminalPane open={terminalOpen} />
</main>
