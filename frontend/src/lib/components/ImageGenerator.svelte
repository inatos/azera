<script lang="ts">
    import { appState, type GeneratedImage } from '$lib/store.svelte';
    
    // Form state
    let prompt = $state('');
    let negativePrompt = $state('');
    let width = $state(1024);
    let height = $state(1024);
    let steps = $state(28);
    let cfgScale = $state(7);
    let seed = $state(-1);
    
    // Reference image
    let referenceImage = $state<string | null>(null);
    let referenceStrength = $state(0.75);
    let fileInput: HTMLInputElement;
    
    // UI state
    let isGenerating = $state(false);
    let progress = $state<{ step: number; totalSteps: number; percentage: number } | null>(null);
    let generatedImage = $state<GeneratedImage | null>(null);
    let error = $state<string | null>(null);
    let showAdvanced = $state(false);
    
    // Aspect ratio presets (1024-based for SDXL / DiT models)
    const aspectRatios = [
        { label: '1:1 Square', width: 1024, height: 1024 },
        { label: '3:2 Landscape', width: 1216, height: 832 },
        { label: '2:3 Portrait', width: 832, height: 1216 },
        { label: '16:9 Wide', width: 1344, height: 768 },
        { label: '9:16 Tall', width: 768, height: 1344 },
    ];
    
    function setAspectRatio(w: number, h: number) {
        width = w;
        height = h;
    }
    
    async function handleReferenceUpload(e: Event) {
        const input = e.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file) return;
        
        const reader = new FileReader();
        reader.onload = () => {
            referenceImage = reader.result as string;
        };
        reader.readAsDataURL(file);
    }
    
    function clearReference() {
        referenceImage = null;
        if (fileInput) fileInput.value = '';
    }
    
    async function generateImage() {
        if (!prompt.trim()) return;
        
        isGenerating = true;
        progress = { step: 0, totalSteps: steps, percentage: 0 };
        error = null;
        generatedImage = null;
        
        try {
            const body: Record<string, unknown> = {
                prompt: prompt.trim(),
                model: 'animagine-xl-3.1',
                width,
                height,
                steps,
                cfg_scale: cfgScale,
                seed,
            };
            
            if (negativePrompt.trim()) {
                body.negative_prompt = negativePrompt.trim();
            }
            
            if (referenceImage) {
                // Extract base64 data from data URL
                const base64 = referenceImage.split(',')[1];
                body.reference_image = base64;
                body.reference_strength = referenceStrength;
            }
            
            const response = await fetch('http://localhost:3000/api/images/generate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            });
            
            if (!response.ok) {
                throw new Error(`Generation failed: ${response.statusText}`);
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
                    if (line.startsWith('data: ')) {
                        try {
                            const data = JSON.parse(line.slice(6));
                            
                            if (data.type === 'progress' || data.step !== undefined) {
                                progress = {
                                    step: data.step,
                                    totalSteps: data.total_steps || steps,
                                    percentage: data.percentage || (data.step / steps * 100),
                                };
                            } else if (data.type === 'complete') {
                                generatedImage = data.image;
                                // Refresh gallery
                                await appState.fetchImages();
                            } else if (data.type === 'error' || data.message) {
                                error = data.message || 'Generation failed';
                            }
                        } catch (e) {
                            // Ignore parse errors
                        }
                    }
                }
            }
        } catch (e) {
            error = e instanceof Error ? e.message : 'Generation failed';
        } finally {
            isGenerating = false;
            progress = null;
        }
    }
    
    function randomizeSeed() {
        seed = Math.floor(Math.random() * 2147483647);
    }
</script>

