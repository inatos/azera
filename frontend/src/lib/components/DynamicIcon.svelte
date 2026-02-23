<script lang="ts">
	import { type Mood } from '$lib/state.svelte';
	
	let { mood = 'idle' }: { mood: Mood } = $props();

	// Simple mapping for colors/animations based on mood
	const colors = {
		idle: 'text-fuchsia-400',
		thinking: 'text-fuchsia-400 animate-pulse',
		surprised: 'text-yellow-400',
		happy: 'text-green-400'
	};
</script>

<div class={`relative w-10 h-10 flex items-center justify-center transition-all duration-300 ${colors[mood]}`}>
	<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-8 h-8">
		<circle cx="12" cy="12" r="10" />
		
		{#if mood === 'surprised'}
			<circle cx="9" cy="9" r="1.5" />
			<circle cx="15" cy="9" r="1.5" />
		{:else if mood === 'thinking'}
			<path d="M8 8h2" />
			<path d="M14 8h2" />
		{:else}
			<circle cx="9" cy="9" r="1" fill="currentColor" />
			<circle cx="15" cy="9" r="1" fill="currentColor" />
		{/if}

		{#if mood === 'surprised'}
			<circle cx="12" cy="16" r="2" />
		{:else if mood === 'happy'}
			<path d="M8 14s1.5 2 4 2 4-2 4-2" />
		{:else if mood === 'thinking'}
			<path d="M9 16h6" />
		{:else}
			<path d="M8 15s1.5 1 4 1 4-1 4-1" />
		{/if}
	</svg>
	
	{#if mood === 'thinking'}
		<span class="absolute -top-1 -right-1 flex h-3 w-3">
		  <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-fuchsia-400 opacity-75"></span>
		  <span class="relative inline-flex rounded-full h-3 w-3 bg-fuchsia-500"></span>
		</span>
	{/if}
</div>