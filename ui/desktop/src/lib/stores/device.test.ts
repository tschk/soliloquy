import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import type { NetworkInfo, BatteryInfo } from './device';

describe('device stores', () => {
	let originalNavigator: any;
	let originalWindow: any;

	beforeEach(() => {
		originalNavigator = global.navigator;
		originalWindow = global.window;
		vi.resetModules();

		// Setup window mock
		global.window = {
			addEventListener: vi.fn(),
			removeEventListener: vi.fn(),
		} as any;

		// Setup navigator mock
		global.navigator = {
			onLine: true,
			connection: {
				type: 'wifi',
				addEventListener: vi.fn(),
				removeEventListener: vi.fn(),
			},
			getBattery: vi.fn().mockResolvedValue({
				level: 0.85,
				charging: true,
			}),
		} as any;
	});

	afterEach(() => {
		global.navigator = originalNavigator;
		global.window = originalWindow;
		vi.restoreAllMocks();
	});

	describe('networkStore', () => {
		it('should initialize with current navigator state', async () => {
            const { networkStore } = await import('./device');
			let unsubscribe = networkStore.subscribe(() => {});
			const state = get(networkStore) as NetworkInfo;
			expect(state).toEqual({ online: true, type: 'wifi' });
			unsubscribe();
		});

		it('should update on online/offline events', async () => {
            const { networkStore } = await import('./device');
			// Capture event listeners
			let listeners: Record<string, Function> = {};
			global.window.addEventListener = vi.fn((event, callback) => {
				listeners[event] = callback;
			});

			let unsubscribe = networkStore.subscribe(() => {});

			// Simulate offline
			Object.defineProperty(global.navigator, 'onLine', { value: false, configurable: true });
			if (listeners.offline) listeners.offline();

			let state = get(networkStore) as NetworkInfo;
			expect(state.online).toBe(false);

			// Simulate online
			Object.defineProperty(global.navigator, 'onLine', { value: true, configurable: true });
			if (listeners.online) listeners.online();

			state = get(networkStore) as NetworkInfo;
			expect(state.online).toBe(true);

			unsubscribe();
		});

		it('should update on connection change events', async () => {
            const { networkStore } = await import('./device');
			let connectionListeners: Record<string, Function> = {};
			(global.navigator as any).connection.addEventListener = vi.fn((event: string, callback: Function) => {
				connectionListeners[event] = callback;
			});

			let unsubscribe = networkStore.subscribe(() => {});

			// Simulate connection change
			(global.navigator as any).connection.type = 'cellular';
			if (connectionListeners.change) connectionListeners.change();

			const state = get(networkStore) as NetworkInfo;
			expect(state.type).toBe('cellular');

			unsubscribe();
		});

		it('should handle missing navigator.connection', async () => {
			delete (global.navigator as any).connection;
            const { networkStore } = await import('./device');

			let unsubscribe = networkStore.subscribe(() => {});
			const state = get(networkStore) as NetworkInfo;

			expect(state).toEqual({ online: true, type: 'unknown' });
			unsubscribe();
		});

		it('should cleanup event listeners on unsubscribe', async () => {
            const { networkStore } = await import('./device');
			let unsubscribe = networkStore.subscribe(() => {});
			unsubscribe();

			expect(global.window.removeEventListener).toHaveBeenCalledWith('online', expect.any(Function));
			expect(global.window.removeEventListener).toHaveBeenCalledWith('offline', expect.any(Function));
			expect((global.navigator as any).connection.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
		});
	});

	describe('batteryStore', () => {
		beforeEach(() => {
			vi.useFakeTimers();
		});

		afterEach(() => {
			vi.useRealTimers();
		});

		it('should initialize and fetch battery status', async () => {
            const { batteryStore } = await import('./device');
			let unsubscribe = batteryStore.subscribe(() => {});

			// Wait for initial update to finish
            await vi.advanceTimersByTimeAsync(0);

			const state = get(batteryStore) as BatteryInfo;
			expect(state).toEqual({ level: 85, charging: true });

			unsubscribe();
		});

		it('should fallback to 100% not charging if getBattery throws', async () => {
			(global.navigator as any).getBattery = vi.fn().mockRejectedValue(new Error('Battery API error'));
            const { batteryStore } = await import('./device');

			let unsubscribe = batteryStore.subscribe(() => {});

            await vi.advanceTimersByTimeAsync(0);

			const state = get(batteryStore) as BatteryInfo;
			expect(state).toEqual({ level: 100, charging: false });

			unsubscribe();
		});

		it('should fallback if getBattery is not supported', async () => {
			delete (global.navigator as any).getBattery;
            const { batteryStore } = await import('./device');

			let unsubscribe = batteryStore.subscribe(() => {});

            await vi.advanceTimersByTimeAsync(0);

			const state = get(batteryStore) as BatteryInfo;
			expect(state).toEqual({ level: 100, charging: false });

			unsubscribe();
		});
	});
});
