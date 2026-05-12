<script lang="ts">
	import { onDestroy } from 'svelte';

	export let open = false;
	export let mode: 'overlay' | 'inline' = 'overlay';

	const apiBase = import.meta.env.VITE_SOL_API_BASE_URL ?? 'http://127.0.0.1:8080';
	const apiToken = import.meta.env.VITE_SOL_TOKEN ?? 'dev-token-change-me';

	let output = '';
	let commandLine = '';
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
		commandLine = '';
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

	function sendInput(value: string) {
		if (!socket || socket.readyState !== WebSocket.OPEN || !value) {
			return;
		}
		socket.send(textEncoder.encode(`${value}\r`));
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			sendInput(commandLine.trimEnd());
			commandLine = '';
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
			event.preventDefault();
			resetTerminal();
			return;
		}
		if (event.key === 'Tab') {
			event.preventDefault();
			commandLine += '\t';
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
				class={mode === 'inline'
					? 'flex h-full min-h-0 flex-col overflow-hidden rounded-lg border border-white/10 bg-black font-mono text-sm text-emerald-300'
					: 'flex h-80 flex-col overflow-hidden rounded-2xl border border-white/10 bg-black font-mono text-sm text-emerald-300'}
			>
				<pre class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-words p-4">{output || 'booting terminal...\n'}</pre>
				<label class="flex items-center gap-2 border-t border-white/10 px-4 py-3 text-white">
					<span class="shrink-0 text-emerald-300">soliloquy%</span>
					<input
						bind:value={commandLine}
						class="min-w-0 flex-1 border-0 bg-transparent font-mono text-sm text-white outline-none placeholder:text-white/30"
						placeholder={ready ? 'type a command' : 'terminal unavailable'}
						autocomplete="off"
						spellcheck="false"
						disabled={!ready}
						on:keydown={handleKeydown}
					/>
				</label>
			</div>
		</div>
	</div>
{/if}
