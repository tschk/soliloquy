<script lang="ts">
	import { fade } from 'svelte/transition';
	import { onMount } from 'svelte';
	import {
		ArrowLeft,
		ArrowRight,
		Columns2,
		Folder,
		Grid2X2,
		PanelLeft,
		PanelTop,
		Plus,
		RefreshCw,
		Rows3,
		Search,
		Settings,
		Square,
		Terminal,
		X
	} from '@lucide/svelte';
	import { clockDisplay, systemClock } from '$lib/stores/system';
	import { batteryStore, weatherStore } from '$lib/stores/device';
	import TerminalPane from '$lib/components/TerminalPane.svelte';
	import { browserStore, activeTab, workspaceTabs, activeWorkspace } from '$lib/stores/browser';

	type ChromeRoute = {
		label: string;
		url: string;
		icon: typeof Terminal;
	};

	type BrowserMode = 'zen' | 'compact' | 'split-horizontal' | 'split-vertical' | 'grid';

	type BrowserModeOption = {
		id: BrowserMode;
		label: string;
		icon: typeof Square;
	};

	const chromeRoutes: ChromeRoute[] = [
		{ label: 'Terminal', url: 'os://terminal', icon: Terminal },
		{ label: 'Files', url: 'os://files', icon: Folder },
		{ label: 'Settings', url: 'os://settings', icon: Settings }
	];

	const browserModes: BrowserModeOption[] = [
		{ id: 'zen', label: 'Zen', icon: PanelLeft },
		{ id: 'compact', label: 'Compact', icon: PanelTop },
		{ id: 'split-horizontal', label: 'Split columns', icon: Columns2 },
		{ id: 'split-vertical', label: 'Split rows', icon: Rows3 },
		{ id: 'grid', label: 'Grid', icon: Grid2X2 }
	];

	const dateFormatter = new Intl.DateTimeFormat('en-US', {
		weekday: 'short',
		month: 'short',
		day: 'numeric'
	});

	const routeTargets: Record<string, string> = {
		'os://terminal': '/terminal',
		'os://files': '/files.html',
		'os://settings': '/settings.html'
	};

	let addressValue = 'os://terminal';
	let terminalOpen = false;
	let navHistory: string[] = ['os://terminal'];
	let navIndex = 0;
	let frameLoading = true;
	let dayStamp = '';
	let heroStatus = '';
	let browserMode: BrowserMode = 'zen';
	let frameRefreshToken = 0;

	$: currentTab = $activeTab;
	$: tabs = $workspaceTabs;
	$: workspace = $activeWorkspace;
	$: visibleTabs = currentTab ? [currentTab, ...tabs.filter((tab) => tab.id !== currentTab?.id)] : tabs;
	$: paneTabs = browserMode === 'grid'
		? visibleTabs.slice(0, 4)
		: browserMode === 'split-horizontal' || browserMode === 'split-vertical'
			? visibleTabs.slice(0, 2)
			: visibleTabs.slice(0, 1);
	$: hasSidebar = browserMode !== 'compact';
	$: dayStamp = dateFormatter.format($systemClock);
	$: heroStatus = `${$clockDisplay.time} / ${dayStamp}`;
	$: if (currentTab && addressValue !== currentTab.url) {
		addressValue = currentTab.url;
	}
	$: if (currentTab && isInlineTerminal(currentTab.url) && frameLoading) {
		frameLoading = false;
	}

	function titleFor(url: string) {
		if (url === 'os://terminal') return 'Terminal';
		if (url === 'os://files') return 'Files';
		if (url === 'os://settings') return 'Settings';
		try {
			const parsed = new URL(url);
			return parsed.hostname || url;
		} catch {
			return url;
		}
	}

	function normalizeAddress(value: string) {
		const trimmed = value.trim();
		if (!trimmed) return 'os://terminal';
		if (routeTargets[trimmed]) return trimmed;
		if (/^[a-z]+:\/\//i.test(trimmed)) return trimmed;
		if (trimmed.includes('.') && !trimmed.includes(' ')) return `https://${trimmed}`;
		return `https://duckduckgo.com/?q=${encodeURIComponent(trimmed)}`;
	}

	function frameSrc(url: string, tabId?: string) {
		const base = routeTargets[url] ?? `/browse?url=${encodeURIComponent(url)}`;
		if (!tabId || tabId !== currentTab?.id || frameRefreshToken === 0) return base;
		return `${base}${base.includes('?') ? '&' : '?'}rv=${frameRefreshToken}`;
	}

	function isInlineTerminal(url: string) {
		return url === 'os://terminal';
	}

	function navigate(value: string, options: { replace?: boolean; newTab?: boolean } = {}) {
		const url = normalizeAddress(value);
		frameLoading = !isInlineTerminal(url);

		if (options.newTab || !currentTab) {
			const tabId = browserStore.openTab(url, { activate: true });
			browserStore.updateTab(tabId, { title: titleFor(url), loading: true });
		} else {
			browserStore.updateTab(currentTab.id, { url, title: titleFor(url), loading: true });
		}

		addressValue = url;
		if (options.replace) {
			navHistory[navIndex] = url;
		} else if (navHistory[navIndex] !== url) {
			navHistory = [...navHistory.slice(0, navIndex + 1), url];
			navIndex = navHistory.length - 1;
		}
	}

	function submitAddress() {
		navigate(addressValue);
	}

	function goBack() {
		if (navIndex <= 0) return;
		navIndex -= 1;
		navigate(navHistory[navIndex], { replace: true });
	}

	function goForward() {
		if (navIndex >= navHistory.length - 1) return;
		navIndex += 1;
		navigate(navHistory[navIndex], { replace: true });
	}

	function reloadFrame() {
		frameLoading = true;
		frameRefreshToken += 1;
	}

	function closeTab(tabId: string, event: Event) {
		event.stopPropagation();
		browserStore.closeTab(tabId);
	}

	function toggleTerminal() {
		terminalOpen = !terminalOpen;
	}

	function handleFrameLoad() {
		frameLoading = false;
		if (currentTab) {
			browserStore.updateTab(currentTab.id, { loading: false, title: titleFor(currentTab.url) });
		}
	}

	onMount(() => {
		if (!currentTab) {
			navigate('os://terminal', { newTab: true, replace: true });
		}

		const toggleTerminalListener = () => {
			toggleTerminal();
		};

		const navigateListener = (event: Event) => {
			const detail = (event as CustomEvent<{ url?: string }>).detail;
			if (detail?.url) navigate(detail.url);
		};

		window.addEventListener('soliloquy:terminal:toggle', toggleTerminalListener);
		window.addEventListener('soliloquy:navigate', navigateListener);
		return () => {
			window.removeEventListener('soliloquy:terminal:toggle', toggleTerminalListener);
			window.removeEventListener('soliloquy:navigate', navigateListener);
		};
	});
</script>

<main class="desktop-browser mode-{browserMode}" transition:fade={{ duration: 160 }}>
	<header class="browser-chrome">
		<div class="window-strip">
			<div class="traffic" aria-hidden="true">
				<span></span>
				<span></span>
				<span></span>
			</div>
			<div class="workspace-pill">
				<PanelLeft size={15} strokeWidth={1.8} />
				<span>{workspace?.name ?? 'Workspace'}</span>
			</div>
			<div class="system-readout">
				<span>{heroStatus}</span>
				<span>{$weatherStore.emoji} {$weatherStore.temp}°</span>
				<span>{$batteryStore.level}%</span>
			</div>
		</div>

		<div class="nav-strip">
			<div class="nav-buttons">
				<button type="button" aria-label="Back" disabled={navIndex <= 0} on:click={goBack}>
					<ArrowLeft size={17} strokeWidth={2} />
				</button>
				<button type="button" aria-label="Forward" disabled={navIndex >= navHistory.length - 1} on:click={goForward}>
					<ArrowRight size={17} strokeWidth={2} />
				</button>
				<button type="button" aria-label="Reload" on:click={reloadFrame}>
					<RefreshCw size={16} strokeWidth={2} />
				</button>
			</div>

			<form class="address-shell" on:submit|preventDefault={submitAddress}>
				<Search size={16} strokeWidth={2} />
				<input
					bind:value={addressValue}
					autocomplete="off"
					spellcheck="false"
					aria-label="Address"
				/>
			</form>

			<nav class="route-buttons" aria-label="System routes">
				{#each chromeRoutes as route}
					<button
						type="button"
						class:active={currentTab?.url === route.url}
						aria-label={route.label}
						title={route.label}
						on:click={() => navigate(route.url)}
					>
						<svelte:component this={route.icon} size={16} strokeWidth={2} />
					</button>
				{/each}
			</nav>

			<nav class="mode-buttons" aria-label="Browser modes">
				{#each browserModes as mode}
					<button
						type="button"
						class:active={browserMode === mode.id}
						aria-label={mode.label}
						title={mode.label}
						on:click={() => (browserMode = mode.id)}
					>
						<svelte:component this={mode.icon} size={16} strokeWidth={2} />
					</button>
				{/each}
			</nav>

			<button type="button" class="new-tab-button" aria-label="New tab" on:click={() => navigate('os://terminal', { newTab: true })}>
				<Plus size={17} strokeWidth={2} />
			</button>
		</div>

		<div class="tab-strip" aria-label="Open tabs">
			{#each tabs as tab (tab.id)}
				<button
					type="button"
					class="tab-chip"
					class:active={tab.id === currentTab?.id}
					on:click={() => browserStore.activateTab(tab.id)}
				>
					<span class="tab-title">{tab.title || titleFor(tab.url)}</span>
					<span
						role="button"
						tabindex="0"
						aria-label="Close tab"
						class="tab-close"
						on:click={(event) => closeTab(tab.id, event)}
						on:keydown={(event) => event.key === 'Enter' && closeTab(tab.id, event)}
					>
						<X size={13} strokeWidth={2.2} />
					</span>
				</button>
			{/each}
		</div>
	</header>

	{#if hasSidebar}
		<aside class="zen-sidebar" aria-label="Browser sidebar">
			<div class="sidebar-routes">
				{#each chromeRoutes as route}
					<button
						type="button"
						class:active={currentTab?.url === route.url}
						aria-label={route.label}
						title={route.label}
						on:click={() => navigate(route.url)}
					>
						<svelte:component this={route.icon} size={17} strokeWidth={2} />
					</button>
				{/each}
			</div>

			<div class="sidebar-tabs" aria-label="Open tabs">
				{#each tabs as tab (tab.id)}
					<button
						type="button"
						class="sidebar-tab"
						class:active={tab.id === currentTab?.id}
						title={tab.title || titleFor(tab.url)}
						on:click={() => browserStore.activateTab(tab.id)}
					>
						<span>{titleFor(tab.url).slice(0, 1).toUpperCase()}</span>
					</button>
				{/each}
			</div>

			<button type="button" class="sidebar-add" aria-label="New tab" title="New tab" on:click={() => navigate('os://terminal', { newTab: true })}>
				<Plus size={17} strokeWidth={2} />
			</button>
		</aside>
	{/if}

	<section class="page-frame {browserMode}">
		<div class="load-line" class:loading={frameLoading}></div>
		{#each paneTabs as paneTab (paneTab.id)}
			<article class="content-pane" class:active={paneTab.id === currentTab?.id}>
				{#if isInlineTerminal(paneTab.url)}
					<TerminalPane open mode="inline" />
				{:else}
					<iframe
						title={paneTab.title || 'Soliloquy page'}
						src={frameSrc(paneTab.url, paneTab.id)}
						on:load={() => paneTab.id === currentTab?.id && handleFrameLoad()}
					></iframe>
				{/if}
			</article>
		{:else}
			<div class="empty-pane">
				<button type="button" on:click={() => navigate('os://terminal', { newTab: true })}>
					<Plus size={17} strokeWidth={2} />
				</button>
			</div>
		{/each}
		{#if (browserMode === 'split-horizontal' || browserMode === 'split-vertical') && paneTabs.length < 2}
			<div class="empty-pane">
				<button type="button" on:click={() => navigate('os://terminal', { newTab: true })}>
					<Plus size={17} strokeWidth={2} />
				</button>
			</div>
		{/if}
		{#if browserMode === 'grid' && paneTabs.length < 4}
			{#each Array(4 - paneTabs.length) as _}
				<div class="empty-pane">
					<button type="button" on:click={() => navigate('os://terminal', { newTab: true })}>
						<Plus size={17} strokeWidth={2} />
					</button>
				</div>
			{/each}
		{/if}
	</section>

	<TerminalPane open={terminalOpen} />
</main>

<style>
	.desktop-browser {
		min-height: 100vh;
		display: grid;
		grid-template-columns: auto 1fr;
		grid-template-rows: auto 1fr;
		background: #070807;
		color: #f8f7f2;
		overflow: hidden;
	}

	.browser-chrome {
		grid-column: 1 / -1;
		background: #11120f;
		border-bottom: 1px solid rgb(248 247 242 / 0.12);
		box-shadow: 0 18px 48px rgb(0 0 0 / 0.28);
	}

	.window-strip,
	.nav-strip,
	.tab-strip {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 14px;
	}

	.window-strip {
		min-height: 38px;
		justify-content: space-between;
		border-bottom: 1px solid rgb(248 247 242 / 0.08);
	}

	.traffic {
		display: flex;
		gap: 7px;
	}

	.traffic span {
		width: 11px;
		height: 11px;
		border-radius: 999px;
		background: #2a2b26;
		border: 1px solid rgb(248 247 242 / 0.16);
	}

	.workspace-pill,
	.system-readout {
		display: flex;
		align-items: center;
		gap: 8px;
		color: rgb(248 247 242 / 0.68);
		font-size: 12px;
		font-weight: 600;
	}

	.system-readout {
		gap: 14px;
	}

	.nav-strip {
		min-height: 54px;
	}

	.nav-buttons,
	.route-buttons,
	.mode-buttons {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	button {
		color: inherit;
		font: inherit;
	}

	.nav-buttons button,
	.route-buttons button,
	.mode-buttons button,
	.new-tab-button {
		width: 32px;
		height: 32px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border: 1px solid transparent;
		border-radius: 7px;
		background: transparent;
		color: rgb(248 247 242 / 0.72);
		transition:
			background 140ms ease,
			border-color 140ms ease,
			color 140ms ease;
	}

	.nav-buttons button:hover:not(:disabled),
	.route-buttons button:hover,
	.route-buttons button.active,
	.mode-buttons button:hover,
	.mode-buttons button.active,
	.new-tab-button:hover {
		background: rgb(248 247 242 / 0.08);
		border-color: rgb(248 247 242 / 0.12);
		color: #f8f7f2;
	}

	.nav-buttons button:disabled {
		color: rgb(248 247 242 / 0.24);
	}

	.address-shell {
		min-width: 0;
		height: 36px;
		flex: 1;
		display: flex;
		align-items: center;
		gap: 9px;
		padding: 0 12px;
		border: 1px solid rgb(248 247 242 / 0.12);
		border-radius: 8px;
		background: #070807;
		color: rgb(248 247 242 / 0.56);
	}

	.address-shell:focus-within {
		border-color: rgb(128 185 164 / 0.68);
		color: #f8f7f2;
	}

	.address-shell input {
		min-width: 0;
		width: 100%;
		border: 0;
		outline: 0;
		background: transparent;
		color: #f8f7f2;
		font-size: 14px;
		letter-spacing: 0;
	}

	.tab-strip {
		min-height: 43px;
		overflow-x: auto;
		border-top: 1px solid rgb(248 247 242 / 0.08);
		scrollbar-width: none;
	}

	.mode-zen .tab-strip,
	.mode-split-horizontal .tab-strip,
	.mode-split-vertical .tab-strip,
	.mode-grid .tab-strip {
		display: none;
	}

	.tab-strip::-webkit-scrollbar {
		display: none;
	}

	.tab-chip {
		height: 30px;
		min-width: 116px;
		max-width: 220px;
		display: inline-flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 0 8px 0 11px;
		border: 1px solid rgb(248 247 242 / 0.08);
		border-radius: 7px 7px 0 0;
		background: #171814;
		color: rgb(248 247 242 / 0.66);
	}

	.tab-chip.active {
		background: #23241f;
		border-color: rgb(128 185 164 / 0.34);
		color: #f8f7f2;
	}

	.tab-title {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 12px;
		font-weight: 600;
	}

	.tab-close {
		width: 18px;
		height: 18px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: 5px;
		color: rgb(248 247 242 / 0.45);
	}

	.tab-close:hover {
		background: rgb(248 247 242 / 0.1);
		color: #f8f7f2;
	}

	.zen-sidebar {
		grid-column: 1;
		grid-row: 2;
		width: 58px;
		min-height: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		padding: 12px 8px;
		background: #0d0e0c;
		border-right: 1px solid rgb(248 247 242 / 0.1);
	}

	.sidebar-routes,
	.sidebar-tabs {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
	}

	.sidebar-tabs {
		min-height: 0;
		flex: 1;
		overflow-y: auto;
		scrollbar-width: none;
	}

	.sidebar-tabs::-webkit-scrollbar {
		display: none;
	}

	.sidebar-routes button,
	.sidebar-tab,
	.sidebar-add,
	.empty-pane button {
		width: 36px;
		height: 36px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border: 1px solid transparent;
		border-radius: 8px;
		background: transparent;
		color: rgb(248 247 242 / 0.66);
		transition:
			background 140ms ease,
			border-color 140ms ease,
			color 140ms ease;
	}

	.sidebar-routes button:hover,
	.sidebar-routes button.active,
	.sidebar-tab:hover,
	.sidebar-tab.active,
	.sidebar-add:hover,
	.empty-pane button:hover {
		background: rgb(248 247 242 / 0.08);
		border-color: rgb(248 247 242 / 0.12);
		color: #f8f7f2;
	}

	.sidebar-tab span {
		font-size: 12px;
		font-weight: 700;
	}

	.page-frame {
		grid-column: 2;
		grid-row: 2;
		position: relative;
		min-height: 0;
		display: grid;
		background: #000;
		gap: 1px;
	}

	.mode-compact .page-frame {
		grid-column: 1 / -1;
	}

	.page-frame.split-horizontal {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}

	.page-frame.split-vertical {
		grid-template-rows: repeat(2, minmax(0, 1fr));
	}

	.page-frame.grid {
		grid-template-columns: repeat(2, minmax(0, 1fr));
		grid-auto-rows: minmax(0, 1fr);
	}

	.content-pane {
		min-width: 0;
		min-height: 0;
		position: relative;
		background: #000;
	}

	.content-pane.active {
		outline: 1px solid rgb(128 185 164 / 0.42);
		outline-offset: -1px;
	}

	.page-frame iframe,
	.content-pane {
		width: 100%;
		height: 100%;
	}

	.page-frame iframe {
		display: block;
		border: 0;
		background: #000;
	}

	.empty-pane {
		min-height: 0;
		display: grid;
		place-items: center;
		background: #000;
	}

	.load-line {
		position: absolute;
		z-index: 2;
		top: 0;
		left: 0;
		width: 0;
		height: 2px;
		background: #80b9a4;
		opacity: 0;
	}

	.load-line.loading {
		width: 72%;
		opacity: 1;
		transition: width 900ms ease;
	}

	@media (max-width: 760px) {
		.window-strip {
			display: none;
		}

		.nav-strip {
			flex-wrap: wrap;
		}

		.route-buttons,
		.mode-buttons {
			display: none;
		}

		.address-shell {
			order: 3;
			flex-basis: 100%;
		}

		.tab-chip {
			min-width: 104px;
		}

		.zen-sidebar {
			width: 48px;
			padding-inline: 6px;
		}
	}
</style>
