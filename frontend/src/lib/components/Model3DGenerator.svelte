<script lang="ts">
    import { generate3DModel, type Generate3DRequest, type Generated3DModel } from '$lib/llm_service';
    
    // Form state
    let referenceImage = $state<string | null>(null);
    let referenceFile = $state<File | null>(null);
    let fileInput: HTMLInputElement;
    
    // Generation parameters
    let steps = $state(50);
    let guidanceScale = $state(7.5);
    let octreeResolution = $state(256);
    let numViews = $state(6);
    let seed = $state(-1);
    let removeBackground = $state(true);
    let foregroundRatio = $state(0.9);
    let textureSize = $state(1024);
    let outputFormat = $state('glb');
    let enableTexture = $state(true);
    
    // UI state
    let isGenerating = $state(false);
    let progress = $state<{ step: number; totalSteps: number; percentage: number; status: string } | null>(null);
    let lastGenerated = $state<Generated3DModel | null>(null);
    let error = $state<string | null>(null);
    let showAdvanced = $state(false);
    
    // Render settings
    let renderEnabled = $state(true);
    let renderExposure = $state(1.0);
    let renderShadow = $state(1.0);
    let renderAutoRotate = $state(true);
    
    async function handleReferenceUpload(e: Event) {
        const input = e.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file) return;
        
        referenceFile = file;
        const reader = new FileReader();
        reader.onload = () => {
            referenceImage = reader.result as string;
        };
        reader.readAsDataURL(file);
    }
    
    function clearReference() {
        referenceImage = null;
        referenceFile = null;
        if (fileInput) fileInput.value = '';
    }
    
    function randomizeSeed() {
        seed = Math.floor(Math.random() * 2147483647);
    }
    
    function statusLabel(status: string): string {
        switch (status) {
            case 'loading': return 'Loading model...';
            case 'preparing': return 'Preparing...';
            case 'diffusion': return 'Generating shape (diffusion)...';
            case 'decoding_mesh': return 'Decoding mesh...';
            case 'generating_shape': return 'Generating shape...';
            case 'saving_shape': return 'Saving shape...';
            case 'generating_texture': return 'Rendering texture onto mesh...';
            case 'converting': return 'Converting to GLB...';
            case 'saving': return 'Saving final mesh...';
            case 'starting': return 'Starting...';
            case 'complete': return 'Complete!';
            default: return status;
        }
    }
    
    async function handleGenerate() {
        if (!referenceImage) return;
        
        isGenerating = true;
        progress = { step: 0, totalSteps: steps, percentage: 0, status: 'starting' };
        error = null;
        lastGenerated = null;
        
        const req: Generate3DRequest = {
            steps,
            guidance_scale: guidanceScale,
            octree_resolution: octreeResolution,
            num_views: numViews,
            seed,
            remove_background: removeBackground,
            foreground_ratio: foregroundRatio,
            texture_size: textureSize,
            output_format: outputFormat,
            enable_texture: enableTexture,
        };
        
        // Encode reference image as base64
        if (referenceImage) {
            const base64 = referenceImage.split(',')[1];
            req.image_base64 = base64;
        }
        
        try {
            await generate3DModel(
                req,
                (step, totalSteps, percentage, status) => {
                    progress = { step, totalSteps, percentage, status };
                },
                (model) => {
                    lastGenerated = model;
                    isGenerating = false;
                    progress = null;
                },
                (msg) => {
                    error = msg;
                    isGenerating = false;
                    progress = null;
                },
            );
        } catch (e) {
            error = e instanceof Error ? e.message : 'Generation failed';
            isGenerating = false;
            progress = null;
        }
    }
</script>

