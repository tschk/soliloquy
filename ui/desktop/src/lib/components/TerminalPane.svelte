<script lang="ts">
	import { onDestroy, tick } from 'svelte';

	export let open = false;
	export let mode: 'overlay' | 'inline' = 'overlay';

	const apiBase = import.meta.env.VITE_SOL_API_BASE_URL ?? 'http://127.0.0.1:8080';
	const apiToken = import.meta.env.VITE_SOL_TOKEN ?? 'dev-token-change-me';

	let output = '';
	let sessionId: string | null = null;
	let socket: WebSocket | null = null;
	let ready = false;
	let terminalRef: HTMLDivElement | null = null;
	let connecting = false;
	let connectionAttempted = false;
	const textEncoder = new TextEncoder();
	const textDecoder = new TextDecoder();

	function appendOutput(chunk: string) {
		output += chunk;
		queueMicrotask(() => {
			if (terminalRef) {
				terminalRef.scrollTop = terminalRef.scrollHeight;
			}
		});
	}

	function resetTerminal() {
		output = '';
	}

	function focusTerminal() {
		void tick().then(() => terminalRef?.focus());
	}

	function sendBytes(value: string) {
		if (!socket || socket.readyState !== WebSocket.OPEN || !value) {
			return;
		}
		socket.send(textEncoder.encode(value));
	}

	async function connectSession() {
		if (connecting || socket || !open || connectionAttempted) {
			return;
		}
		connectionAttempted = true;
		connecting = true;
		ready = false;
		try {
			const response = await fetch(`${apiBase}/v1/term/session`, {
				method: 'POST',
				headers: {
					Authorization: `Bearer ${apiToken}`
				}
			});
			if (!response.ok) {
				throw new Error(`terminal session failed (${response.status})`);
			}
			const data: { session_id: string } = await response.json();
			sessionId = data.session_id;

			const wsUrl = new URL(`${apiBase.replace(/^http/, 'ws')}/v1/term/session/${sessionId}/ws`);
			wsUrl.searchParams.set('token', apiToken);
			socket = new WebSocket(wsUrl.toString());
			socket.binaryType = 'arraybuffer';
			socket.onopen = () => {
				ready = true;
				appendOutput('connected to terminal\n');
				socket?.send(JSON.stringify({ type: 'resize', cols: 96, rows: 28 }));
				focusTerminal();
			};
			socket.onmessage = async (event) => {
				if (event.data instanceof ArrayBuffer) {
					appendOutput(textDecoder.decode(event.data));
				} else if (event.data instanceof Blob) {
					appendOutput(textDecoder.decode(await event.data.arrayBuffer()));
				} else {
					appendOutput(String(event.data));
				}
			};
			socket.onerror = () => {
				appendOutput('\n[terminal error]\n');
			};
			socket.onclose = () => {
				ready = false;
				socket = null;
				sessionId = null;
				appendOutput('\n[terminal closed]\n');
			};
		} catch (error) {
			appendOutput(`\n[terminal unavailable] ${error instanceof Error ? error.message : 'unknown error'}\n`);
		} finally {
			connecting = false;
		}
	}

	function disconnectSession() {
		ready = false;
		connecting = false;
		if (socket) {
			socket.close();
			socket = null;
		}
		sessionId = null;
		if (!open) {
			connectionAttempted = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
			event.preventDefault();
			resetTerminal();
			return;
		}

		if (!ready) {
			return;
		}

		const ctrlKey = event.ctrlKey && event.key.length === 1;
		if (ctrlKey) {
			event.preventDefault();
			const code = event.key.toUpperCase().charCodeAt(0) - 64;
			if (code > 0 && code < 32) sendBytes(String.fromCharCode(code));
			return;
		}

		const controlKeys: Record<string, string> = {
			Enter: '\r',
			Backspace: '\x7f',
			Tab: '\t',
			Escape: '\x1b',
			ArrowUp: '\x1b[A',
			ArrowDown: '\x1b[B',
			ArrowRight: '\x1b[C',
			ArrowLeft: '\x1b[D',
			Delete: '\x1b[3~',
			Home: '\x1b[H',
			End: '\x1b[F',
			PageUp: '\x1b[5~',
			PageDown: '\x1b[6~'
		};

		const bytes = controlKeys[event.key] ?? (event.key.length === 1 ? event.key : '');
		if (bytes) {
			event.preventDefault();
			sendBytes(bytes);
		}
	}

	$: if (open && !socket && !connecting) {
		void connectSession();
	}

	$: if (!open && socket) {
		disconnectSession();
	}

	onDestroy(() => {
		disconnectSession();
	});
</script>

{#if open}
	<div
		class={mode === 'inline'
			? 'h-full overflow-hidden bg-black'
			: 'fixed inset-x-6 bottom-6 z-50 overflow-hidden rounded-3xl border border-white/10 bg-black/90 shadow-2xl backdrop-blur-xl'}
	>
		{#if mode === 'overlay'}
			<div class="flex items-center justify-between border-b border-white/10 px-4 py-3">
				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.4em] text-white/50">Terminal</p>
					<p class="text-sm text-white/70">{ready ? 'zellij / ash' : 'connecting...'}</p>
				</div>
				<button type="button" class="text-sm text-white/50 hover:text-white" on:click={disconnectSession}>
					Close
				</button>
			</div>
		{/if}

		<div class={mode === 'inline' ? 'h-full p-4' : 'p-4'}>
			<div
				bind:this={terminalRef}
				role="textbox"
				aria-label="Terminal"
				aria-multiline="true"
				tabindex="0"
				class={mode === 'inline'
					? 'h-full min-h-0 overflow-auto rounded-lg border border-white/10 bg-black p-4 font-mono text-sm text-emerald-300 outline-none focus:border-emerald-300/50'
					: 'h-80 overflow-auto rounded-2xl border border-white/10 bg-black p-4 font-mono text-sm text-emerald-300 outline-none focus:border-emerald-300/50'}
				on:click={focusTerminal}
				on:keydown={handleKeydown}
			>
				<pre class="whitespace-pre-wrap break-words">{output || 'booting terminal...\n'}</pre>
			</div>
		</div>
	</div>
{/if}
