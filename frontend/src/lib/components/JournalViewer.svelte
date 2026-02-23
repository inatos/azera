<script lang="ts">
    import { appState } from '$lib/store.svelte';
    import { marked } from 'marked';
    
    let renderedContent = $derived(
        appState.viewingJournalEntry 
            ? marked.parse(appState.viewingJournalEntry.content) 
            : ''
    );
    
    function formatDate(dateStr: string) {
        const date = new Date(dateStr);
        return date.toLocaleDateString('en-US', { 
            weekday: 'long', 
            year: 'numeric', 
            month: 'long', 
            day: 'numeric' 
        });
    }
    
    function getMoodEmoji(mood?: string) {
        const moods: Record<string, string> = {
            'contemplative': '🌙',
            'reflective': '💭',
            'joyful': '✨',
            'curious': '🔮',
            'serene': '🌸',
            'thoughtful': '📖',
        };
        return moods[mood || ''] || '📝';
    }
</script>

{#if appState.journalViewerOpen && appState.viewingJournalEntry}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div class="overlay" onclick={() => appState.closeJournalViewer()} role="presentation">
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
        <div class="viewer-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
            <header class="modal-header">
                <div class="header-info">
                    <span class="mood-emoji">{getMoodEmoji(appState.viewingJournalEntry.mood)}</span>
                    <div class="header-text">
                        <h2>{appState.viewingJournalEntry.title}</h2>
                        <span class="date">{formatDate(appState.viewingJournalEntry.date)}</span>
                    </div>
                </div>
                <button class="close-btn" onclick={() => appState.closeJournalViewer()} aria-label="Close">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 6L6 18M6 6l12 12"/>
                    </svg>
                </button>
            </header>
            
            <div class="modal-body">
                <article class="journal-content prose">
                    {@html renderedContent}
                </article>
            </div>
            
            <footer class="modal-footer">
                <span class="readonly-notice">📖 Journal entries are written by AI Persona and are read-only</span>
            </footer>
        </div>
    </div>
{/if}

<style>
    .overlay {
        position: fixed;
        inset: 0;
        background: rgba(10, 5, 25, 0.85);
        backdrop-filter: blur(8px);
        z-index: 100;
        display: flex;
        align-items: center;
        justify-content: center;
        animation: fadeIn 0.2s ease-out;
    }
    
    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }
    
    .viewer-modal {
        width: min(700px, 90vw);
        max-height: 85vh;
        background: linear-gradient(145deg, rgba(28, 30, 31, 0.98), rgba(24, 26, 27, 0.98));
        border: 1px solid rgba(232, 121, 249, 0.25);
        border-radius: 1rem;
        display: flex;
        flex-direction: column;
        box-shadow: 
            0 0 40px rgba(0, 0, 0, 0.4),
            inset 0 1px 0 rgba(255, 255, 255, 0.02);
        animation: slideUp 0.3s ease-out;
    }
    
    @keyframes slideUp {
        from { 
            opacity: 0; 
            transform: translateY(20px); 
        }
        to { 
            opacity: 1; 
            transform: translateY(0); 
        }
    }
    
    .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 1.25rem 1.5rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
        background: rgba(232, 121, 249, 0.04);
    }
    
    .header-info {
        display: flex;
        align-items: center;
        gap: 1rem;
    }
    
    .mood-emoji {
        font-size: 2rem;
    }
    
    .header-text {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .modal-header h2 {
        margin: 0;
        font-size: 1.25rem;
        font-weight: 600;
        color: rgba(232, 234, 236, 1);
    }
    
    .date {
        font-size: 0.85rem;
        color: rgba(139, 146, 149, 0.8);
    }
    
    .close-btn {
        width: 2rem;
        height: 2rem;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
        border: none;
        color: rgba(139, 146, 149, 0.7);
        cursor: pointer;
        border-radius: 0.5rem;
        transition: all 0.2s;
    }
    
    .close-btn:hover {
        background: rgba(232, 121, 249, 0.15);
        color: rgba(232, 234, 236, 1);
    }
    
    .close-btn svg {
        width: 1.25rem;
        height: 1.25rem;
    }
    
    .modal-body {
        flex: 1;
        overflow-y: auto;
        padding: 1.5rem;
    }
    
    .journal-content {
        color: rgba(220, 210, 240, 0.9);
        line-height: 1.7;
        font-size: 0.95rem;
    }
    
    .journal-content :global(h1) {
        font-size: 1.75rem;
        font-weight: 700;
        color: rgba(232, 234, 236, 1);
        margin: 0 0 0.5rem 0;
        background: linear-gradient(to right, rgba(232, 234, 236, 1), rgba(150, 200, 255, 0.9));
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        background-clip: text;
    }
    
    .journal-content :global(h2) {
        font-size: 1.25rem;
        font-weight: 600;
        color: rgba(200, 220, 240, 0.9);
        margin: 1.5rem 0 0.75rem 0;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
        padding-bottom: 0.5rem;
    }
    
    .journal-content :global(p) {
        margin: 1rem 0;
    }
    
    .journal-content :global(em) {
        color: rgba(232, 121, 249, 0.8);
        font-style: italic;
    }
    
    .journal-content :global(strong) {
        color: rgba(220, 210, 240, 1);
        font-weight: 600;
    }
    
    .journal-content :global(blockquote) {
        margin: 1.5rem 0;
        padding: 1rem 1.5rem;
        border-left: 3px solid rgba(232, 121, 249, 0.4);
        background: rgba(232, 121, 249, 0.08);
        border-radius: 0 0.5rem 0.5rem 0;
        font-style: italic;
        color: rgba(180, 210, 240, 0.9);
    }
    
    .journal-content :global(ul),
    .journal-content :global(ol) {
        margin: 1rem 0;
        padding-left: 1.5rem;
    }
    
    .journal-content :global(li) {
        margin: 0.5rem 0;
    }
    
    .journal-content :global(code) {
        background: rgba(0, 0, 0, 0.3);
        padding: 0.2em 0.4em;
        border-radius: 0.25rem;
        font-family: 'Consolas', monospace;
        font-size: 0.9em;
        color: rgba(232, 121, 249, 1);
    }
    
    .journal-content :global(pre) {
        background: rgba(20, 22, 23, 0.8);
        padding: 1rem;
        border-radius: 0.5rem;
        overflow-x: auto;
        margin: 1rem 0;
        border: 1px solid rgba(232, 121, 249, 0.15);
    }
    
    .journal-content :global(pre code) {
        background: transparent;
        padding: 0;
        color: rgba(180, 210, 240, 0.9);
    }
    
    .modal-footer {
        padding: 1rem 1.5rem;
        border-top: 1px solid rgba(232, 121, 249, 0.15);
        display: flex;
        justify-content: center;
    }
    
    .readonly-notice {
        font-size: 0.8rem;
        color: rgba(139, 146, 149, 0.6);
        font-style: italic;
    }
</style>
