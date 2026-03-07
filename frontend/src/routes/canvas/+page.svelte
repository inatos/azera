<script lang="ts">
    import Sidebar from '$lib/components/Sidebar.svelte';
    import ImageGenerator from '$lib/components/ImageGenerator.svelte';
    import ImageGallery from '$lib/components/ImageGallery.svelte';
    import Model3DGenerator from '$lib/components/Model3DGenerator.svelte';
    import Model3DGallery from '$lib/components/Model3DGallery.svelte';
    import { appState } from '$lib/store.svelte';
    import { getFeatures } from '$lib/llm_service';
    import { onMount } from 'svelte';
    
    const validTabs = ['generate-2d', 'gallery-2d', 'generate-3d', 'gallery-3d'] as const;
    type Tab = typeof validTabs[number];
    
    function getInitialTab(): Tab {
        if (typeof window !== 'undefined') {
            const hash = window.location.hash.replace('#', '');
            if (validTabs.includes(hash as Tab)) return hash as Tab;
        }
        return 'generate-2d';
    }
    
    let activeTab = $state<Tab>(getInitialTab());
    let enable3D = $state(true);
    
    function setTab(tab: Tab) {
        activeTab = tab;
        window.location.hash = tab;
    }
    
    onMount(async () => {
        const flags = await getFeatures();
        enable3D = flags.enable_3d;
        
        window.addEventListener('hashchange', () => {
            const hash = window.location.hash.replace('#', '');
            if (validTabs.includes(hash as Tab)) activeTab = hash as Tab;
        });
    });
</script>

<svelte:head>
    <!-- Google model-viewer for interactive 3D preview -->
    <script type="module" src="https://ajax.googleapis.com/ajax/libs/model-viewer/3.4.0/model-viewer.min.js"></script>
</svelte:head>

<div class="flex h-screen bg-midnight-950">
    <Sidebar />
    
    <main class="flex-1 flex flex-col transition-all overflow-hidden" style={appState.isSidebarOpen ? 'margin-left: 18rem;' : 'margin-left: 3.5rem;'}>
        <header class="canvas-header">
            <h1>🎨 Canvas</h1>
            <p class="subtitle">AI Image & 3D Generation</p>
            
            <nav class="tabs">
                <button 
                    class="tab" 
                    class:active={activeTab === 'generate-2d'}
                    onclick={() => setTab('generate-2d')}
                >
                    Generate-2D
                </button>
                <button 
                    class="tab" 
                    class:active={activeTab === 'gallery-2d'}
                    onclick={() => setTab('gallery-2d')}
                >
                    Gallery-2D
                </button>
                <span class="tab-divider"></span>
                {#if enable3D}
                    <button 
                        class="tab" 
                        class:active={activeTab === 'generate-3d'}
                        onclick={() => setTab('generate-3d')}
                    >
                        Generate-3D
                    </button>
                {/if}
                <button 
                    class="tab" 
                    class:active={activeTab === 'gallery-3d'}
                    onclick={() => setTab('gallery-3d')}
                >
                    Gallery-3D
                </button>
            </nav>
        </header>
        
        <div class="canvas-content">
            {#if activeTab === 'generate-2d'}
                <ImageGenerator />
            {:else if activeTab === 'gallery-2d'}
                <ImageGallery />
            {:else if activeTab === 'generate-3d'}
                <Model3DGenerator />
            {:else}
                <Model3DGallery />
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
    
    .tab-divider {
        width: 1px;
        background: rgba(53, 57, 58, 0.5);
        margin: 0.25rem 0.25rem;
    }
    
    .canvas-content {
        flex: 1;
        overflow-y: auto;
        padding: 1.5rem 2rem;
    }
</style>
