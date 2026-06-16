export type SystemAction =
	| 'auth.unlock'
	| 'auth.reset'
	| 'commands.open'
	| 'files.open'
	| 'terminal.open'
	| 'sessions.resume'
	| 'tabs.restore'
	| 'navigate.url';

// URL navigation state
let pendingNavigationUrl: string | null = null;

export function setPendingNavigation(url: string) {
	pendingNavigationUrl = url;
}

export function getPendingNavigation(): string | null {
	const url = pendingNavigationUrl;
	pendingNavigationUrl = null;
	return url;
}

// Check if a string looks like a URL
export function isValidUrl(input: string): boolean {
	input = input.trim();
	// Check for explicit protocol
	if (/^https?:\/\//i.test(input)) {
		try {
			new URL(input);
			return true;
		} catch {
			return false;
		}
	}
	// Check for domain-like patterns (e.g., google.com, sub.domain.org/path)
	if (/^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z]{2,})+/.test(input)) {
		return true;
	}
	// Check for localhost and typical local dev patterns
	if (/^localhost(:\d+)?(\/.*)?$/.test(input) || /^127\.0\.0\.1(:\d+)?(\/.*)?$/.test(input)) {
		return true;
	}
	return false;
}

// Normalize URL (add https:// if missing)
export function normalizeUrl(input: string): string {
	if (/^https?:\/\//i.test(input)) {
		return input;
	}
	return `https://${input}`;
}

const actionHandlers: Record<SystemAction, () => Promise<void> | void> = {
	'auth.unlock': async () => {
		console.info('[system-action] Unlock request dispatched');
	},
	'auth.reset': async () => {
		console.info('[system-action] Credential reset invoked');
	},
	'commands.open': () => {
		if (typeof window !== 'undefined') {
			window.dispatchEvent(new CustomEvent('soliloquy:command:toggle'));
		}
	},
	'files.open': () => {
		console.info('[system-action] File explorer launch');
	},
	'terminal.open': () => {
		if (typeof window !== 'undefined') {
			window.dispatchEvent(new CustomEvent('soliloquy:terminal:toggle'));
		}
	},
	'sessions.resume': () => {
		console.info('[system-action] Resuming chat session');
	},
	'tabs.restore': () => {
		console.info('[system-action] Restoring browser tabs');
	},
	'navigate.url': () => {
		const url = getPendingNavigation();
		if (url) {
			const normalizedUrl = normalizeUrl(url);
			console.info('[system-action] Navigating to:', normalizedUrl);
			// Dispatch navigation event for Servo/browser to handle
			if (typeof window !== 'undefined') {
				window.dispatchEvent(new CustomEvent('soliloquy:navigate', { 
					detail: { url: normalizedUrl } 
				}));
				// Fallback: open in new tab if not in Servo
				window.location.href = normalizedUrl;
			}
		}
	}
};

export async function runSystemAction(action: SystemAction) {
	const handler = actionHandlers[action];
	if (!handler) {
		console.warn('[system-action] Missing handler for', action);
		return;
	}

	await handler();
}

export function getGoogleAuthURL(): string {
	const baseUrl = import.meta.env.VITE_TABLEWARE_BASE_URL ?? 'http://localhost:3030';
	return `${baseUrl.replace(/\/$/, '')}/api/auth/google`;
}

export type CommandSuggestion = {
	title: string;
	description: string;
	shortcut?: string;
	action: SystemAction;
};

export const defaultCommandSuggestions: CommandSuggestion[] = [
	{
		title: 'Unlock Desktop',
		description: 'Authenticate and enter the Servo environment',
		shortcut: '⌘ ↵',
		action: 'auth.unlock'
	},
	{
		title: 'Resume Chats',
		description: 'Open the most recent Soliloquy conversation',
		shortcut: '⌘ ⇧ C',
		action: 'sessions.resume'
	},
	{
		title: 'Open File Explorer',
		description: 'Navigate system files starting at /home/max',
		shortcut: '⌘ ⌥ F',
		action: 'files.open'
	},
	{
		title: 'Open Terminal',
		description: 'Launch the sold-backed terminal session',
		shortcut: '⌘ ⌥ T',
		action: 'terminal.open'
	},
	{
		title: 'Restore Tabs',
		description: 'Rehydrate the previous browsing workspace',
		action: 'tabs.restore'
	}
];
