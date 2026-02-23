<script lang="ts">
    import { appState, type Persona } from '$lib/store.svelte';
    import { slide, fade } from 'svelte/transition';
    import { marked } from 'marked';
    
    // Render system prompt markdown
    function renderProfile(md: string): string {
        try {
            return marked.parse(md, { breaks: true, gfm: true }) as string;
        } catch {
            return md;
        }
    }
    
    let { persona, onclose }: { persona: Persona; onclose: () => void } = $props();
    
    let isAi = $derived(persona.type === 'ai');
    
    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            onclose();
        }
    }
    
    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            onclose();
        }
    }
    
    function handleEdit() {
        appState.openPersonaEditor(persona);
        onclose();
    }
    
    // Get tag by ID
    function getTagById(tagId: string) {
        return appState.tags.find(t => t.id === tagId);
    }
    
    // Get emoji for mood display
    function getMoodEmoji(mood: string): string {
        const moodEmojis: Record<string, string> = {
            'idle': '😌',
            'thinking': '🤔',
            'surprised': '😮',
            'happy': '😊',
            'content': '😌',
            'thoughtful': '🤔',
            'melancholy': '😢',
            'curious': '🧐',
            'excited': '🤩',
            'calm': '😊',
            'concerned': '😟',
        };
        return moodEmojis[mood] || '💭';
    }
    
    // Get the current mood for this persona
    let personaMood = $derived(persona.currentMood || appState.currentMood || 'content');
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="modal-backdrop" onclick={handleBackdropClick} role="button" tabindex="-1" transition:fade={{ duration: 150 }}>
    <div class="modal-content" transition:slide={{ duration: 200 }}>
        <!-- Header with avatar -->
        <div class="profile-header">
            <div class="avatar-large" class:ai-avatar={isAi} class:user-avatar={!isAi}>
                {#if isAi}
                    <span class="avatar-icon">✦</span>
                {:else}
                    <span class="avatar-icon">⚡</span>
                {/if}
            </div>
            <div class="header-info">
                <h2 class="profile-name">{persona.name}</h2>
                <p class="profile-type">{isAi ? 'AI Persona' : 'User Persona'}</p>
            </div>
            <button class="header-edit-btn" onclick={handleEdit} title="Edit Persona">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
                    <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
                </svg>
            </button>
            <button class="close-btn" onclick={onclose} title="Close">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M6 18L18 6M6 6l12 12"/>
                </svg>
            </button>
        </div>
        
        <!-- Status indicator (AI only) -->
        {#if isAi}
            <div class="status-section">
                <div class="status-indicator">
                    <div class="status-dot" class:awake={appState.status === 'awake'} class:thinking={appState.status === 'thinking'} class:dreaming={appState.status === 'dreaming'}></div>
                    <span class="status-text">{appState.status}</span>
                </div>
                <div class="mood-indicator">
                    <span class="mood-emoji">{getMoodEmoji(personaMood)}</span>
                    <span class="mood-text">{personaMood}</span>
                    <div class="mood-bar">
                        <div class="mood-fill" style="width: {appState.mood * 100}%"></div>
                    </div>
                </div>
            </div>
        {/if}
        
        <!-- Tags -->
        {#if persona.tags && persona.tags.length > 0}
            <div class="section tags-section">
                <h3 class="section-title">Tags</h3>
                <div class="tags-list">
                    {#each persona.tags as tagId}
                        {@const tag = getTagById(tagId)}
                        {#if tag}
                            <span class="tag-badge" style="--tag-color: {tag.color}">{tag.name}</span>
                        {/if}
                    {/each}
                </div>
            </div>
        {/if}
        
        <!-- Description -->
        <div class="section">
            <h3 class="section-title">Description</h3>
            <p class="section-content">{persona.description || 'No description provided.'}</p>
        </div>
        
        <!-- Profile (AI only) -->
        {#if isAi && persona.systemPrompt}
            <div class="section">
                <h3 class="section-title">Profile</h3>
                <div class="section-content prompt markdown-body">
                    {@html renderProfile(persona.systemPrompt)}
                </div>
            </div>
        {/if}
        
        <!-- Metadata -->
        {#if Object.keys(persona.metadata).length > 0}
            <div class="section">
                <h3 class="section-title">Attributes</h3>
                <div class="metadata-grid">
                    {#each Object.entries(persona.metadata) as [key, value]}
                        <div class="metadata-item">
                            <span class="metadata-key">{key}</span>
                            <span class="metadata-value">{value}</span>
                        </div>
                    {/each}
                </div>
            </div>
        {/if}
        
        <!-- Timestamps -->
        <div class="section timestamps">
            <div class="timestamp">
                <span class="timestamp-label">Created</span>
                <span class="timestamp-value">{new Date(persona.createdAt).toLocaleDateString()}</span>
            </div>
            <div class="timestamp">
                <span class="timestamp-label">Updated</span>
                <span class="timestamp-value">{new Date(persona.updatedAt).toLocaleDateString()}</span>
            </div>
        </div>
        

    </div>
</div>

<style>
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.6);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
    }
    
    .modal-content {
        background: linear-gradient(135deg, rgba(28, 30, 31, 0.98), rgba(24, 26, 27, 0.98));
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 1rem;
        padding: 1.5rem;
        width: 90%;
        max-width: 420px;
        max-height: 85vh;
        overflow-y: auto;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    }
    
    .profile-header {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin-bottom: 1.5rem;
        padding-bottom: 1rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
    }
    
    .avatar-large {
        width: 64px;
        height: 64px;
        border-radius: 1rem;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
    }
    
    .avatar-large.ai-avatar {
        background: linear-gradient(135deg, rgba(255, 140, 66, 0.25), rgba(255, 107, 0, 0.35));
        border: 2px solid rgba(255, 140, 66, 0.5);
        box-shadow: 0 0 20px rgba(255, 140, 66, 0.25);
    }
    
    .avatar-large.user-avatar {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.25), rgba(192, 80, 210, 0.35));
        border: 2px solid rgba(232, 121, 249, 0.5);
        box-shadow: 0 0 20px rgba(232, 121, 249, 0.25);
    }
    
    .avatar-icon {
        font-size: 1.75rem;
    }
    
    .ai-avatar .avatar-icon {
        color: rgba(255, 180, 130, 1);
    }
    
    .user-avatar .avatar-icon {
        color: rgba(150, 200, 255, 1);
    }
    
    .header-info {
        flex: 1;
    }
    
    .profile-name {
        font-size: 1.5rem;
        font-weight: 700;
        background: linear-gradient(90deg, rgba(232, 234, 236, 1), rgba(180, 210, 240, 1));
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        background-clip: text;
        margin: 0;
    }
    
    .profile-type {
        color: rgba(139, 146, 149, 0.8);
        font-size: 0.875rem;
        margin: 0.25rem 0 0;
    }
    
    .header-edit-btn {
        width: 32px;
        height: 32px;
        padding: 0;
        background: rgba(232, 121, 249, 0.15);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 0.5rem;
        color: rgba(232, 121, 249, 0.8);
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .header-edit-btn:hover {
        background: rgba(232, 121, 249, 0.25);
        border-color: rgba(232, 121, 249, 0.5);
        color: rgba(232, 121, 249, 1);
    }
    
    .header-edit-btn svg {
        width: 16px;
        height: 16px;
    }
    
    .close-btn {
        width: 32px;
        height: 32px;
        padding: 0;
        background: rgba(232, 121, 249, 0.1);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(180, 210, 240, 0.7);
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .close-btn:hover {
        background: rgba(232, 121, 249, 0.2);
        border-color: rgba(232, 121, 249, 0.4);
        color: rgba(232, 234, 236, 0.9);
    }
    
    .close-btn svg {
        width: 18px;
        height: 18px;
    }
    
    .status-section {
        margin-bottom: 1.25rem;
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
    }
    
    .status-indicator {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 2rem;
    }
    
    .mood-indicator {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: rgba(255, 200, 100, 0.08);
        border: 1px solid rgba(255, 200, 100, 0.2);
        border-radius: 2rem;
        flex: 1;
        min-width: 140px;
    }
    
    .mood-emoji {
        font-size: 1rem;
    }
    
    .mood-text {
        color: rgba(255, 200, 100, 0.9);
        font-size: 0.875rem;
        text-transform: capitalize;
    }
    
    .mood-bar {
        flex: 1;
        height: 6px;
        background: rgba(255, 200, 100, 0.15);
        border-radius: 3px;
        overflow: hidden;
        min-width: 40px;
    }
    
    .mood-fill {
        height: 100%;
        background: linear-gradient(90deg, rgba(255, 200, 100, 0.6), rgba(255, 180, 80, 0.8));
        border-radius: 3px;
        transition: width 0.3s ease;
    }
    
    .status-dot {
        width: 10px;
        height: 10px;
        border-radius: 50%;
    }
    
    .status-dot.awake {
        background: rgba(52, 211, 153, 1);
        box-shadow: 0 0 8px rgba(52, 211, 153, 0.5);
    }
    
    .status-dot.thinking {
        background: rgba(96, 165, 250, 1);
        box-shadow: 0 0 8px rgba(96, 165, 250, 0.5);
        animation: pulse 1.5s infinite;
    }
    
    .status-dot.dreaming {
        background: rgba(232, 121, 249, 1);
        box-shadow: 0 0 8px rgba(232, 121, 249, 0.5);
        animation: pulse 2s infinite;
    }
    
    @keyframes pulse {
        0%, 100% { opacity: 1; transform: scale(1); }
        50% { opacity: 0.6; transform: scale(0.9); }
    }
    
    .status-text {
        color: rgba(180, 210, 240, 0.9);
        font-size: 0.875rem;
        text-transform: capitalize;
    }
    
    .section {
        margin-bottom: 1.25rem;
    }
    
    .section-title {
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: rgba(232, 121, 249, 0.7);
        margin: 0 0 0.5rem;
    }
    
    .section-content {
        color: rgba(232, 234, 236, 0.85);
        font-size: 0.9375rem;
        line-height: 1.6;
        margin: 0;
    }
    
    .section-content.prompt {
        font-size: 0.875rem;
        padding: 0.75rem 1rem;
        background: rgba(20, 22, 23, 0.8);
        border-radius: 0.5rem;
        border: 1px solid rgba(232, 121, 249, 0.12);
        max-height: 400px;
        overflow-y: auto;
        line-height: 1.7;
    }
    
    .section-content.prompt :global(h1),
    .section-content.prompt :global(h2),
    .section-content.prompt :global(h3) {
        color: rgba(232, 121, 249, 0.85);
        margin: 0.75rem 0 0.25rem;
        font-size: 1rem;
    }
    
    .section-content.prompt :global(h1) { font-size: 1.1rem; }
    .section-content.prompt :global(h2) { font-size: 1rem; }
    .section-content.prompt :global(h3) { font-size: 0.9rem; }
    
    .section-content.prompt :global(ul),
    .section-content.prompt :global(ol) {
        padding-left: 1.25rem;
        margin: 0.25rem 0;
    }
    
    .section-content.prompt :global(li) {
        margin: 0.15rem 0;
    }
    
    .section-content.prompt :global(p) {
        margin: 0.35rem 0;
    }
    
    .section-content.prompt :global(strong) {
        color: rgba(232, 234, 236, 1);
    }
    
    .metadata-grid {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }
    
    .metadata-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.375rem 0.625rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.15);
        border-radius: 0.375rem;
    }
    
    .metadata-key {
        color: rgba(232, 121, 249, 0.7);
        font-size: 0.75rem;
        text-transform: capitalize;
    }
    
    .metadata-value {
        color: rgba(232, 234, 236, 0.9);
        font-size: 0.8125rem;
    }
    
    .timestamps {
        display: flex;
        gap: 1.5rem;
        padding-top: 1rem;
        border-top: 1px solid rgba(232, 121, 249, 0.12);
    }
    
    .timestamp {
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
    }
    
    .timestamp-label {
        font-size: 0.6875rem;
        text-transform: uppercase;
        color: rgba(139, 146, 149, 0.6);
    }
    
    .timestamp-value {
        font-size: 0.8125rem;
        color: rgba(180, 210, 240, 0.8);
    }
    
    /* Tags Section */
    .tags-section {
        padding-bottom: 0.75rem;
    }
    
    .tags-list {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }
    
    .tag-badge {
        display: inline-flex;
        align-items: center;
        padding: 0.35rem 0.75rem;
        background: var(--tag-color);
        border-radius: 9999px;
        color: white;
        font-size: 0.75rem;
        font-weight: 500;
        box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
    }
</style>
