import { derived, readable } from 'svelte/store';

const timeOnlyFormatter = new Intl.DateTimeFormat('en-US', {
	hour: '2-digit',
	minute: '2-digit',
	hour12: false
});

const dateOnlyFormatter = new Intl.DateTimeFormat('en-US', {
	weekday: 'short',
	month: 'short',
	day: 'numeric'
});

export const systemClock = readable(new Date(), (set) => {
	const interval = setInterval(() => set(new Date()), 1000);
	return () => clearInterval(interval);
});

export const clockDisplay = derived(systemClock, ($clock) => ({
	time: timeOnlyFormatter.format($clock),
	date: dateOnlyFormatter.format($clock)
}));

export const systemIndicators = readable(
	{
		network: {
			label: 'Connected',
			signal: 'excellent'
		},
		battery: {
			percentage: 85
		},
		runtime: 'Servo Runtime • Alpine Linux'
	},
	() => () => undefined
);
