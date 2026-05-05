<script lang="ts">
	import { onDestroy } from 'svelte';

	export let open = false;

	const apiBase = import.meta.env.VITE_SOL_API_BASE_URL ?? 'http://127.0.0.1:8080';
	const apiToken = import.meta.env.VITE_SOL_TOKEN ?? 'dev-token-change-me';

	let output = '';
	let commandLine = '';
	let sessionId: string | null = null;
	let socket: WebSocket | null = null;
	let ready = false;
	let terminalRef: HTMLDivElement | null = null;
	let connecting = false;
	const commandInputId = 'terminal-command-input';
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
		if (connecting || socket || !open) {
			return;
		}
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
	<div class="fixed inset-x-6 bottom-6 z-50 overflow-hidden rounded-3xl border border-white/10 bg-black/90 shadow-2xl backdrop-blur-xl">
		<div class="flex items-center justify-between border-b border-white/10 px-4 py-3">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.4em] text-white/50">Terminal</p>
				<p class="text-sm text-white/70">{ready ? 'zellij / ash' : 'connecting...'}</p>
			</div>
			<button type="button" class="text-sm text-white/50 hover:text-white" on:click={disconnectSession}>
				Close
			</button>
		</div>

		<div class="grid gap-3 p-4 lg:grid-cols-[1fr_360px]">
			<div
				bind:this={terminalRef}
				class="h-72 overflow-auto rounded-2xl border border-white/10 bg-black p-4 font-mono text-sm text-emerald-300"
			>
				<pre class="whitespace-pre-wrap break-words">{output || 'booting terminal...\n'}</pre>
			</div>

			<div class="space-y-3">
				<label for={commandInputId} class="block text-xs font-semibold uppercase tracking-[0.4em] text-white/50">
					Command
				</label>
				<textarea
					id={commandInputId}
					bind:value={commandLine}
					class="h-56 w-full rounded-2xl border border-white/10 bg-white/5 p-4 font-mono text-sm text-white outline-none placeholder:text-white/30"
					placeholder="type a command and press Enter"
					on:keydown={handleKeydown}
				></textarea>
				<p class="text-xs text-white/40">
					This terminal connects to the PTY bridge in <code>sold</code> and defaults to zellij.
				</p>
			</div>
		</div>
	</div>
{/if}
