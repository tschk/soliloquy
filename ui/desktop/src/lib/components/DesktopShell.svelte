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

<main class="flex min-h-screen flex-col bg-black text-white">
	<header class="flex items-center justify-between px-6 py-8 sm:px-12 lg:px-24">
		<nav class="flex gap-8 text-sm font-semibold uppercase tracking-[0.4em]">
			{#each navFilters as filter}
				<button
					type="button"
					class="text-white/50 transition hover:text-white"
					on:click={() => runSystemAction(filter.action)}
				>
					{filter.label}
				</button>
			{/each}
		</nav>

		<div class="flex items-center gap-6 text-sm">
			<span class="font-semibold">{heroStatus}</span>
			<span class="text-white/60">{$weatherStore.emoji} {$weatherStore.temp}°</span>
			<span class="text-white/60">{$batteryStore.level}%</span>
			<button
				type="button"
				class="rounded-full border border-white/15 bg-white/5 px-4 py-2 text-xs font-semibold uppercase tracking-[0.4em] text-white/70 hover:bg-white/10"
				on:click={toggleTerminal}
			>
				Terminal
			</button>
		</div>
	</header>

	<section class="flex-1 px-6 py-12 sm:px-12 lg:px-24" transition:fade={{ duration: 220 }}>
		<div class="space-y-4">
			<p class="text-xs font-semibold uppercase tracking-[0.5em] text-white/40">Soliloquy desktop</p>
			<h1 class="text-4xl font-semibold text-white/90 md:text-6xl">One browser, one shell.</h1>
			<p class="max-w-3xl text-base text-white/60 md:text-lg">
				Servo hosts the desktop surface, the command bar drives navigation, and the terminal opens
				on demand through the sold PTY bridge.
			</p>
		</div>

		<div class="mt-10">
			<SearchBar bind:value={commandQuery} loading={searchLoading} on:submit={handleSearch} on:input={handleSearchInput} />
			<SearchCarousel cards={searchCards} onCardClick={handleCardClick} />
		</div>
	</section>

	<aside class="fixed bottom-6 left-6 space-y-3 text-white" transition:fade={{ delay: 120, duration: 240 }}>
		<p class="text-xs font-semibold uppercase tracking-[0.4em] text-white/50">{fallbackPickup.label}</p>
		<h3 class="text-2xl font-semibold text-white/90">{fallbackPickup.title}</h3>
		{#if fallbackPickup.description}
			<p class="text-sm text-white/60">{fallbackPickup.description}</p>
		{/if}
		<p class="text-base text-white/60">{fallbackPickup.source ?? 'Desktop mode'}</p>
		<button
			type="button"
			class="text-xs font-semibold uppercase tracking-[0.4em] text-white/60 underline underline-offset-4"
			on:click={toggleTerminal}
		>
			Open terminal
		</button>
	</aside>

	<TerminalPane open={terminalOpen} />
</main>
