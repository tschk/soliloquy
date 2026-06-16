import { readable } from 'svelte/store';

export type NetworkInfo = {
online: boolean;
type: string;
};

export const networkStore = readable<NetworkInfo>({ online: true, type: 'wifi' }, (set) => {
	const updateNetwork = () => {
		let type = 'unknown';
		try {
			// Typecast needed as connection isn't standard in all TS DOM lib versions
			const nav = navigator as any;
			if (nav.connection && nav.connection.type) {
				type = nav.connection.type;
			}
		} catch (e) { /* ignore */ }

		set({
			online: navigator.onLine ?? true,
			type
		});
	};

	if (typeof window !== 'undefined') {
		window.addEventListener('online', updateNetwork);
		window.addEventListener('offline', updateNetwork);

		const nav = navigator as any;
		if (nav.connection) {
			nav.connection.addEventListener('change', updateNetwork);
		}

		// Initial update
		updateNetwork();
	}

	return () => {
		if (typeof window !== 'undefined') {
			window.removeEventListener('online', updateNetwork);
			window.removeEventListener('offline', updateNetwork);
			const nav = navigator as any;
			if (nav.connection) {
				nav.connection.removeEventListener('change', updateNetwork);
			}
		}
	};
});

export type BatteryInfo = {
level: number;
charging: boolean;
};

export type WeatherInfo = {
temp: number;
condition: string;
emoji: string;
};

async function getBatteryInfo(): Promise<BatteryInfo> {
if (typeof navigator !== 'undefined' && 'getBattery' in navigator) {
try {
const battery = await (navigator as any).getBattery();
return {
level: Math.round(battery.level * 100),
charging: battery.charging
};
} catch {
return { level: 100, charging: false };
}
}
return { level: 100, charging: false };
}

export const batteryStore = readable<BatteryInfo>({ level: 100, charging: false }, (set) => {
let interval: ReturnType<typeof setInterval>;

const update = async () => {
const info = await getBatteryInfo();
set(info);
};

update();
interval = setInterval(update, 30000);

return () => clearInterval(interval);
});

export const weatherStore = readable<WeatherInfo>(
{ temp: 19, condition: 'Cloudy', emoji: '☁️' },
() => () => undefined
);
