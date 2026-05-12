import browserChromeTemplate from './browserChrome.crepus?raw';

export type ChromeRouteKey = 'terminal' | 'files' | 'settings';
export type BrowserMode = 'zen' | 'compact' | 'split-horizontal' | 'split-vertical' | 'grid';

export type ChromeRouteSpec = {
	key: ChromeRouteKey;
	label: string;
	url: string;
};

export type BrowserModeSpec = {
	id: BrowserMode;
	label: string;
	crepusVariant: string;
};

export const crepuscularityChromeTemplate = browserChromeTemplate;

export const crepuscularityChromeSource = {
	project: '../crepuscularity',
	template: 'ui/desktop/src/lib/crepuscularity/browserChrome.crepus',
	component: 'SoliloquyBrowserChrome',
	nativeBackend: 'crepuscularity-gpui'
} as const;

export const crepuscularityChromeTokens = {
	canvas: '#070807',
	chrome: '#11120f',
	sidebar: '#0d0e0c',
	text: '#f8f7f2',
	mutedText: 'rgb(248 247 242 / 0.66)',
	line: 'rgb(248 247 242 / 0.12)',
	activeLine: 'rgb(128 185 164 / 0.42)',
	accent: '#80b9a4',
	buttonRadius: '7px',
	sidebarWidth: '58px'
} as const;

export const crepuscularityChromeRoutes: ChromeRouteSpec[] = [
	{ key: 'terminal', label: 'Terminal', url: 'os://terminal' },
	{ key: 'files', label: 'Files', url: 'os://files' },
	{ key: 'settings', label: 'Settings', url: 'os://settings' }
];

export const crepuscularityChromeModes: BrowserModeSpec[] = [
	{ id: 'zen', label: 'Zen', crepusVariant: 'sidebar' },
	{ id: 'compact', label: 'Compact', crepusVariant: 'topbar' },
	{ id: 'split-horizontal', label: 'Split columns', crepusVariant: 'split-columns' },
	{ id: 'split-vertical', label: 'Split rows', crepusVariant: 'split-rows' },
	{ id: 'grid', label: 'Grid', crepusVariant: 'grid' }
];

export const crepuscularityChromeCssVars = [
	`--crepus-canvas: ${crepuscularityChromeTokens.canvas}`,
	`--crepus-chrome: ${crepuscularityChromeTokens.chrome}`,
	`--crepus-sidebar: ${crepuscularityChromeTokens.sidebar}`,
	`--crepus-text: ${crepuscularityChromeTokens.text}`,
	`--crepus-muted-text: ${crepuscularityChromeTokens.mutedText}`,
	`--crepus-line: ${crepuscularityChromeTokens.line}`,
	`--crepus-active-line: ${crepuscularityChromeTokens.activeLine}`,
	`--crepus-accent: ${crepuscularityChromeTokens.accent}`,
	`--crepus-button-radius: ${crepuscularityChromeTokens.buttonRadius}`,
	`--crepus-sidebar-width: ${crepuscularityChromeTokens.sidebarWidth}`
].join('; ');

export function assertCrepuscularityChromeContract() {
	const required = ['SoliloquyBrowserChrome', 'os://terminal', 'terminal', 'settings'];
	return required.every((token) => crepuscularityChromeTemplate.includes(token));
}
