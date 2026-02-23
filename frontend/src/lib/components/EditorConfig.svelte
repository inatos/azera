<script lang="ts">
    import { appState } from '$lib/store.svelte';
    
    let { isOpen = $bindable(false) } = $props();
    
    function close() {
        isOpen = false;
    }
    
    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            close();
        }
    }
</script>

{#if isOpen}
    <div class="backdrop" onclick={handleBackdropClick} onkeydown={(e) => e.key === 'Escape' && close()} role="button" tabindex="-1">
        <div class="config-panel">
            <header class="panel-header">
                <h2>⚙️ Editor Settings</h2>
                <button class="close-btn" onclick={close}>✕</button>
            </header>
            
            <div class="config-body">
                <div class="setting-group">
                    <h3>Display</h3>
                    
                    <label class="setting-row">
                        <span class="setting-label">Word Wrap</span>
                        <input 
                            type="checkbox" 
                            checked={appState.editorSettings.wordWrap}
                            onchange={(e) => appState.updateEditorSetting('wordWrap', e.currentTarget.checked)}
                        />
                    </label>
                    
                    <label class="setting-row">
                        <span class="setting-label">Line Numbers</span>
                        <input 
                            type="checkbox" 
                            checked={appState.editorSettings.lineNumbers}
                            onchange={(e) => appState.updateEditorSetting('lineNumbers', e.currentTarget.checked)}
                        />
                    </label>
                    
                    <label class="setting-row">
                        <span class="setting-label">Highlight Current Line</span>
                        <input 
                            type="checkbox" 
                            checked={appState.editorSettings.lineHighlight}
                            onchange={(e) => appState.updateEditorSetting('lineHighlight', e.currentTarget.checked)}
                        />
                    </label>
                </div>
                
                <div class="setting-group">
                    <h3>Editor</h3>
                    
                    <label class="setting-row">
                        <span class="setting-label">Font Size</span>
                        <div class="number-input">
                            <button onclick={() => appState.updateEditorSetting('fontSize', Math.max(10, appState.editorSettings.fontSize - 1))}>-</button>
                            <span>{appState.editorSettings.fontSize}px</span>
                            <button onclick={() => appState.updateEditorSetting('fontSize', Math.min(24, appState.editorSettings.fontSize + 1))}>+</button>
                        </div>
                    </label>
                    
                    <label class="setting-row">
                        <span class="setting-label">Tab Size</span>
                        <select 
                            value={appState.editorSettings.tabSize}
                            onchange={(e) => appState.updateEditorSetting('tabSize', parseInt(e.currentTarget.value))}
                        >
                            <option value={2}>2 spaces</option>
                            <option value={4}>4 spaces</option>
                            <option value={8}>8 spaces</option>
                        </select>
                    </label>
                    
                    <label class="setting-row">
                        <span class="setting-label">Quick Suggestions</span>
                        <input 
                            type="checkbox" 
                            checked={appState.editorSettings.quickSuggestions}
                            onchange={(e) => appState.updateEditorSetting('quickSuggestions', e.currentTarget.checked)}
                        />
                    </label>
                </div>
            </div>
            
            <footer class="panel-footer">
                <span class="hint">Settings are saved automatically</span>
            </footer>
        </div>
    </div>
{/if}

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.6);
        backdrop-filter: blur(4px);
        z-index: 9999;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .config-panel {
        background: linear-gradient(135deg, #1a1b1c, #2a2b2d);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 16px;
        width: 360px;
        max-height: 80vh;
        overflow: hidden;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px rgba(232, 121, 249, 0.1);
    }
    
    .panel-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 1rem 1.25rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.2);
    }
    
    .panel-header h2 {
        font-size: 1.1rem;
        font-weight: 600;
        color: #e8eaec;
        margin: 0;
    }
    
    .close-btn {
        background: transparent;
        border: none;
        color: rgba(232, 234, 236, 0.5);
        font-size: 1.2rem;
        cursor: pointer;
        padding: 0.25rem 0.5rem;
        border-radius: 4px;
        transition: all 0.15s;
    }
    
    .close-btn:hover {
        color: #e879f9;
        background: rgba(232, 121, 249, 0.1);
    }
    
    .config-body {
        padding: 1rem 1.25rem;
        max-height: 60vh;
        overflow-y: auto;
    }
    
    .setting-group {
        margin-bottom: 1.5rem;
    }
    
    .setting-group:last-child {
        margin-bottom: 0;
    }
    
    .setting-group h3 {
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: rgba(232, 121, 249, 0.8);
        margin: 0 0 0.75rem 0;
    }
    
    .setting-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.6rem 0;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        cursor: pointer;
    }
    
    .setting-row:last-child {
        border-bottom: none;
    }
    
    .setting-label {
        font-size: 0.9rem;
        color: rgba(232, 234, 236, 0.9);
    }
    
    .setting-row input[type="checkbox"] {
        width: 18px;
        height: 18px;
        accent-color: #e879f9;
        cursor: pointer;
    }
    
    .setting-row select {
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 6px;
        padding: 0.4rem 0.6rem;
        color: #e8eaec;
        font-size: 0.85rem;
        cursor: pointer;
    }
    
    .setting-row select:focus {
        outline: none;
        border-color: rgba(232, 121, 249, 0.5);
    }
    
    .number-input {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    
    .number-input button {
        width: 28px;
        height: 28px;
        background: rgba(232, 121, 249, 0.15);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 6px;
        color: #e879f9;
        font-size: 1rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.15s;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .number-input button:hover {
        background: rgba(232, 121, 249, 0.25);
        border-color: rgba(232, 121, 249, 0.5);
    }
    
    .number-input span {
        min-width: 45px;
        text-align: center;
        font-size: 0.9rem;
        color: #e8eaec;
        font-family: monospace;
    }
    
    .panel-footer {
        padding: 0.75rem 1.25rem;
        border-top: 1px solid rgba(232, 121, 249, 0.15);
        text-align: center;
    }
    
    .hint {
        font-size: 0.75rem;
        color: rgba(232, 234, 236, 0.4);
        font-style: italic;
    }
</style>
