<script>
    import { marked } from 'marked';
    import { slide } from 'svelte/transition';
    import { appState } from '$lib/store.svelte';
    import { speak, stop, getIsSpeaking, getIsLoading, getCurrentMessageId } from '$lib/tts_service';
    
    let { message, isLastUserMessage = false, onviewprofile = () => {} } = $props();
    let thoughtsOpen = $state(false);
    let isSpeakingThis = $state(false);
    let isLoadingThis = $state(false);
    
    // Check speaking/loading state periodically
    $effect(() => {
        const interval = setInterval(() => {
            const isThisMessage = getCurrentMessageId() === message.id;
            isSpeakingThis = getIsSpeaking() && isThisMessage;
            isLoadingThis = getIsLoading() && isThisMessage;
        }, 100);
        return () => clearInterval(interval);
    });

    // Configure marked for better rendering
    marked.setOptions({
        breaks: true,
        gfm: true,
    });

    // Render markdown to HTML
    let renderedContent = $derived(marked(message.content));
    
    // Get persona/model info for watermark
    let personaName = $derived(() => {
        if (message.role === 'assistant') {
            return message.aiPersona || appState.currentAiPersona?.name || 'AI';
        } else {
            return message.userPersona || appState.currentUserPersona?.name || 'You';
        }
    });
    
    let modelName = $derived(() => {
        if (message.role === 'assistant') {
            return message.model || appState.selectedModel || 'llama3';
        }
        return null;
    });
    
    // Get the actual persona object for profile viewing
    let persona = $derived(() => {
        if (message.role === 'assistant') {
            const personaId = message.aiPersona ? appState.aiPersonas.find(p => p.name === message.aiPersona)?.id : appState.currentAiPersonaId;
            return appState.aiPersonas.find(p => p.id === personaId) || appState.currentAiPersona;
        } else {
            const personaId = message.userPersona ? appState.userPersonas.find(p => p.name === message.userPersona)?.id : appState.currentUserPersonaId;
            return appState.userPersonas.find(p => p.id === personaId) || appState.currentUserPersona;
        }
    });
    
    // Get bubble color from persona
    let bubbleColor = $derived(() => {
        const p = persona();
        return p?.bubbleColor || (message.role === 'assistant' ? '#ff8c42' : '#e879f9');
    });
    
    // Get avatar from persona
    let avatarContent = $derived(() => {
        const p = persona();
        return p?.avatar || (message.role === 'assistant' ? '✦' : '⚡');
    });
    
    // Get voice profile from persona
    let voiceProfile = $derived(() => {
        const p = persona();
        return p?.voice || null;
    });
    
    function handleEdit() {
        if (message.id) {
            appState.openMessageEditor(message.id);
        }
    }
    
    function handleAvatarClick() {
        const p = persona();
        if (p) {
            onviewprofile(p);
        }
    }
    
    function handleSpeak() {
        if (isSpeakingThis) {
            stop();
            isSpeakingThis = false;
        } else {
            const p = persona();
            const voiceConfig = voiceProfile();
            
            // Debug: log voice config to understand what we're working with
            console.log('🔊 TTS Debug:', {
                personaName: p?.name,
                personaId: p?.id,
                voiceConfig,
                rawVoice: p?.voice,
            });
            
            // Build profile with persona context and mood for voice modulation
            /** @type {any} */
            const vc = voiceConfig;
            const profile = {
                description: voiceConfig?.description || '',
                pitch: voiceConfig?.pitch || 1.0,
                rate: voiceConfig?.rate || 1.0,
                volume: voiceConfig?.volume || 1.0,
                // AI TTS settings - handle both snake_case (from API) and camelCase
                useAiTts: voiceConfig?.useAiTts ?? vc?.['use_ai_tts'] ?? false,
                voiceSampleUrl: voiceConfig?.voiceSampleUrl ?? vc?.['voice_sample_url'],
                voiceDescription: voiceConfig?.voiceDescription ?? vc?.['voice_description'],
                // Runtime context for TTS
                personaId: p?.id,
                mood: message.mood,  // Pass mood for voice modulation
            };
            
            console.log('🔊 TTS Profile being sent:', profile);
            
            speak(message.content, profile, message.id);
            isSpeakingThis = true;
        }
    }
    
    // Get emoji for mood display
    /** @param {string} mood */
    function getMoodEmoji(mood) {
        /** @type {Record<string, string>} */
        const moodEmojis = {
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
    
    // Check if this is the streaming message (last assistant message while loading)
    let isStreamingMessage = $derived(() => {
        if (message.role !== 'assistant' || !appState.isLoading) return false;
        const messages = appState.messages;
        if (messages.length === 0) return false;
        const last = messages[messages.length - 1];
        return last.id === message.id;
    });
</script>

<!-- Hide the message if it's currently being streamed (shown in ThinkingIndicator instead) -->
{#if !isStreamingMessage()}
<div class="message {message.role}" transition:slide>
    
    <button class="avatar" onclick={handleAvatarClick} title="View profile" style="--avatar-color: {bubbleColor()}">
        {#if avatarContent().startsWith('http') || avatarContent().startsWith('data:')}
            <img class="avatar-img" src={avatarContent()} alt="Avatar" />
        {:else}
            <div class="avatar-icon">{avatarContent()}</div>
        {/if}
    </button>

    <div class="content-col">
        <div class="bubble" style="--bubble-color: {bubbleColor()}">
            {#if message.role === 'assistant' && appState.showThinking}
                <button class="thinking-toggle" onclick={() => thoughtsOpen = !thoughtsOpen}>
                    <svg class="thinking-icon" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M9.5 3A6.5 6.5 0 0 1 16 9.5c0 1.61-.59 3.09-1.56 4.23l.27.27h.79l5 5-1.5 1.5-5-5v-.79l-.27-.27A6.516 6.516 0 0 1 9.5 16 6.5 6.5 0 0 1 3 9.5 6.5 6.5 0 0 1 9.5 3m0 2C7 5 5 7 5 9.5S7 14 9.5 14 14 12 14 9.5 12 5 9.5 5Z"/>
                    </svg>
                    <span>Show thinking</span>
                    <svg class="chevron" class:rotate={thoughtsOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M6 9l6 6 6-6"/>
                    </svg>
                </button>
                
                {#if thoughtsOpen}
                    <div class="thinking-content" transition:slide>
                        {#if message.thinking}
                            <div class="thinking-text">{message.thinking}</div>
                        {:else}
                            <div class="thinking-text thinking-unsupported">(Selected model doesn't support thinking.)</div>
                        {/if}
                    </div>
                {/if}
            {/if}
            
            {#if message.thoughts && message.thoughts.length > 0}
                <div class="thought-container">
                    <button class="thought-toggle" onclick={() => thoughtsOpen = !thoughtsOpen}>
                        <span>{thoughtsOpen ? 'Hide thoughts' : 'View thought process'}</span>
                        <svg class:rotate={thoughtsOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6"/></svg>
                    </button>
                    
                    {#if thoughtsOpen}
                        <div class="thought-list" transition:slide>
                            {#each message.thoughts as thought}
                                <div class="thought-item">
                                    <span class="dot"></span> {thought}
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}

            <div class="text-markdown">
                {@html renderedContent}
            </div>
            
            <!-- Message action buttons -->
            <div class="message-actions">
                <!-- Speak button -->
                <button 
                    class="action-btn speak-btn" 
                    class:speaking={isSpeakingThis}
                    class:loading={isLoadingThis}
                    onclick={handleSpeak} 
                    title={isLoadingThis ? 'Generating audio...' : isSpeakingThis ? 'Stop speaking' : 'Read aloud'}
                    disabled={isLoadingThis}
                >
                    {#if isLoadingThis}
                        <svg class="spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="12" cy="12" r="10" stroke-dasharray="30 30"/>
                        </svg>
                    {:else if isSpeakingThis}
                        <svg viewBox="0 0 24 24" fill="currentColor">
                            <rect x="6" y="6" width="12" height="12" rx="2"/>
                        </svg>
                    {:else}
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M11 5L6 9H2v6h4l5 4V5z"/>
                            <path d="M15.54 8.46a5 5 0 010 7.07"/>
                            <path d="M19.07 4.93a10 10 0 010 14.14"/>
                        </svg>
                    {/if}
                </button>
                
                <!-- Edit button for last user message -->
                {#if isLastUserMessage && message.role === 'user'}
                    <button class="action-btn edit-btn" onclick={handleEdit} title="Edit message (creates new branch)">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.863 4.487zm0 0L19.5 7.125" />
                        </svg>
                    </button>
                {/if}
            </div>
            
            <!-- Watermark - hover to reveal -->
            <div class="watermark">
                <span class="watermark-persona">{personaName()}</span>
                {#if modelName()}
                    <span class="watermark-divider">•</span>
                    <span class="watermark-model">{modelName()}</span>
                {/if}
                {#if message.role === 'assistant' && message.mood}
                    <span class="watermark-divider">•</span>
                    <span class="watermark-mood">{getMoodEmoji(message.mood)} {message.mood}</span>
                {/if}
            </div>
        </div>
    </div>
</div>
{/if}

<style>
    .message {
        display: flex;
        gap: 1rem;
        margin-bottom: 1.5rem;
        max-width: 850px;
        margin-inline: auto;
        animation: slideIn 0.3s ease-out;
    }

    @keyframes slideIn {
        from {
            opacity: 0;
            transform: translateY(10px);
        }
        to {
            opacity: 1;
            transform: translateY(0);
        }
    }

    .message.user { flex-direction: row-reverse; }

    .avatar { 
        width: 2.5rem;  
        flex-shrink: 0; 
        display: flex; 
        align-items: center; 
        justify-content: center;
        background: none;
        border: none;
        padding: 0;
        cursor: pointer;
        transition: transform 0.2s;
    }
    
    .avatar:hover {
        transform: scale(1.1);
    }
    
    .avatar-icon {
        width: 2.5rem;
        height: 2.5rem;
        background: linear-gradient(135deg, color-mix(in srgb, var(--avatar-color) 30%, transparent), color-mix(in srgb, var(--avatar-color) 20%, transparent));
        border: 1px solid var(--avatar-color);
        border-radius: 50%;
        font-size: 1.25rem;
        display: flex;
        align-items: center;
        justify-content: center;
        box-shadow: 0 0 10px color-mix(in srgb, var(--avatar-color) 40%, transparent), inset 0 0 10px color-mix(in srgb, var(--avatar-color) 10%, transparent);
        color: rgba(255, 255, 255, 0.9);
    }
    
    .avatar-img {
        width: 2.5rem;
        height: 2.5rem;
        border-radius: 50%;
        object-fit: cover;
        border: 1px solid var(--avatar-color);
        box-shadow: 0 0 10px color-mix(in srgb, var(--avatar-color) 40%, transparent);
    }

    .content-col {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
    }
    
    .message.user .content-col { align-items: flex-end; }

    .bubble {
        position: relative;
        width: 100%;
        padding: 1rem 1.25rem;
        border-radius: 0.875rem;
        backdrop-filter: blur(10px);
        background: linear-gradient(135deg, rgba(35, 40, 45, 0.8), rgba(28, 30, 31, 0.7));
        border: 1px solid color-mix(in srgb, var(--bubble-color) 35%, transparent);
        box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2), inset 0 1px 2px rgba(255, 255, 255, 0.02);
    }

    /* Thinking Toggle (Gemini-style) */
    .thinking-toggle {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0;
        margin-bottom: 0.5rem;
        background: transparent;
        border: none;
        color: rgba(100, 149, 237, 0.9);
        font-size: 0.8125rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .thinking-toggle:hover {
        color: rgba(135, 180, 255, 1);
    }
    
    .thinking-toggle .thinking-icon {
        width: 16px;
        height: 16px;
    }
    
    .thinking-toggle .chevron {
        width: 14px;
        height: 14px;
        transition: transform 0.2s;
        margin-left: auto;
    }
    
    .thinking-toggle .chevron.rotate {
        transform: rotate(180deg);
    }
    
    .thinking-content {
        border-left: 3px solid rgba(100, 149, 237, 0.5);
        padding-left: 1rem;
        margin-bottom: 1rem;
    }
    
    .thinking-text {
        font-size: 0.8125rem;
        line-height: 1.6;
        color: rgba(180, 200, 220, 0.8);
        white-space: pre-wrap;
        word-break: break-word;
        font-style: italic;
    }
    
    .thinking-unsupported {
        color: rgba(150, 150, 170, 0.6);
        font-size: 0.75rem;
    }

    /* Thought Styles */
    .thought-container {
        background: rgba(232, 121, 249, 0.08);
        border-radius: 0.5rem;
        margin-bottom: 0.75rem;
        overflow: hidden;
        border: 1px solid rgba(232, 121, 249, 0.25);
    }

    .thought-toggle {
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.5rem 0.75rem;
        font-size: 0.75rem;
        color: rgba(232, 121, 249, 0.7);
        font-weight: 600;
        background: rgba(232, 121, 249, 0.05);
        cursor: pointer;
        transition: all 0.2s ease;
    }
    
    .thought-toggle:hover { 
        background: rgba(232, 121, 249, 0.1);
        color: rgba(232, 121, 249, 0.9);
    }

    .thought-toggle svg { width: 1rem; height: 1rem; transition: transform 0.2s; }
    .thought-toggle svg.rotate { transform: rotate(180deg); }

    .thought-list {
        padding: 0.75rem;
        background: rgba(232, 121, 249, 0.05);
        border-top: 1px solid rgba(232, 121, 249, 0.25);
    }

    .thought-item {
        font-size: 0.75rem;
        color: rgba(232, 121, 249, 0.6);
        font-family: monospace;
        margin-bottom: 0.25rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .dot { width: 4px; height: 4px; background: rgba(232, 121, 249, 0.5); border-radius: 50%; }

    .text-markdown {
        line-height: 1.6;
        color: rgba(232, 234, 236, 0.95);
        word-wrap: break-word;
        overflow-wrap: break-word;
    }

    .assistant-bubble .text-markdown {
        color: rgba(232, 234, 236, 0.95);
    }

    .user-bubble .text-markdown {
        color: rgba(200, 220, 240, 0.95);
    }

    /* Markdown Styles */
    :global(.text-markdown h1),
    :global(.text-markdown h2),
    :global(.text-markdown h3),
    :global(.text-markdown h4),
    :global(.text-markdown h5),
    :global(.text-markdown h6) {
        margin: 1rem 0 0.5rem;
        font-weight: 700;
        line-height: 1.4;
    }

    :global(.text-markdown h1) { font-size: 1.875rem; }
    :global(.text-markdown h2) { font-size: 1.5rem; }
    :global(.text-markdown h3) { font-size: 1.25rem; }
    :global(.text-markdown h4) { font-size: 1.1rem; }
    :global(.text-markdown h5) { font-size: 1rem; }
    :global(.text-markdown h6) { font-size: 0.95rem; }

    :global(.text-markdown p) {
        margin: 0.5rem 0;
    }

    :global(.text-markdown code) {
        background: rgba(28, 30, 31, 0.9);
        border: 1px solid rgba(232, 121, 249, 0.25);
        padding: 0.2rem 0.4rem;
        border-radius: 0.25rem;
        font-family: 'Courier New', monospace;
        font-size: 0.9em;
        color: rgba(240, 180, 255, 0.95);
    }

    :global(.text-markdown pre) {
        background: rgba(20, 22, 23, 0.95);
        border: 1px solid rgba(232, 121, 249, 0.25);
        border-radius: 0.5rem;
        padding: 1rem;
        overflow-x: auto;
        margin: 0.75rem 0;
    }

    :global(.text-markdown pre code) {
        background: transparent;
        border: none;
        padding: 0;
        color: rgba(220, 200, 240, 0.95);
        font-size: 0.875em;
        line-height: 1.5;
    }

    :global(.text-markdown ul),
    :global(.text-markdown ol) {
        margin: 0.75rem 0;
        padding-left: 1.5rem;
    }

    :global(.text-markdown li) {
        margin: 0.25rem 0;
    }

    :global(.text-markdown blockquote) {
        border-left: 4px solid rgba(232, 121, 249, 0.5);
        padding-left: 1rem;
        margin: 0.75rem 0;
        color: rgba(232, 121, 249, 0.8);
        font-style: italic;
    }

    :global(.text-markdown a) {
        color: rgba(232, 121, 249, 0.9);
        text-decoration: underline;
        transition: color 0.2s;
    }

    :global(.text-markdown a:hover) {
        color: rgba(250, 200, 255, 0.95);
    }

    :global(.text-markdown strong) {
        font-weight: 700;
        color: rgba(232, 234, 236, 0.95);
    }

    :global(.text-markdown em) {
        font-style: italic;
        color: rgba(200, 180, 220, 0.9);
    }

    :global(.text-markdown hr) {
        border: none;
        border-top: 1px solid rgba(232, 121, 249, 0.25);
        margin: 1rem 0;
    }

    :global(.text-markdown table) {
        border-collapse: collapse;
        width: 100%;
        margin: 1rem 0;
    }

    :global(.text-markdown th),
    :global(.text-markdown td) {
        border: 1px solid rgba(232, 121, 249, 0.25);
        padding: 0.5rem;
        text-align: left;
    }

    :global(.text-markdown th) {
        background: rgba(232, 121, 249, 0.1);
        font-weight: 700;
    }
    
    /* Message action buttons container */
    .message-actions {
        position: absolute;
        top: 0.5rem;
        right: 0.5rem;
        display: flex;
        gap: 0.375rem;
        opacity: 0;
        transition: opacity 0.2s;
    }
    
    .bubble:hover .message-actions {
        opacity: 1;
    }
    
    .action-btn {
        width: 28px;
        height: 28px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(232, 121, 249, 0.2);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 0.375rem;
        color: rgba(240, 180, 255, 0.8);
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .action-btn svg {
        width: 14px;
        height: 14px;
    }
    
    .action-btn:hover {
        background: rgba(232, 121, 249, 0.35);
        border-color: rgba(232, 121, 249, 0.5);
        color: rgba(250, 200, 255, 0.95);
        transform: scale(1.05);
    }
    
    /* Speak button states */
    .speak-btn.speaking {
        background: rgba(74, 158, 255, 0.3);
        border-color: rgba(74, 158, 255, 0.5);
        color: #4a9eff;
        animation: pulse-speak 1.5s ease-in-out infinite;
    }
    
    .speak-btn.loading {
        background: rgba(255, 140, 66, 0.3);
        border-color: rgba(255, 140, 66, 0.5);
        color: #ff8c42;
        cursor: wait;
    }
    
    .speak-btn .spin {
        animation: spin 1s linear infinite;
    }
    
    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }
    
    @keyframes pulse-speak {
        0%, 100% { box-shadow: 0 0 0 0 rgba(74, 158, 255, 0.4); }
        50% { box-shadow: 0 0 0 6px rgba(74, 158, 255, 0); }
    }

    .bubble {
        position: relative;
    }

    /* Watermark - hover to reveal */
    .watermark {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        margin-top: 0.75rem;
        padding-top: 0.5rem;
        border-top: 1px solid transparent;
        font-size: 0.65rem;
        opacity: 0;
        transform: translateY(-3px);
        transition: all 0.2s ease-out;
        color: rgba(139, 146, 149, 0.6);
    }
    
    .bubble:hover .watermark {
        opacity: 1;
        transform: translateY(0);
        border-top-color: rgba(232, 121, 249, 0.1);
    }
    
    .watermark-persona {
        font-weight: 600;
        color: rgba(255, 140, 66, 0.7);
    }
    
    .watermark-divider {
        color: rgba(232, 121, 249, 0.4);
    }
    
    .watermark-model {
        font-family: 'Consolas', monospace;
        font-size: 0.6rem;
        padding: 0.1rem 0.35rem;
        background: rgba(232, 121, 249, 0.12);
        border-radius: 0.25rem;
        color: rgba(232, 121, 249, 0.7);
    }
    
    .watermark-mood {
        font-size: 0.6rem;
        padding: 0.1rem 0.35rem;
        background: rgba(255, 200, 100, 0.12);
        border-radius: 0.25rem;
        color: rgba(255, 200, 100, 0.8);
        text-transform: capitalize;
    }
    
    .message.user .watermark {
        justify-content: flex-end;
    }
</style>