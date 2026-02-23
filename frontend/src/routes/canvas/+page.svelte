<script lang="ts">
    import Sidebar from '$lib/components/Sidebar.svelte';
    import ImageGenerator from '$lib/components/ImageGenerator.svelte';
    import ImageGallery from '$lib/components/ImageGallery.svelte';
    import { appState } from '$lib/store.svelte';
    
    let activeTab = $state<'generate' | 'gallery'>('generate');
</script>

<div class="flex h-screen bg-midnight-950">
    <Sidebar />
    
    <main class="flex-1 flex flex-col transition-all overflow-hidden" style={appState.isSidebarOpen ? 'margin-left: 18rem;' : 'margin-left: 3.5rem;'}>
        <header class="canvas-header">
            <h1>🎨 Canvas</h1>
            <p class="subtitle">AI Image Generation</p>
            
            <nav class="tabs">
                <button 
                    class="tab" 
                    class:active={activeTab === 'generate'}
                    onclick={() => activeTab = 'generate'}
                >
                    Generate
                </button>
                <button 
                    class="tab" 
                    class:active={activeTab === 'gallery'}
                    onclick={() => activeTab = 'gallery'}
                >
                    Gallery
                </button>
            </nav>
        </header>
        
        <div class="canvas-content">
            {#if activeTab === 'generate'}
                <ImageGenerator />
            {:else}
                <ImageGallery />
            {/if}
        </div>
    </main>
</div>

<style>
    .canvas-header {
        padding: 1.5rem 2rem 1rem;
        border-bottom: 1px solid rgba(53, 57, 58, 0.5);
        background: rgba(18, 19, 20, 0.8);
    }
    
    h1 {
        font-size: 1.75rem;
        margin: 0;
        color: #e8eaec;
    }
    
    .subtitle {
        color: #7c8186;
        margin: 0.25rem 0 1rem;
        font-size: 0.875rem;
    }
    
    .tabs {
        display: flex;
        gap: 0.5rem;
    }
    
    .tab {
        padding: 0.5rem 1rem;
        border: none;
        background: transparent;
        color: #7c8186;
        font-size: 0.875rem;
        cursor: pointer;
        border-radius: 6px;
        transition: all 0.15s ease;
    }
    
    .tab:hover {
        background: rgba(35, 37, 38, 0.8);
        color: #e8eaec;
    }
    
    .tab.active {
        background: linear-gradient(135deg, #e879f9 0%, #a855f7 100%);
        color: white;
    }
    
    .canvas-content {
        flex: 1;
        overflow-y: auto;
        padding: 1.5rem 2rem;
    }
</style>