<div class="generator">
    <div class="generator-form">
        <!-- Model -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label>Model</label>
            <div class="model-badge">🧊 Hunyuan3D 2.1</div>
            <span class="model-hint">Image-to-3D (Tencent)</span>
        </div>
        
        <!-- Reference Image -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="has-tooltip" data-tooltip="The source image that Hunyuan3D will convert into a 3D model. Best results with a single object on a clean background.">Reference Image</label>
            <div class="reference-area">
                {#if referenceImage}
                    <div class="reference-preview">
                        <img src={referenceImage} alt="Reference" />
                        <button class="clear-ref" onclick={clearReference} title="Remove reference">×</button>
                    </div>
                {:else}
                    <button class="upload-btn" onclick={() => fileInput.click()} disabled={isGenerating}>
                        📷 Upload Reference Image
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
        
        <!-- Render Texture -->
        <div class="form-section">
            <label class="render-toggle texture-toggle has-tooltip" data-tooltip="Paints PBR textures (colour, metallic, roughness) onto the mesh from multiple camera views. Adds ~60-120s to generation time.">
                <input type="checkbox" bind:checked={enableTexture} />
                Render Texture
            </label>
        </div>
        
        <!-- Output Format -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="has-tooltip" data-tooltip="GLB is a self-contained binary format with embedded textures — best for web viewers. OBJ is a plain mesh format without embedded materials.">Output Format</label>
            <div class="aspect-buttons">
                <button 
                    class="aspect-btn"
                    class:active={outputFormat === 'glb'}
                    onclick={() => outputFormat = 'glb'}
                    disabled={isGenerating}
                >
                    GLB
                </button>
                <button 
                    class="aspect-btn"
                    class:active={outputFormat === 'obj'}
                    onclick={() => outputFormat = 'obj'}
                    disabled={isGenerating}
                >
                    OBJ
                </button>
            </div>
        </div>
        
        <!-- Render Preview -->
        <div class="form-section">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label class="has-tooltip" data-tooltip="Controls how the 3D model preview displays after generation. These settings don't affect the output file.">Preview Rendering</label>
            <div class="render-form-controls">
                <label class="render-toggle">
                    <input type="checkbox" bind:checked={renderEnabled} />
                    Environment Lighting
                </label>
                <label class="render-toggle">
                    <input type="checkbox" bind:checked={renderAutoRotate} />
                    Auto-Rotate
                </label>
            </div>
            {#if renderEnabled}
                <div class="render-form-sliders">
                    <div class="render-slider">
                        <span>Exposure ({renderExposure.toFixed(1)})</span>
                        <input type="range" min="0.2" max="2" step="0.1" bind:value={renderExposure} />
                    </div>
                    <div class="render-slider">
                        <span>Shadow ({renderShadow.toFixed(1)})</span>
                        <input type="range" min="0" max="2" step="0.1" bind:value={renderShadow} />
                    </div>
                </div>
            {/if}
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
                        <label class="has-tooltip" data-tooltip="Number of diffusion denoising steps. More steps = higher quality but slower. 50 is a good default; 30 is faster, 100+ is diminishing returns.">Steps</label>
                        <input type="number" bind:value={steps} min="10" max="200" disabled={isGenerating} />
                    </div>
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label class="has-tooltip" data-tooltip="How strictly the model follows the reference image. Higher values (10-15) match the input more closely; lower values (3-5) allow more creative freedom.">Guidance Scale</label>
                        <input type="number" bind:value={guidanceScale} min="1" max="20" step="0.5" disabled={isGenerating} />
                    </div>
                </div>
                <div class="option-row">
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label class="has-tooltip" data-tooltip="Controls mesh polygon density. Higher = more geometric detail but larger file size and slower generation. 256 balances quality and speed.">Octree Resolution</label>
                        <select bind:value={octreeResolution} disabled={isGenerating}>
                            <option value={128}>128 (fast)</option>
                            <option value={256}>256 (default)</option>
                            <option value={384}>384 (detail)</option>
                            <option value={512}>512 (high)</option>
                        </select>
                    </div>
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label class="has-tooltip" data-tooltip="Number of camera angles used during texture painting. More views = better texture coverage but slower. 4-6 views works well for most objects.">Views</label>
                        <input type="number" bind:value={numViews} min="4" max="12" disabled={isGenerating} />
                    </div>
                </div>
                <div class="option-row">
                    <div class="option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label class="has-tooltip" data-tooltip="Resolution of the painted texture map in pixels. Higher = sharper surface detail but more VRAM usage. 1024 is recommended for 16 GB VRAM systems.">Texture Size</label>
                        <select bind:value={textureSize} disabled={isGenerating}>
                            <option value={512}>512</option>
                            <option value={1024}>1024 (default)</option>
                            <option value={2048}>2048 (high)</option>
                        </select>
                    </div>
                    <div class="option seed-option">
                        <!-- svelte-ignore a11y_label_has_associated_control -->
                        <label class="has-tooltip" data-tooltip="Fixed seed for reproducible results. Use -1 for a random seed each time. Set a specific value to regenerate the same shape.">Seed (-1 = random)</label>
                        <div class="seed-input">
                            <input type="number" bind:value={seed} disabled={isGenerating} />
                            <button class="dice-btn" onclick={randomizeSeed} disabled={isGenerating} title="Randomize">🎲</button>
                        </div>
                    </div>
                </div>
                {#if referenceImage}
                    <div class="option-row">
                        <div class="option">
                            <label class="has-tooltip" data-tooltip="Automatically removes the background from the reference image using rembg. Recommended for photos with busy backgrounds.">
                                <input type="checkbox" bind:checked={removeBackground} disabled={isGenerating} />
                                Remove Background
                            </label>
                        </div>
                        {#if removeBackground}
                            <div class="option">
                                <!-- svelte-ignore a11y_label_has_associated_control -->
                                <label class="has-tooltip" data-tooltip="How much of the image to keep after background removal. Lower values crop tighter around the subject; 0.9 leaves some padding.">Foreground Ratio ({foregroundRatio})</label>
                                <input type="range" min="0.5" max="1" step="0.05" bind:value={foregroundRatio} disabled={isGenerating} />
                            </div>
                        {/if}
                    </div>
                {/if}
            </div>
        {/if}
        
        <!-- Generate Button -->
        <button 
            class="generate-btn" 
            onclick={handleGenerate}
            disabled={isGenerating || !referenceImage}
        >
            {#if isGenerating}
                ⏳ Generating...
            {:else}
                🧊 Generate 3D Model
            {/if}
        </button>
        
        <!-- Progress Bar -->
        {#if progress}
            <div class="progress-container">
                <div class="progress-bar">
                    <div class="progress-fill" style="width: {progress.percentage}%"></div>
                </div>
                <span class="progress-text">
                    {statusLabel(progress.status)} — Step {progress.step} / {progress.totalSteps} ({Math.round(progress.percentage)}%)
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
        {#if lastGenerated}
            <div class="generated-result">
                <div class="model-viewer-container">
                    <model-viewer
                        src={`http://localhost:3000${lastGenerated.url}`}
                        alt={lastGenerated.prompt || lastGenerated.filename}
                        camera-controls
                        auto-rotate={renderAutoRotate ? true : undefined}
                        environment-image={renderEnabled ? 'neutral' : undefined}
                        shadow-intensity={renderEnabled ? String(renderShadow) : '0'}
                        shadow-softness={renderEnabled ? '0.8' : '0'}
                        exposure={String(renderExposure)}
                        tone-mapping={renderEnabled ? 'commerce' : undefined}
                        style="width: 100%; height: 400px;"
                    ></model-viewer>
                </div>
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
                <div class="result-info">
                    <p class="result-prompt">{lastGenerated.prompt || lastGenerated.filename}</p>
                    <p class="result-meta">
                        {lastGenerated.format.toUpperCase()} · {(lastGenerated.file_size / 1024 / 1024).toFixed(2)} MB
                        {#if lastGenerated.seed} · Seed: {lastGenerated.seed}{/if}
                    </p>
                    <a
                        href={`http://localhost:3000${lastGenerated.url}`}
                        download={lastGenerated.filename}
                        class="download-btn"
                    >
                        ⬇ Download {lastGenerated.format.toUpperCase()}
                    </a>
                </div>
            </div>
        {:else if isGenerating}
            <div class="generating-placeholder">
                <div class="spinner"></div>
                <p>Creating your 3D model...</p>
                {#if progress}
                    <p class="gen-status">{statusLabel(progress.status)}</p>
                {/if}
            </div>
        {:else}
            <div class="empty-preview">
                <span class="preview-icon">🧊</span>
                <p>Your generated 3D model will appear here</p>
            </div>
        {/if}
    </div>
</div>

<style>
    /* ── Tooltips ──────────────────────────────────────────────────────── */
    .has-tooltip {
        position: relative;
        cursor: help;
        border-bottom: 1px dotted var(--text-tertiary);
        width: fit-content;
    }

    .has-tooltip::after {
        content: attr(data-tooltip);
        position: absolute;
        bottom: calc(100% + 8px);
        left: 0;
        max-width: 280px;
        padding: 0.5rem 0.75rem;
        background: var(--bg-primary, #1a1a2e);
        border: 1px solid var(--border-color, #333);
        border-radius: 6px;
        color: var(--text-primary, #e0e0e0);
        font-size: 0.75rem;
        font-weight: 400;
        line-height: 1.4;
        white-space: normal;
        z-index: 100;
        pointer-events: none;
        opacity: 0;
        transition: opacity 0.15s ease;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    }

    .has-tooltip:hover::after {
        opacity: 1;
    }

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
    
    textarea, input[type="number"], select {
        background: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 8px;
        padding: 0.75rem;
        color: var(--text-primary);
        font-size: 0.875rem;
        resize: vertical;
    }
    
    textarea:focus, input:focus, select:focus {
        outline: none;
        border-color: var(--accent-primary);
    }
    
    textarea:disabled, input:disabled, select:disabled {
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
    
    .option input, .option select {
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
    
    .gen-status {
        font-size: 0.75rem;
        color: var(--text-tertiary);
        margin-top: 0.5rem;
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
    
    .model-viewer-container {
        width: 100%;
        border-radius: 8px;
        overflow: hidden;
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
    
    .download-btn {
        display: inline-block;
        margin-top: 0.75rem;
        padding: 0.5rem 1rem;
        background: var(--accent-primary);
        border-radius: 6px;
        color: white;
        text-decoration: none;
        font-size: 0.875rem;
        font-weight: 500;
        transition: filter 0.15s ease;
    }
    
    .download-btn:hover {
        filter: brightness(1.1);
    }
    
    .render-controls {
        display: flex;
        align-items: center;
        gap: 1rem;
        padding: 0.5rem 0.75rem;
        background: rgba(0, 0, 0, 0.3);
        border-radius: 0 0 8px 8px;
        flex-wrap: wrap;
    }
    
    .render-form-controls {
        display: flex;
        gap: 1.25rem;
        flex-wrap: wrap;
    }
    
    .render-form-sliders {
        display: flex;
        gap: 1.25rem;
        flex-wrap: wrap;
        margin-top: 0.25rem;
    }
    
    .render-toggle {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        font-size: 0.75rem;
        color: var(--text-secondary);
        cursor: pointer;
        user-select: none;
    }
    
    .render-toggle input[type="checkbox"] {
        accent-color: var(--accent-primary);
    }
    
    .texture-toggle {
        font-size: 0.85rem;
        color: var(--text-primary);
        gap: 0.5rem;
    }
    
    .render-slider {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        font-size: 0.7rem;
        color: var(--text-tertiary);
    }
    
    .render-slider input[type="range"] {
        width: 60px;
        height: 4px;
        accent-color: var(--accent-primary);
    }
</style>
