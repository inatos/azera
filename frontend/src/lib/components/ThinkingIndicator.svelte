<script lang="ts">
    import { appState } from '$lib/store.svelte';
    import { slide } from 'svelte/transition';
    import { marked } from 'marked';
    
    let isExpanded = $state(true);
    
    // Get the last assistant message (which is being streamed)
    let streamingMessage = $derived(() => {
        const messages = appState.messages;
        if (messages.length === 0) return null;
        const last = messages[messages.length - 1];
        // Only return if it's an assistant message and we're loading
        if (last.role === 'assistant' && appState.isLoading) {
            return last;
        }
        return null;
    });
    
    // Render markdown
    let renderedContent = $derived(() => {
        const msg = streamingMessage();
        if (!msg?.content) return '';
        return marked(msg.content, { breaks: true, gfm: true });
    });
    
    // Auto-collapse when loading completes
    $effect(() => {
        if (!appState.isLoading && streamingMessage()) {
            // Delay collapse slightly
            setTimeout(() => {
                isExpanded = false;
            }, 300);
        }
    });
    
    // Auto-expand when loading starts
    $effect(() => {
        if (appState.isLoading) {
            isExpanded = true;
        }
    });
    
    function toggleExpanded() {
        isExpanded = !isExpanded;
    }
    
    // Format elapsed time
    let startTime = $state(Date.now());
    let elapsedSeconds = $state(0);
    
    $effect(() => {
        if (appState.isLoading) {
            startTime = Date.now();
            const interval = setInterval(() => {
                elapsedSeconds = Math.floor((Date.now() - startTime) / 1000);
            }, 100);
            return () => clearInterval(interval);
        } else {
            // Keep the final time when done
        }
    });
</script>

