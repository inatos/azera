<script lang="ts">
	import Prism from 'prismjs';
    import { marked } from 'marked';
    // Import specific languages you expect to support
    import 'prismjs/components/prism-javascript'; 
    import 'prismjs/components/prism-typescript';
    import 'prismjs/components/prism-css';
    import 'prismjs/components/prism-python';
    import 'prismjs/components/prism-bash';
    
	import DynamicIcon from './DynamicIcon.svelte';
	import type { Message } from '$lib/state.svelte';

	let { message }: { message: Message } = $props();

    let htmlContent = $derived(marked.parse(message.content));

    $effect(() => {
        // Re-run highlight when message content changes
        if (message.content) {
            setTimeout(() => Prism.highlightAll(), 0);
        }
    });
</script>

<div class={`flex gap-4 p-6 ${message.role === 'ai' ? 'bg-zinc-900/30' : 'bg-transparent'}`}>
    <div class="flex-shrink-0 mt-1">
        {#if message.role === 'ai'}
            <DynamicIcon mood={message.mood || 'idle'} />
        {:else}
             <div class="w-10 h-10 rounded-full bg-zinc-700 flex items-center justify-center text-zinc-300 font-bold shadow-inner">
                 U
             </div>
        {/if}
    </div>
    
    <div class="flex-1 min-w-0 prose prose-invert prose-p:text-zinc-300 prose-pre:bg-[#1d1f21] prose-pre:my-2 prose-pre:rounded-lg max-w-none">
        {@html htmlContent}
    </div>
</div>