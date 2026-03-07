<script lang="ts">
    import { list3DModels, delete3DModel, type Generated3DModel } from '$lib/llm_service';
    import { onMount } from 'svelte';
    
    let models = $state<Generated3DModel[]>([]);
    let loading = $state(true);
    let selectedModel = $state<Generated3DModel | null>(null);
    let isDeleting = $state(false);
    let deleteConfirm = $state<string | null>(null);
    
    // Render settings (for full viewer)
    let renderEnabled = $state(true);
    let renderExposure = $state(1.0);
    let renderShadow = $state(1.0);
    let renderAutoRotate = $state(true);
    
    async function refresh() {
        loading = true;
        try {
            const result = await list3DModels();
            // model-viewer only supports GLB/GLTF — filter out OBJ and other formats
            models = result.items.filter(m => m.format === 'glb' || m.format === 'gltf');
        } catch (e) {
            console.error('Failed to load 3D models:', e);
        }
        loading = false;
    }
    
    async function handleDelete(filename: string) {
        isDeleting = true;
        try {
            await delete3DModel(filename);
            models = models.filter(m => m.filename !== filename);
            if (selectedModel?.filename === filename) {
                selectedModel = null;
            }
            deleteConfirm = null;
        } catch (e) {
            alert(`Failed to delete: ${e instanceof Error ? e.message : 'Unknown error'}`);
        } finally {
            isDeleting = false;
        }
    }
    
    function openModel(model: Generated3DModel) {
        selectedModel = model;
    }
    
    function closeViewer() {
        selectedModel = null;
    }
    
    function downloadModel(model: Generated3DModel) {
        const link = document.createElement('a');
        link.href = `http://localhost:3000${model.url}`;
        link.download = model.filename;
        link.click();
    }
    
    function formatSize(bytes: number): string {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
    }
    
    function formatDate(dateStr: string): string {
        if (!dateStr) return '';
        const date = new Date(dateStr);
        return date.toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        });
    }
    
    onMount(() => {
        refresh();
    });
</script>

