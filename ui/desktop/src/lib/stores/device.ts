import { readable } from 'svelte/store';

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