{#if appState.isLoading}
    <div class="thinking-container" transition:slide={{ duration: 200 }}>
        <button class="thinking-header" onclick={toggleExpanded}>
            <div class="thinking-icon active">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 2a7 7 0 017 7c0 2.38-1.19 4.47-3 5.74V17a2 2 0 01-2 2H10a2 2 0 01-2-2v-2.26C6.19 13.47 5 11.38 5 9a7 7 0 017-7z"/>
                    <path d="M9 22h6"/>
                    <path d="M10 22v-3"/>
                    <path d="M14 22v-3"/>
                </svg>
            </div>
            <span class="thinking-label">
                {#if streamingMessage()?.content}
                    Generating<span class="dots"><span>.</span><span>.</span><span>.</span></span>
                {:else}
                    Thinking<span class="dots"><span>.</span><span>.</span><span>.</span></span>
                {/if}
                <span class="elapsed">{elapsedSeconds}s</span>
            </span>
            <svg class="chevron" class:rotate={isExpanded} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M6 9l6 6 6-6"/>
            </svg>
        </button>
        
        {#if isExpanded}
            <div class="thinking-content" transition:slide={{ duration: 200 }}>
                {#if streamingMessage()?.content}
                    <div class="streaming-text">
                        {@html renderedContent()}
                        <span class="cursor">▌</span>
                    </div>
                {:else}
                    <div class="thinking-placeholder">
                        <span class="shimmer"></span>
                    </div>
                {/if}
            </div>
        {/if}
    </div>
{/if}

<style>
    .thinking-container {
        margin-bottom: 1rem;
        max-width: 850px;
        margin-inline: auto;
    }
    
    .thinking-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        width: 100%;
        padding: 0.625rem 0.875rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(240, 200, 255, 0.9);
        font-size: 0.8125rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .thinking-header:hover {
        background: rgba(232, 121, 249, 0.12);
        border-color: rgba(232, 121, 249, 0.3);
    }
    
    .thinking-icon {
        width: 18px;
        height: 18px;
        color: rgba(232, 121, 249, 0.7);
        transition: all 0.3s;
    }
    
    .thinking-icon.active {
        color: rgba(232, 121, 249, 1);
        animation: think-pulse 1.2s ease-in-out infinite, brain-rotate 3s ease-in-out infinite;
    }
    
    @keyframes think-pulse {
        0%, 100% {
            filter: drop-shadow(0 0 3px rgba(232, 121, 249, 0.4));
            transform: scale(1);
        }
        25% {
            filter: drop-shadow(0 0 10px rgba(232, 121, 249, 0.8));
            transform: scale(1.1);
        }
        50% {
            filter: drop-shadow(0 0 15px rgba(232, 121, 249, 1));
            transform: scale(1.15);
        }
        75% {
            filter: drop-shadow(0 0 10px rgba(232, 121, 249, 0.8));
            transform: scale(1.1);
        }
    }
    
    @keyframes brain-rotate {
        0%, 100% {
            transform: rotate(-3deg) scale(1);
        }
        25% {
            transform: rotate(2deg) scale(1.1);
        }
        50% {
            transform: rotate(-2deg) scale(1.15);
        }
        75% {
            transform: rotate(3deg) scale(1.1);
        }
    }
    
    .thinking-icon svg {
        width: 100%;
        height: 100%;
    }
    
    .thinking-label {
        flex: 1;
        text-align: left;
        font-weight: 500;
    }
    
    .dots {
        display: inline-flex;
    }
    
    .dots span {
        animation: dot-bounce 1.4s ease-in-out infinite;
    }
    
    .dots span:nth-child(1) { animation-delay: 0s; }
    .dots span:nth-child(2) { animation-delay: 0.2s; }
    .dots span:nth-child(3) { animation-delay: 0.4s; }
    
    @keyframes dot-bounce {
        0%, 80%, 100% { 
            opacity: 0.3;
            transform: translateY(0);
        }
        40% { 
            opacity: 1;
            transform: translateY(-2px);
        }
    }
    
    .elapsed {
        margin-left: 0.5rem;
        font-size: 0.75rem;
        color: rgba(139, 146, 149, 0.6);
        font-weight: 400;
    }
    
    .chevron {
        width: 16px;
        height: 16px;
        color: rgba(232, 121, 249, 0.5);
        transition: transform 0.2s;
    }
    
    .chevron.rotate {
        transform: rotate(180deg);
    }
    
    .thinking-content {
        margin-top: 0.5rem;
        padding: 0.875rem;
        background: rgba(20, 22, 23, 0.8);
        border: 1px solid rgba(232, 121, 249, 0.12);
        border-radius: 0.5rem;
        max-height: 300px;
        overflow-y: auto;
    }
    
    .streaming-text {
        font-size: 0.85rem;
        line-height: 1.6;
        color: rgba(232, 234, 236, 0.9);
    }
    
    .streaming-text :global(p) {
        margin: 0 0 0.5rem 0;
    }
    
    .streaming-text :global(p:last-child) {
        margin-bottom: 0;
    }
    
    .streaming-text :global(code) {
        background: rgba(232, 121, 249, 0.1);
        padding: 0.1rem 0.3rem;
        border-radius: 3px;
        font-size: 0.8rem;
    }
    
    .streaming-text :global(pre) {
        background: rgba(20, 22, 23, 0.9);
        padding: 0.75rem;
        border-radius: 0.5rem;
        overflow-x: auto;
        margin: 0.5rem 0;
    }
    
    .cursor {
        display: inline-block;
        color: rgba(232, 121, 249, 0.8);
        animation: cursor-blink 0.8s ease-in-out infinite;
        margin-left: 2px;
    }
    
    @keyframes cursor-blink {
        0%, 50% { opacity: 1; }
        51%, 100% { opacity: 0; }
    }
    
    .thinking-placeholder {
        height: 2rem;
        border-radius: 0.25rem;
        overflow: hidden;
        position: relative;
        background: rgba(232, 121, 249, 0.05);
    }
    
    .shimmer {
        position: absolute;
        inset: 0;
        background: linear-gradient(
            90deg,
            transparent,
            rgba(232, 121, 249, 0.15),
            rgba(232, 121, 249, 0.25),
            rgba(232, 121, 249, 0.15),
            transparent
        );
        animation: shimmer 1.2s ease-in-out infinite;
    }
    
    @keyframes shimmer {
        0% { transform: translateX(-100%); }
        100% { transform: translateX(100%); }
    }
    
    /* Pulsing background on container while thinking */
    .thinking-container:has(.thinking-icon.active) .thinking-header {
        animation: header-pulse 2s ease-in-out infinite;
    }
    
    @keyframes header-pulse {
        0%, 100% {
            background: rgba(232, 121, 249, 0.08);
            border-color: rgba(232, 121, 249, 0.2);
        }
        50% {
            background: rgba(232, 121, 249, 0.12);
            border-color: rgba(232, 121, 249, 0.35);
        }
    }
</style>
