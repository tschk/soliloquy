export type SearchCard = {
	id: string;
	title: string;
	snippet: string;
	url: string;
	source: string;
	image_url: string;
	card_type: 'web' | 'cupboard' | 'command' | 'browser';
	metadata: Record<string, string>;
};

export type SearchResponse = {
	query: string;
	cards: SearchCard[];
	suggestions: string[];
	took_ms: number;
};

const DEFAULT_BASE_URL = import.meta.env.VITE_TABLEWARE_BASE_URL ?? 'http://localhost:3030';

function resolveBaseUrl() {
	return DEFAULT_BASE_URL.replace(/\/$/, '');
}

export async function performSearch(
	query: string,
	fetcher: typeof fetch = fetch
): Promise<SearchResponse | null> {
	const endpoint = `${resolveBaseUrl()}/api/search`;
	try {
		const res = await fetcher(endpoint, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Accept: 'application/json'
			},
			credentials: 'include',
			body: JSON.stringify({ query, limit: 10 })
		});

		if (!res.ok) {
			console.warn('[search] Search request failed with', res.status);
			return null;
		}

		const data = await res.json();
		return data as SearchResponse;
	} catch (error) {
		console.error('[search] Search request error:', error);
		const fallbackUrl = `https://duckduckgo.com/?q=${encodeURIComponent(query)}`;
		return {
			query,
			cards: [
				{
					id: `fallback-${Date.now()}`,
					title: query,
					snippet: 'Open the query in DuckDuckGo',
					url: fallbackUrl,
					source: 'local fallback',
					image_url: '',
					card_type: 'browser',
					metadata: { query }
				}
			],
			suggestions: [],
			took_ms: 0
		};
	}
}

export async function getSearchSuggestions(
	query: string,
	fetcher: typeof fetch = fetch
): Promise<string[]> {
	const endpoint = `${resolveBaseUrl()}/api/search/suggestions`;
	try {
		const res = await fetcher(endpoint, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Accept: 'application/json'
			},
			credentials: 'include',
			body: JSON.stringify({ query })
		});

		if (!res.ok) {
			return [];
		}

		const data = await res.json();
		return data.suggestions || [];
	} catch (error) {
		console.error('[search] Suggestions request error:', error);
		return query ? [query] : [];
	}
}
