<script lang="ts">
    import { appState } from '$lib/store.svelte';
    
    let { isOpen = $bindable(false) } = $props();
    
    let pullModelName = $state('');
    let isPulling = $state(false);
    let pullProgress = $state<{ status: string; completed?: number; total?: number } | null>(null);
    let pullError = $state<string | null>(null);
    let deleteConfirm = $state<string | null>(null);
    let isDeleting = $state(false);
    
    // Popular models to suggest
    const suggestedModels = [
        { name: 'llama3.2:latest', description: 'Fast and capable general-purpose model' },
        { name: 'llama3.2:3b', description: 'Smaller, faster version' },
        { name: 'mistral:latest', description: 'Efficient local model' },
        { name: 'codellama:latest', description: 'Code-focused model' },
        { name: 'nomic-embed-text:latest', description: 'Text embedding model' },
        { name: 'phi3:latest', description: 'Small but capable' },
        { name: 'gemma2:latest', description: 'Google\'s open model' },
        { name: 'qwen2.5:latest', description: 'Alibaba\'s multilingual model' },
    ];
    
    async function pullModel(modelName: string) {
        if (!modelName.trim()) return;
        
        isPulling = true;
        pullProgress = { status: 'Starting...' };
        pullError = null;
        
        try {
            const response = await fetch('http://localhost:3000/api/models/pull', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ model: modelName.trim() }),
            });
            
            if (!response.ok) {
                throw new Error(`Failed to start pull: ${response.statusText}`);
            }
            
            const reader = response.body?.getReader();
            if (!reader) throw new Error('No response body');
            
            const decoder = new TextDecoder();
            let buffer = '';
            
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                
                buffer += decoder.decode(value, { stream: true });
                const lines = buffer.split('\n');
                buffer = lines.pop() || '';
                
                for (const line of lines) {
                    if (line.startsWith('event: ')) {
                        const eventType = line.slice(7);
                        continue;
                    }
                    if (line.startsWith('data: ')) {
                        try {
                            const data = JSON.parse(line.slice(6));
                            if (data.error) {
                                pullError = data.error;
                                isPulling = false;
                                return;
                            }
                            if (data.status === 'success') {
                                pullProgress = { status: 'Complete!' };
                                // Refresh models list
                                await appState.fetchModels();
                                setTimeout(() => {
                                    isPulling = false;
                                    pullProgress = null;
                                    pullModelName = '';
                                }, 1000);
                                return;
                            }
                            pullProgress = {
                                status: data.status || 'Pulling...',
                                completed: data.completed,
                                total: data.total,
                            };
                        } catch (e) {
                            // Ignore parse errors for incomplete chunks
                        }
                    }
                }
            }
        } catch (e) {
            pullError = e instanceof Error ? e.message : 'Failed to pull model';
            isPulling = false;
        }
    }
    
    async function deleteModel(modelId: string) {
        isDeleting = true;
        try {
            const response = await fetch(`http://localhost:3000/api/models/${modelId}`, {
                method: 'DELETE',
            });
            
            if (response.ok) {
                await appState.fetchModels();
                deleteConfirm = null;
            } else {
                const error = await response.text();
                alert(`Failed to delete model: ${error}`);
            }
        } catch (e) {
            alert(`Failed to delete model: ${e instanceof Error ? e.message : 'Unknown error'}`);
        } finally {
            isDeleting = false;
        }
    }
    
    function formatBytes(bytes: number): string {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }
    
    function getProgressPercent(): number {
        if (!pullProgress?.total || !pullProgress?.completed) return 0;
        return Math.round((pullProgress.completed / pullProgress.total) * 100);
    }
</script>

