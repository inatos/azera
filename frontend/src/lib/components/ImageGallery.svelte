<script lang="ts">
    import { appState, type GeneratedImage } from '$lib/store.svelte';
    import { onMount } from 'svelte';
    
    let selectedImage = $state<GeneratedImage | null>(null);
    let isDeleting = $state(false);
    let deleteConfirm = $state<string | null>(null);
    
    async function deleteImage(filename: string) {
        isDeleting = true;
        try {
            const response = await fetch(`http://localhost:3000/api/images/${filename}`, {
                method: 'DELETE',
            });
            
            if (response.ok) {
                await appState.fetchImages();
                if (selectedImage?.filename === filename) {
                    selectedImage = null;
                }
                deleteConfirm = null;
            } else {
                const error = await response.text();
                alert(`Failed to delete image: ${error}`);
            }
        } catch (e) {
            alert(`Failed to delete image: ${e instanceof Error ? e.message : 'Unknown error'}`);
        } finally {
            isDeleting = false;
        }
    }
    
    function openImage(image: GeneratedImage) {
        selectedImage = image;
    }
    
    function closeViewer() {
        selectedImage = null;
    }
    
    function downloadImage(image: GeneratedImage) {
        const link = document.createElement('a');
        link.href = `http://localhost:3000${image.url}`;
        link.download = image.filename;
        link.click();
    }
    
    function formatDate(dateStr: string): string {
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
        appState.fetchImages();
    });
</script>

<div class="gallery">
    {#if appState.generatedImages.length === 0}
        <div class="empty-state">
            <span class="empty-icon">🖼️</span>
            <h3>No images yet</h3>
            <p>Generate some images to see them here!</p>
        </div>
    {:else}
        <div class="gallery-grid">
            {#each appState.generatedImages as image (image.id)}
                <div class="gallery-item">
                    <button class="image-btn" onclick={() => openImage(image)}>
                        <img 
                            src={`http://localhost:3000${image.url}`} 
                            alt={image.prompt || image.filename}
                            loading="lazy"
                        />
                    </button>
                    <div class="item-overlay">
                        {#if image.personaName}
                            <span class="persona-badge">{image.personaName}</span>
                        {/if}
                    </div>
                    <div class="item-actions">
                        <button 
                            class="action-btn" 
                            onclick={() => downloadImage(image)}
                            title="Download"
                        >
                            ⬇️
                        </button>
                        {#if deleteConfirm === image.filename}
                            <button 
                                class="action-btn danger" 
                                onclick={() => deleteImage(image.filename)}
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
                                onclick={() => deleteConfirm = image.filename}
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

<!-- Image Viewer Modal -->
{#if selectedImage}
    <div class="viewer-overlay" onclick={closeViewer} onkeydown={(e) => e.key === 'Escape' && closeViewer()} role="button" tabindex="-1">
        <div class="viewer-content" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
            <button class="close-btn" onclick={closeViewer}>×</button>
            
            <img 
                src={`http://localhost:3000${selectedImage.url}`} 
                alt={selectedImage.prompt || selectedImage.filename}
            />
            
            <div class="viewer-info">
                {#if selectedImage.prompt}
                    <p class="viewer-prompt">{selectedImage.prompt}</p>
                {/if}
                <div class="viewer-meta">
                    <span>{selectedImage.width}×{selectedImage.height}</span>
                    {#if selectedImage.model}
                        <span>Model: {selectedImage.model}</span>
                    {/if}
                    {#if selectedImage.seed && selectedImage.seed > 0}
                        <span>Seed: {selectedImage.seed}</span>
                    {/if}
                    {#if selectedImage.personaName}
                        <span>By: {selectedImage.personaName}</span>
                    {/if}
                    <span>{formatDate(selectedImage.createdAt)}</span>
                </div>
                
                <div class="viewer-actions">
                    <button class="btn" onclick={() => downloadImage(selectedImage!)}>
                        ⬇️ Download
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
    
    .gallery-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
        gap: 1rem;
    }
    
    .gallery-item {
        position: relative;
        aspect-ratio: 1;
        border-radius: 12px;
        overflow: hidden;
        background: var(--bg-secondary);
    }
    
    .image-btn {
        width: 100%;
        height: 100%;
        padding: 0;
        border: none;
        background: none;
        cursor: pointer;
    }
    
    .gallery-item img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        transition: transform 0.2s ease;
    }
    
    .gallery-item:hover img {
        transform: scale(1.05);
    }
    
    .item-overlay {
        position: absolute;
        top: 0.5rem;
        left: 0.5rem;
        right: 0.5rem;
        display: flex;
        justify-content: flex-start;
        pointer-events: none;
    }
    
    .persona-badge {
        padding: 0.25rem 0.5rem;
        background: rgba(0, 0, 0, 0.6);
        border-radius: 4px;
        font-size: 0.75rem;
        color: white;
    }
    
    .item-actions {
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        padding: 0.5rem;
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
    }
    
    .viewer-content img {
        max-width: 100%;
        max-height: 70vh;
        border-radius: 8px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
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
    
    .viewer-prompt {
        margin: 0 0 1rem;
        font-size: 0.875rem;
        line-height: 1.5;
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
</style>
