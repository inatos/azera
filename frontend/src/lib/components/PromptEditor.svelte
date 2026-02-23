<script lang="ts">
    import { appState } from "$lib/store.svelte";
    
    // Props in Svelte 5
    let { isOpen, close }: { isOpen: boolean, close: () => void } = $props();
    
    let activePrompt = $state(appState.prompts[0]);
</script>

{#if isOpen}
    <div class="fixed inset-0 bg-black/80 z-50 flex items-center justify-center p-10">
        <div class="bg-[#1e1f20] w-full max-w-4xl h-[80vh] rounded-xl flex flex-col border border-gray-700 overflow-hidden shadow-2xl">
            <div class="flex justify-between p-4 border-b border-gray-700 bg-[#2a2b2d]">
                <h3 class="font-bold">Prompt Engineer</h3>
                <button onclick={close}>✕</button>
            </div>
            
            <div class="flex flex-1">
                <div class="w-1/3 border-r border-gray-700 overflow-y-auto">
                    {#each appState.prompts as p}
                        <button 
                            onclick={() => activePrompt = p}
                            class="w-full text-left p-3 hover:bg-gray-700 border-b border-gray-800 text-sm {activePrompt.id === p.id ? 'bg-blue-900/30' : ''}"
                        >
                            <div class="font-bold">{p.title}</div>
                            <div class="text-xs text-gray-400">{p.tag}</div>
                        </button>
                    {/each}
                </div>
                
                <div class="flex-1 bg-[#131314] p-4 font-mono text-sm relative group">
                    <textarea 
                        class="w-full h-full bg-transparent text-green-400 resize-none focus:outline-none"
                        spellcheck="false"
                        bind:value={activePrompt.code}
                    ></textarea>
                    
                    <div class="absolute bottom-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity">
                         <button class="bg-blue-600 px-4 py-2 rounded text-white">Save Changes</button>
                    </div>
                </div>
            </div>
        </div>
    </div>
{/if}