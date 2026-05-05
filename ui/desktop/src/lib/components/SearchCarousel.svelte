<script lang="ts">
	import { fade, fly } from 'svelte/transition';
	import type { SearchCard } from '$lib/api/search';

	export let cards: SearchCard[] = [];
	export let onCardClick: (card: SearchCard) => void = () => {};

	let scrollContainer: HTMLElement;

	function scrollLeft() {
		scrollContainer?.scrollBy({ left: -400, behavior: 'smooth' });
	}

	function scrollRight() {
		scrollContainer?.scrollBy({ left: 400, behavior: 'smooth' });
	}

	function getCardIcon(type: string): string {
		switch (type) {
			case 'web':
				return '🌐';
			case 'cupboard':
				return '🗄️';
			case 'command':
				return '⚡';
			case 'browser':
				return '🔗';
			default:
				return '📄';
		}
	}
</script>

{#if cards.length > 0}
	<div class="carousel-wrapper" transition:fade={{ duration: 300 }}>
		<button type="button" class="scroll-button left" on:click={scrollLeft} aria-label="Scroll left">
			<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
			</svg>
		</button>

		<div class="carousel-container" bind:this={scrollContainer}>
			{#each cards as card, i (card.id)}
				<button
					type="button"
					class="card"
					on:click={() => onCardClick(card)}
					in:fly={{ x: 50, delay: i * 100, duration: 300 }}
				>
					<div class="card-icon">{getCardIcon(card.card_type)}</div>
					<div class="card-content">
						<h3 class="card-title">{card.title}</h3>
						<p class="card-snippet">{card.snippet}</p>
						{#if card.source}
							<span class="card-source">{card.source}</span>
						{/if}
					</div>
					{#if card.image_url}
						<img src={card.image_url} alt={card.title} class="card-image" />
					{/if}
				</button>
			{/each}
		</div>

		<button type="button" class="scroll-button right" on:click={scrollRight} aria-label="Scroll right">
			<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
			</svg>
		</button>
	</div>
{/if}

<style>
	.carousel-wrapper {
		position: relative;
		width: 100%;
		margin-top: 2rem;
	}

	.carousel-container {
		display: flex;
		gap: 1.5rem;
		overflow-x: auto;
		scroll-behavior: smooth;
		padding: 1rem 0;
		scrollbar-width: none;
	}

	.carousel-container::-webkit-scrollbar {
		display: none;
	}

	.card {
		flex: 0 0 24rem;
		background: rgb(255 255 255 / 0.05);
		border: 1px solid rgb(255 255 255 / 0.1);
		border-radius: 1.5rem;
		padding: 1.5rem;
		transition: all 0.3s ease;
		cursor: pointer;
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.card:hover {
		background: rgb(255 255 255 / 0.08);
		border-color: rgb(255 255 255 / 0.2);
		transform: translateY(-4px);
		box-shadow: 0 20px 40px rgb(0 0 0 / 0.4);
	}

	.card-icon {
		font-size: 2rem;
		line-height: 1;
	}

	.card-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.card-title {
		font-size: 1.25rem;
		font-weight: 600;
		color: white;
		margin: 0;
		line-height: 1.4;
	}

	.card-snippet {
		font-size: 0.875rem;
		color: rgb(255 255 255 / 0.7);
		margin: 0;
		line-height: 1.5;
		display: -webkit-box;
		line-clamp: 3;
		-webkit-line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.card-source {
		font-size: 0.75rem;
		color: rgb(255 255 255 / 0.5);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.card-image {
		width: 100%;
		height: 10rem;
		object-fit: cover;
		border-radius: 0.75rem;
		background: rgb(255 255 255 / 0.05);
	}

	.scroll-button {
		position: absolute;
		top: 50%;
		transform: translateY(-50%);
		background: rgb(255 255 255 / 0.1);
		border: 1px solid rgb(255 255 255 / 0.2);
		border-radius: 50%;
		width: 3rem;
		height: 3rem;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		cursor: pointer;
		transition: all 0.2s ease;
		z-index: 10;
		backdrop-filter: blur(12px);
	}

	.scroll-button:hover {
		background: rgb(255 255 255 / 0.2);
		border-color: rgb(255 255 255 / 0.3);
	}

	.scroll-button.left {
		left: -1.5rem;
	}

	.scroll-button.right {
		right: -1.5rem;
	}
</style>