{#if isOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
    <div 
        class="overlay" 
        onclick={() => { if (!isPulling) isOpen = false; }}
        onkeydown={(e) => { if (e.key === 'Escape' && !isPulling) isOpen = false; }}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
        <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
            <div class="header">
                <h2>🧠 Model Manager</h2>
                <button class="close-btn" onclick={() => { if (!isPulling) isOpen = false; }} disabled={isPulling}>×</button>
            </div>
            
            <div class="content">
                <!-- Installed Models -->
                <section class="section">
                    <h3>Installed Models</h3>
                    <div class="model-list">
                        {#each appState.availableModels as model (model.id)}
                            <div class="model-item">
                                <div class="model-info">
                                    <span class="model-name">{model.name}</span>
                                    <span class="model-provider">{model.provider}</span>
                                </div>
                                <div class="model-actions">
                                    {#if deleteConfirm === model.id}
                                        <span class="confirm-text">Delete?</span>
                                        <button 
                                            class="btn btn-danger btn-sm" 
                                            onclick={() => deleteModel(model.id)}
                                            disabled={isDeleting}
                                        >
                                            {isDeleting ? '...' : 'Yes'}
                                        </button>
                                        <button 
                                            class="btn btn-secondary btn-sm" 
                                            onclick={() => deleteConfirm = null}
                                            disabled={isDeleting}
                                        >
                                            No
                                        </button>
                                    {:else}
                                        <button 
                                            class="btn btn-ghost btn-sm" 
                                            onclick={() => deleteConfirm = model.id}
                                            title="Delete model"
                                        >
                                            🗑️
                                        </button>
                                    {/if}
                                </div>
                            </div>
                        {:else}
                            <p class="empty">No models installed. Pull one below!</p>
                        {/each}
                    </div>
                </section>
                
                <!-- Pull New Model -->
                <section class="section">
                    <h3>Pull New Model</h3>
                    
                    {#if pullError}
                        <div class="error-banner">
                            ⚠️ {pullError}
                            <button class="dismiss" onclick={() => pullError = null}>×</button>
                        </div>
                    {/if}
                    
                    <div class="pull-form">
                        <input 
                            type="text" 
                            bind:value={pullModelName}
                            placeholder="model:version (e.g., llama3.2:latest)"
                            disabled={isPulling}
                        />
                        <button 
                            class="btn btn-primary" 
                            onclick={() => pullModel(pullModelName)}
                            disabled={isPulling || !pullModelName.trim()}
                        >
                            {isPulling ? 'Pulling...' : 'Pull'}
                        </button>
                    </div>
                    
                    {#if isPulling && pullProgress}
                        <div class="progress-section">
                            <div class="progress-status">{pullProgress.status}</div>
                            {#if pullProgress.total}
                                <div class="progress-bar">
                                    <div class="progress-fill" style="width: {getProgressPercent()}%"></div>
                                </div>
                                <div class="progress-text">
                                    {formatBytes(pullProgress.completed || 0)} / {formatBytes(pullProgress.total)}
                                    ({getProgressPercent()}%)
                                </div>
                            {/if}
                        </div>
                    {/if}
                    
                    <div class="suggested">
                        <h4>Suggested Models</h4>
                        <div class="suggested-list">
                            {#each suggestedModels as suggestion}
                                <button 
                                    class="suggestion-chip"
                                    onclick={() => pullModelName = suggestion.name}
                                    disabled={isPulling}
                                    title={suggestion.description}
                                >
                                    {suggestion.name.split(':')[0]}
                                </button>
                            {/each}
                        </div>
                    </div>
                </section>
            </div>
        </div>
    </div>
{/if}

<style>
    .overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.7);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
        backdrop-filter: blur(4px);
    }
    
    .modal {
        background: #1a1a2e;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 12px;
        width: 90%;
        max-width: 500px;
        max-height: 80vh;
        overflow: hidden;
        display: flex;
        flex-direction: column;
    }
    
    .header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 1rem 1.25rem;
        border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }
    
    .header h2 {
        margin: 0;
        font-size: 1.1rem;
        color: #fff;
    }
    
    .close-btn {
        background: none;
        border: none;
        color: #888;
        font-size: 1.5rem;
        cursor: pointer;
        padding: 0;
        line-height: 1;
    }
    
    .close-btn:hover:not(:disabled) {
        color: #fff;
    }
    
    .content {
        padding: 1rem 1.25rem;
        overflow-y: auto;
    }
    
    .section {
        margin-bottom: 1.5rem;
    }
    
    .section:last-child {
        margin-bottom: 0;
    }
    
    .section h3 {
        font-size: 0.85rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #888;
        margin: 0 0 0.75rem 0;
    }
    
    .model-list {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    
    .model-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 8px;
    }
    
    .model-info {
        display: flex;
        flex-direction: column;
        gap: 0.2rem;
    }
    
    .model-name {
        color: #fff;
        font-weight: 500;
    }
    
    .model-provider {
        font-size: 0.75rem;
        color: #888;
    }
    
    .model-actions {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    
    .confirm-text {
        font-size: 0.8rem;
        color: #f87171;
    }
    
    .empty {
        color: #666;
        font-style: italic;
        text-align: center;
        padding: 1rem;
    }
    
    .error-banner {
        background: rgba(248, 113, 113, 0.1);
        border: 1px solid rgba(248, 113, 113, 0.3);
        color: #f87171;
        padding: 0.75rem;
        border-radius: 8px;
        margin-bottom: 1rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
    }
    
    .error-banner .dismiss {
        background: none;
        border: none;
        color: #f87171;
        cursor: pointer;
        font-size: 1.2rem;
        padding: 0;
    }
    
    .pull-form {
        display: flex;
        gap: 0.5rem;
    }
    
    .pull-form input {
        flex: 1;
        padding: 0.6rem 0.8rem;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 6px;
        color: #fff;
        font-size: 0.9rem;
    }
    
    .pull-form input:focus {
        outline: none;
        border-color: rgba(139, 92, 246, 0.5);
    }
    
    .pull-form input::placeholder {
        color: #666;
    }
    
    .btn {
        padding: 0.6rem 1rem;
        border-radius: 6px;
        font-size: 0.9rem;
        cursor: pointer;
        border: none;
        transition: all 0.15s;
    }
    
    .btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    
    .btn-primary {
        background: #8b5cf6;
        color: white;
    }
    
    .btn-primary:hover:not(:disabled) {
        background: #7c3aed;
    }
    
    .btn-secondary {
        background: rgba(255, 255, 255, 0.1);
        color: #fff;
    }
    
    .btn-danger {
        background: #f87171;
        color: white;
    }
    
    .btn-ghost {
        background: transparent;
        color: #888;
    }
    
    .btn-ghost:hover:not(:disabled) {
        color: #fff;
        background: rgba(255, 255, 255, 0.1);
    }
    
    .btn-sm {
        padding: 0.3rem 0.6rem;
        font-size: 0.8rem;
    }
    
    .progress-section {
        margin-top: 1rem;
        padding: 0.75rem;
        background: rgba(255, 255, 255, 0.05);
        border-radius: 8px;
    }
    
    .progress-status {
        color: #8b5cf6;
        font-size: 0.85rem;
        margin-bottom: 0.5rem;
    }
    
    .progress-bar {
        height: 6px;
        background: rgba(255, 255, 255, 0.1);
        border-radius: 3px;
        overflow: hidden;
    }
    
    .progress-fill {
        height: 100%;
        background: linear-gradient(90deg, #8b5cf6, #a78bfa);
        transition: width 0.3s;
    }
    
    .progress-text {
        font-size: 0.75rem;
        color: #888;
        margin-top: 0.4rem;
        text-align: right;
    }
    
    .suggested {
        margin-top: 1rem;
    }
    
    .suggested h4 {
        font-size: 0.75rem;
        color: #666;
        margin: 0 0 0.5rem 0;
    }
    
    .suggested-list {
        display: flex;
        flex-wrap: wrap;
        gap: 0.4rem;
    }
    
    .suggestion-chip {
        padding: 0.3rem 0.6rem;
        background: rgba(139, 92, 246, 0.15);
        border: 1px solid rgba(139, 92, 246, 0.3);
        border-radius: 4px;
        color: #a78bfa;
        font-size: 0.75rem;
        cursor: pointer;
        transition: all 0.15s;
    }
    
    .suggestion-chip:hover:not(:disabled) {
        background: rgba(139, 92, 246, 0.25);
        border-color: rgba(139, 92, 246, 0.5);
    }
    
    .suggestion-chip:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
</style>