<div class="generator">
    <div class="generator-form">
        <div class="form-section">
            <label for="prompt">Prompt</label>
            <textarea 
                id="prompt"
                bind:value={prompt}
                placeholder="Describe the image you want to generate..."
                rows="3"
                disabled={isGenerating}
            ></textarea>
        </div>
        
        <div class="form-section">
            <label for="negative">Negative Prompt (optional)</label>
            <textarea 
                id="negative"
                bind:value={negativePrompt}
                placeholder="What to avoid in the image..."
                rows="2"
                disabled={isGenerating}
            ></textarea>
        </div>
        
        <!-- Model -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label>Model</label>
            <div class="model-badge">🎨 Animagine XL 3.1</div>
            <span class="model-hint">Anime / manga generation (SDXL fine-tune)</span>
        </div>
        
        <!-- Aspect Ratio Presets -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label>Aspect Ratio</label>
            <div class="aspect-buttons">
                {#each aspectRatios as ar}
                    <button 
                        class="aspect-btn"
                        class:active={width === ar.width && height === ar.height}
                        onclick={() => setAspectRatio(ar.width, ar.height)}
                        disabled={isGenerating}
                    >
                        {ar.label}
                    </button>
                {/each}
            </div>
        </div>
        
        <!-- Reference Image -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label>Reference Image (optional)</label>
            <div class="reference-area">
                {#if referenceImage}
                    <div class="reference-preview">
                        <img src={referenceImage} alt="Reference" />
                        <button class="clear-ref" onclick={clearReference} title="Remove reference">×</button>
                    </div>
                    <div class="strength-slider">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label>Influence: {Math.round(referenceStrength * 100)}%</label>
                        <input 
                            type="range" 
                            min="0.1" 
                            max="1" 
                            step="0.05" 
                            bind:value={referenceStrength}
                            disabled={isGenerating}
                        />
                    </div>
                {:else}
                    <button class="upload-btn" onclick={() => fileInput.click()} disabled={isGenerating}>
                        📷 Upload Reference
                    </button>
                {/if}
                <input 
                    type="file" 
                    accept="image/*" 
                    bind:this={fileInput}
                    onchange={handleReferenceUpload}
                    style="display: none;"
                />
            </div>
        </div>
        
        <!-- Advanced Options -->
        <button class="toggle-advanced" onclick={() => showAdvanced = !showAdvanced}>
            {showAdvanced ? '▼' : '▶'} Advanced Options
        </button>
        
        {#if showAdvanced}
            <div class="advanced-options">
                <div class="option-row">
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label>Steps</label>
                        <input type="number" bind:value={steps} min="1" max="150" disabled={isGenerating} />
                    </div>
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label>CFG Scale</label>
                        <input type="number" bind:value={cfgScale} min="1" max="30" step="0.5" disabled={isGenerating} />
                    </div>
                </div>
                <div class="option-row">
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label>Width</label>
                        <input type="number" bind:value={width} min="64" max="2048" step="64" disabled={isGenerating} />
                    </div>
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label>Height</label>
                        <input type="number" bind:value={height} min="64" max="2048" step="64" disabled={isGenerating} />
                    </div>
                </div>
                <div class="option-row">
                    <div class="option seed-option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label>Seed (-1 = random)</label>
                        <div class="seed-input">
                            <input type="number" bind:value={seed} disabled={isGenerating} />
                            <button class="dice-btn" onclick={randomizeSeed} disabled={isGenerating} title="Randomize">🎲</button>
                        </div>
                    </div>
                </div>
            </div>
        {/if}
        
        <!-- Generate Button -->
        <button 
            class="generate-btn" 
            onclick={generateImage}
            disabled={isGenerating || !prompt.trim()}
        >
            {#if isGenerating}
                ⏳ Generating...
            {:else}
                🎨 Generate Image
            {/if}
        </button>
        
        <!-- Progress Bar -->
        {#if progress}
            <div class="progress-container">
                <div class="progress-bar">
                    <div class="progress-fill" style="width: {progress.percentage}%"></div>
                </div>
                <span class="progress-text">
                    Step {progress.step} / {progress.totalSteps} ({Math.round(progress.percentage)}%)
                </span>
            </div>
        {/if}
        
        <!-- Error Display -->
        {#if error}
            <div class="error-message">
                ❌ {error}
            </div>
        {/if}
    </div>
    
    <!-- Preview Panel -->
    <div class="preview-panel">
        {#if generatedImage}
            <div class="generated-result">
                <img src={`http://localhost:3000${generatedImage.url}`} alt={generatedImage.prompt} />
                <div class="result-info">
                    <p class="result-prompt">{generatedImage.prompt}</p>
                    {#if generatedImage.seed}
                        <p class="result-meta">Seed: {generatedImage.seed}</p>
                    {/if}
                </div>
            </div>
        {:else if isGenerating}
            <div class="generating-placeholder">
                <div class="spinner"></div>
                <p>Creating your image...</p>
            </div>
        {:else}
            <div class="empty-preview">
                <span class="preview-icon">🖼️</span>
                <p>Your generated image will appear here</p>
            </div>
        {/if}
    </div>
</div>

<style>
    .generator {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 2rem;
        max-width: 1400px;
        margin: 0 auto;
    }
    
    @media (max-width: 1024px) {
        .generator {
            grid-template-columns: 1fr;
        }
    }
    
    .generator-form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    
    .form-section {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    
    label {
        font-size: 0.875rem;
        color: var(--text-secondary);
        font-weight: 500;
    }
    
    textarea, input[type="number"] {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 0.75rem;
        color: var(--text-primary);
        font-size: 0.875rem;
        resize: vertical;
    }
    
    textarea:focus, input:focus {
        outline: none;
        border-color: var(--accent-primary);
    }
    
    textarea:disabled, input:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    
    .model-badge {
        display: inline-block;
        padding: 0.5rem 0.85rem;
        border-radius: 8px;
        border: 1px solid var(--accent-primary);
        background: rgba(232, 121, 249, 0.08);
        color: var(--text-primary);
        font-size: 0.875rem;
        font-weight: 500;
    }

    .model-hint {
        display: block;
        margin-top: 0.35rem;
        font-size: 0.75rem;
        color: var(--text-tertiary);
    }
    
    .aspect-buttons {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }
    
    .aspect-btn {
        padding: 0.375rem 0.75rem;
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        color: var(--text-secondary);
        font-size: 0.75rem;
        cursor: pointer;
        transition: all 0.15s ease;
    }
    
    .aspect-btn:hover {
        border-color: var(--accent-primary);
        color: var(--text-primary);
    }
    
    .aspect-btn.active {
        background: var(--accent-primary);
        border-color: var(--accent-primary);
        color: white;
    }
    
    .reference-area {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    
    .reference-preview {
        position: relative;
        width: fit-content;
    }
    
    .reference-preview img {
        max-width: 150px;
        max-height: 150px;
        border-radius: 8px;
        object-fit: cover;
    }
    
    .clear-ref {
        position: absolute;
        top: -8px;
        right: -8px;
        width: 24px;
        height: 24px;
        background: var(--bg-error);
        border: none;
        border-radius: 50%;
        color: white;
        font-size: 1rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .strength-slider {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .strength-slider input[type="range"] {
        width: 150px;
    }
    
    .upload-btn {
        padding: 0.75rem 1rem;
        background: var(--bg-secondary);
        border: 2px dashed var(--border-color);
        border-radius: 8px;
        color: var(--text-secondary);
        cursor: pointer;
        transition: all 0.15s ease;
    }
    
    .upload-btn:hover {
        border-color: var(--accent-primary);
        color: var(--text-primary);
    }
    
    .toggle-advanced {
        background: none;
        border: none;
        color: var(--text-secondary);
        font-size: 0.875rem;
        cursor: pointer;
        text-align: left;
        padding: 0.5rem 0;
    }
    
    .toggle-advanced:hover {
        color: var(--text-primary);
    }
    
    .advanced-options {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        padding: 1rem;
        background: var(--bg-secondary);
        border-radius: 8px;
    }
    
    .option-row {
        display: flex;
        gap: 1rem;
    }
    
    .option {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .option input {
        padding: 0.5rem;
    }
    
    .seed-option {
        flex: 2;
    }
    
    .seed-input {
        display: flex;
        gap: 0.5rem;
    }
    
    .seed-input input {
        flex: 1;
    }
    
    .dice-btn {
        padding: 0.5rem 0.75rem;
        background: var(--bg-tertiary);
        border: 1px solid var(--border-color);
        border-radius: 6px;
        cursor: pointer;
    }
    
    .generate-btn {
        margin-top: 1rem;
        padding: 1rem 2rem;
        background: var(--accent-primary);
        border: none;
        border-radius: 8px;
        color: white;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.15s ease;
    }
    
    .generate-btn:hover:not(:disabled) {
        filter: brightness(1.1);
    }
    
    .generate-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    
    .progress-container {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    
    .progress-bar {
        height: 8px;
        background: var(--bg-secondary);
        border-radius: 4px;
        overflow: hidden;
    }
    
    .progress-fill {
        height: 100%;
        background: var(--accent-primary);
        transition: width 0.2s ease;
    }
    
    .progress-text {
        font-size: 0.75rem;
        color: var(--text-secondary);
    }
    
    .error-message {
        padding: 0.75rem;
        background: rgba(239, 68, 68, 0.1);
        border: 1px solid var(--bg-error);
        border-radius: 8px;
        color: var(--bg-error);
        font-size: 0.875rem;
    }
    
    /* Preview Panel */
    .preview-panel {
        background: var(--bg-secondary);
        border-radius: 12px;
        padding: 1.5rem;
        min-height: 400px;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .empty-preview {
        text-align: center;
        color: var(--text-tertiary);
    }
    
    .preview-icon {
        font-size: 4rem;
        opacity: 0.5;
    }
    
    .empty-preview p {
        margin-top: 1rem;
    }
    
    .generating-placeholder {
        text-align: center;
        color: var(--text-secondary);
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
    
    .generated-result {
        width: 100%;
    }
    
    .generated-result img {
        width: 100%;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    }
    
    .result-info {
        margin-top: 1rem;
    }
    
    .result-prompt {
        color: var(--text-primary);
        font-size: 0.875rem;
    }
    
    .result-meta {
        color: var(--text-tertiary);
        font-size: 0.75rem;
        margin-top: 0.25rem;
    }
</style>