<div class="gallery">
    {#if loading}
        <div class="empty-state">
            <div class="spinner"></div>
            <p>Loading 3D models...</p>
        </div>
    {:else if models.length === 0}
        <div class="empty-state">
            <span class="empty-icon">🧊</span>
            <h3>No 3D models yet</h3>
            <p>Generate some 3D models to see them here!</p>
        </div>
    {:else}
        <div class="gallery-header">
            <span class="model-count">{models.length} model{models.length !== 1 ? 's' : ''}</span>
            <button class="refresh-btn" onclick={refresh} title="Refresh">🔄</button>
        </div>
        <div class="gallery-grid">
            {#each models as model (model.filename)}
                <div class="gallery-item">
                    <button class="model-btn" onclick={() => openModel(model)} aria-label={`View ${model.filename}`}>
                        <div class="model-preview">
                            <model-viewer
                                src={`http://localhost:3000${model.url}`}
                                alt={model.prompt || model.filename}
                                auto-rotate
                                camera-controls
                                interaction-prompt="none"
                                style="width: 100%; height: 100%;"
                            ></model-viewer>
                        </div>
                    </button>
                    <div class="item-info">
                        <p class="item-name" title={model.filename}>{model.filename}</p>
                        {#if model.prompt}
                            <p class="item-prompt" title={model.prompt}>{model.prompt}</p>
                        {/if}
                        <p class="item-meta">{model.format.toUpperCase()} · {formatSize(model.file_size)}</p>
                    </div>
                    <div class="item-actions">
                        <button 
                            class="action-btn" 
                            onclick={() => downloadModel(model)}
                            title="Download"
                        >
                            ⬇️
                        </button>
                        {#if deleteConfirm === model.filename}
                            <button 
                                class="action-btn danger" 
                                onclick={() => handleDelete(model.filename)}
                                disabled={isDeleting}
                                title="Confirm delete"
                            >
                                ✓
                            </button>
                            <button 
                                class="action-btn" 
                                onclick={() => deleteConfirm = null}
                                disabled={isDeleting}
                                title="Cancel"
                            >
                                ✕
                            </button>
                        {:else}
                            <button 
                                class="action-btn" 
                                onclick={() => deleteConfirm = model.filename}
                                title="Delete"
                            >
                                🗑️
                            </button>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>

<!-- 3D Model Viewer Modal -->
{#if selectedModel}
    <div class="viewer-overlay" onclick={closeViewer} onkeydown={(e) => e.key === 'Escape' && closeViewer()} role="button" tabindex="-1">
        <div class="viewer-content" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
            <button class="close-btn" onclick={closeViewer}>×</button>
            
            <div class="viewer-model">
                <model-viewer
                    src={`http://localhost:3000${selectedModel.url}`}
                    alt={selectedModel.prompt || selectedModel.filename}
                    camera-controls
                    auto-rotate={renderAutoRotate ? true : undefined}
                    environment-image={renderEnabled ? 'neutral' : undefined}
                    shadow-intensity={renderEnabled ? String(renderShadow) : '0'}
                    shadow-softness={renderEnabled ? '0.8' : '0'}
                    exposure={String(renderExposure)}
                    tone-mapping={renderEnabled ? 'commerce' : undefined}
                    style="width: 100%; height: 500px;"
                ></model-viewer>
                <div class="render-controls">
                    <label class="render-toggle">
                        <input type="checkbox" bind:checked={renderEnabled} />
                        Render
                    </label>
                    <label class="render-toggle">
                        <input type="checkbox" bind:checked={renderAutoRotate} />
                        Rotate
                    </label>
                    {#if renderEnabled}
                        <div class="render-slider">
                            <span>Exposure</span>
                            <input type="range" min="0.2" max="2" step="0.1" bind:value={renderExposure} />
                        </div>
                        <div class="render-slider">
                            <span>Shadow</span>
                            <input type="range" min="0" max="2" step="0.1" bind:value={renderShadow} />
                        </div>
                    {/if}
                </div>
            </div>
            
            <div class="viewer-info">
                <p class="viewer-filename">{selectedModel.filename}</p>
                {#if selectedModel.prompt}
                    <p class="viewer-prompt">{selectedModel.prompt}</p>
                {/if}
                <div class="viewer-meta">
                    <span>{selectedModel.format.toUpperCase()}</span>
                    <span>{formatSize(selectedModel.file_size)}</span>
                    {#if selectedModel.seed}
                        <span>Seed: {selectedModel.seed}</span>
                    {/if}
                    <span>{formatDate(selectedModel.created_at)}</span>
                </div>
                
                <div class="viewer-actions">
                    <button class="btn" onclick={() => downloadModel(selectedModel!)}>
                        ⬇️ Download {selectedModel.format.toUpperCase()}
                    </button>
                </div>
            </div>
        </div>
    </div>
{/if}

<style>
    .gallery {
        min-height: 300px;
    }
    
    .empty-state {
        text-align: center;
        padding: 4rem 2rem;
        color: var(--text-tertiary);
    }
    
    .empty-icon {
        font-size: 4rem;
        opacity: 0.5;
    }
    
    .empty-state h3 {
        margin: 1rem 0 0.5rem;
        color: var(--text-secondary);
    }
    
    .gallery-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }
    
    .model-count {
        font-size: 0.875rem;
        color: var(--text-secondary);
    }
    
    .refresh-btn {
        padding: 0.375rem 0.5rem;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        cursor: pointer;
        font-size: 0.875rem;
        transition: all 0.15s ease;
    }
    
    .refresh-btn:hover {
        border-color: var(--accent-primary);
    }
    
    .gallery-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
        gap: 1rem;
    }
    
    .gallery-item {
        position: relative;
        border-radius: 12px;
        overflow: hidden;
        background: var(--bg-secondary);
        transition: transform 0.15s ease;
    }
    
    .gallery-item:hover {
        transform: translateY(-2px);
    }
    
    .model-btn {
        width: 100%;
        padding: 0;
        border: none;
        background: none;
        cursor: pointer;
    }
    
    .model-preview {
        width: 100%;
        height: 200px;
        background: var(--bg-tertiary);
    }
    
    .item-info {
        padding: 0.75rem;
    }
    
    .item-name {
        font-size: 0.8rem;
        color: var(--text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin: 0;
    }
    
    .item-prompt {
        font-size: 0.75rem;
        color: var(--text-tertiary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        margin: 0.25rem 0 0;
    }
    
    .item-meta {
        font-size: 0.7rem;
        color: var(--text-tertiary);
        margin: 0.25rem 0 0;
    }
    
    .item-actions {
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        padding: 0.5rem 0.75rem;
        background: linear-gradient(transparent, rgba(0, 0, 0, 0.8));
        display: flex;
        justify-content: flex-end;
        gap: 0.25rem;
        opacity: 0;
        transition: opacity 0.2s ease;
    }
    
    .gallery-item:hover .item-actions {
        opacity: 1;
    }
    
    .action-btn {
        padding: 0.375rem 0.5rem;
        background: rgba(255, 255, 255, 0.1);
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.875rem;
        transition: all 0.15s ease;
    }
    
    .action-btn:hover {
        background: rgba(255, 255, 255, 0.2);
    }
    
    .action-btn.danger {
        background: rgba(239, 68, 68, 0.8);
    }
    
    .spinner {
        width: 48px;
        height: 48px;
        border: 3px solid var(--border-color);
        border-top-color: var(--accent-primary);
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin: 0 auto 1rem;
    }
    
    @keyframes spin {
        to { transform: rotate(360deg); }
    }
    
    /* Viewer Modal */
    .viewer-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.9);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
        padding: 2rem;
    }
    
    .viewer-content {
        max-width: 90vw;
        max-height: 90vh;
        display: flex;
        flex-direction: column;
        align-items: center;
        position: relative;
        width: 800px;
    }
    
    .viewer-model {
        width: 100%;
        border-radius: 8px;
        overflow: hidden;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
        background: var(--bg-tertiary);
    }
    
    .close-btn {
        position: absolute;
        top: -3rem;
        right: 0;
        width: 40px;
        height: 40px;
        background: rgba(255, 255, 255, 0.1);
        border: none;
        border-radius: 50%;
        color: white;
        font-size: 1.5rem;
        cursor: pointer;
        transition: all 0.15s ease;
    }
    
    .close-btn:hover {
        background: rgba(255, 255, 255, 0.2);
    }
    
    .viewer-info {
        margin-top: 1.5rem;
        text-align: center;
        color: white;
        max-width: 600px;
    }
    
    .viewer-filename {
        font-size: 0.875rem;
        font-weight: 500;
        margin: 0 0 0.5rem;
    }
    
    .viewer-prompt {
        margin: 0 0 1rem;
        font-size: 0.875rem;
        line-height: 1.5;
        color: var(--text-secondary);
    }
    
    .viewer-meta {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: 1rem;
        font-size: 0.75rem;
        color: var(--text-tertiary);
    }
    
    .viewer-actions {
        margin-top: 1rem;
    }
    
    .btn {
        padding: 0.5rem 1rem;
        background: var(--accent-primary);
        border: none;
        border-radius: 6px;
        color: white;
        font-size: 0.875rem;
        cursor: pointer;
        transition: all 0.15s ease;
    }
    
    .btn:hover {
        filter: brightness(1.1);
    }
    
    .render-controls {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.5rem 0.75rem;
        background: rgba(0, 0, 0, 0.3);
        flex-wrap: wrap;
    }
    
    .render-toggle {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        font-size: 0.75rem;
        color: rgba(255, 255, 255, 0.7);
        cursor: pointer;
        user-select: none;
    }
    
    .render-toggle input[type="checkbox"] {
        accent-color: var(--accent-primary);
    }
    
    .render-slider {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        font-size: 0.7rem;
        color: rgba(255, 255, 255, 0.5);
    }
    
    .render-slider input[type="range"] {
        width: 60px;
        height: 4px;
        accent-color: var(--accent-primary);
    }
</style>
