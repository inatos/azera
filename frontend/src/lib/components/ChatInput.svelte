<script lang="ts">
    import MonacoInput from './MonacoInput.svelte';
    import SearchableDropdown from './SearchableDropdown.svelte';
    import BranchSelector from './BranchSelector.svelte';
    import ModelManager from './ModelManager.svelte';
    import EditorConfig from './EditorConfig.svelte';
    import { appState } from '$lib/store.svelte';

    let inputValue = $state('');
    let editorComponent: { clear: () => void } | undefined;
    let fileInput: HTMLInputElement;
    let isResizing = $state(false);
    let modelManagerOpen = $state(false);
    let editorConfigOpen = $state(false);
    let lineCount = $state(1);
    let manualHeight = $state<number | null>(null); // Manual override from resize
    
    // Auto-height calculation based on line count
    const TOOLBAR_HEIGHT = 60; // Toolbar + padding
    const LINE_HEIGHT = 20;
    const PADDING = 36; // Top + bottom padding
    const MIN_LINES = 3;
    const MIN_HEIGHT = 180;
    const MAX_HEIGHT_RATIO = 0.33; // 1/3 of viewport
    
    let wrapperHeight = $derived.by(() => {
        // If user manually resized, use that
        if (manualHeight !== null) return manualHeight;
        
        const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800;
        const maxHeight = viewportHeight * MAX_HEIGHT_RATIO;
        const desiredHeight = TOOLBAR_HEIGHT + PADDING + Math.max(lineCount, MIN_LINES) * LINE_HEIGHT;
        return Math.min(Math.max(desiredHeight, MIN_HEIGHT), maxHeight);
    });
    
    // Derived dropdown options
    let aiPersonaOptions = $derived(appState.aiPersonas.map(p => ({ id: p.id, name: p.name, description: p.description })));
    let userPersonaOptions = $derived(appState.userPersonas.map(p => ({ id: p.id, name: p.name, description: p.description })));
    let modelOptions = $derived(appState.availableModels.map(m => ({ id: m.id, name: m.name, description: m.provider })));
    
    // Dynamic persona icons - derive from current chat's last message or chat settings
    let currentAiIcon = $derived.by(() => {
        // Try to get from current chat's last AI message
        const messages = appState.messages;
        const lastAiMsg = [...messages].reverse().find(m => m.role === 'assistant');
        if (lastAiMsg?.aiPersona) {
            const persona = appState.aiPersonas.find(p => p.name === lastAiMsg.aiPersona);
            if (persona?.avatar) return persona.avatar;
        }
        // Fall back to chat's stored persona or current selection
        const chat = appState.currentChat;
        if (chat?.aiPersonaId) {
            const persona = appState.aiPersonas.find(p => p.id === chat.aiPersonaId);
            if (persona?.avatar) return persona.avatar;
        }
        // Final fallback
        const persona = appState.aiPersonas.find(p => p.id === appState.currentAiPersonaId);
        return persona?.avatar || '🤖';
    });
    
    let currentUserIcon = $derived.by(() => {
        // Try to get from current chat's last user message
        const messages = appState.messages;
        const lastUserMsg = [...messages].reverse().find(m => m.role === 'user');
        if (lastUserMsg?.userPersona) {
            const persona = appState.userPersonas.find(p => p.name === lastUserMsg.userPersona);
            if (persona?.avatar) return persona.avatar;
        }
        // Fall back to chat's stored persona or current selection
        const chat = appState.currentChat;
        if (chat?.userPersonaId) {
            const persona = appState.userPersonas.find(p => p.id === chat.userPersonaId);
            if (persona?.avatar) return persona.avatar;
        }
        // Final fallback
        const persona = appState.userPersonas.find(p => p.id === appState.currentUserPersonaId);
        return persona?.avatar || '👤';
    });
    
    // Handlers to sync persona changes to current chat
    function handleAiPersonaChange(personaId: string) {
        appState.setCurrentAiPersona(personaId);
    }
    
    function handleUserPersonaChange(personaId: string) {
        appState.setCurrentUserPersona(personaId);
    }
    
    function handleModelChange(modelId: string) {
        appState.setCurrentModel(modelId);
    }

    async function handleSend() {
        if (!inputValue.trim()) return;
        appState.currentInput = inputValue;
        inputValue = '';
        if (editorComponent) editorComponent.clear();
        await appState.sendMessage();
    }

    function triggerFile() {
        fileInput.click();
    }
    
    function handleFile(e: Event) {
        const file = (e.target as HTMLInputElement).files?.[0];
        if (file) {
            inputValue += `\n[Attached: ${file.name}]`;
        }
    }
    
    function handleLineCountChange(count: number) {
        lineCount = count;
        // Reset manual height when content changes significantly
        if (manualHeight !== null) {
            const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800;
            const maxHeight = viewportHeight * MAX_HEIGHT_RATIO;
            const autoHeight = TOOLBAR_HEIGHT + PADDING + Math.max(count, MIN_LINES) * LINE_HEIGHT;
            // Only reset if auto-height would be larger than manual
            if (autoHeight > manualHeight) {
                manualHeight = null;
            }
        }
    }
    
    function startResize(e: MouseEvent) {
        e.preventDefault();
        isResizing = true;
        const startY = e.clientY;
        const startHeight = wrapperHeight;
        
        function onMouseMove(moveEvent: MouseEvent) {
            const delta = startY - moveEvent.clientY;
            const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800;
            const maxHeight = viewportHeight * MAX_HEIGHT_RATIO;
            manualHeight = Math.max(MIN_HEIGHT, Math.min(maxHeight, startHeight + delta));
        }
        
        function onMouseUp() {
            isResizing = false;
            document.removeEventListener('mousemove', onMouseMove);
            document.removeEventListener('mouseup', onMouseUp);
        }
        
        document.addEventListener('mousemove', onMouseMove);
        document.addEventListener('mouseup', onMouseUp);
    }
</script>

<div class="wrapper" style="height: {wrapperHeight}px;" class:resizing={isResizing} class:sidebar-open={appState.isSidebarOpen}>
    <input type="file" bind:this={fileInput} onchange={handleFile} hidden />

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="resize-handle" onmousedown={startResize} title="Drag to resize">
        <div class="handle-line"></div>
    </div>

    <div class="input-container">
        <div class="toolbar">
            <div class="toolbar-left">
                <BranchSelector />
                <!-- TODO(future): Implement file/image analyzer -->
                <!-- <button class="pill-btn" onclick={triggerFile}>+ Attach File</button> -->
                <button class="config-btn" onclick={() => editorConfigOpen = true} title="Editor Settings">
                    ⚙️
                </button>
            </div>
            
            <div class="toolbar-right">
                <SearchableDropdown 
                    options={aiPersonaOptions}
                    bind:value={appState.currentAiPersonaId}
                    label="AI"
                    icon={currentAiIcon}
                    onchange={handleAiPersonaChange}
                />

                <SearchableDropdown 
                    options={userPersonaOptions}
                    bind:value={appState.currentUserPersonaId}
                    label="User"
                    icon={currentUserIcon}
                    onchange={handleUserPersonaChange}
                />
                <div class="model-wrapper">
                    <SearchableDropdown 
                        options={modelOptions}
                        bind:value={appState.selectedModel}
                        label="Model"
                        icon="⚡"
                        onchange={handleModelChange}
                    />
                    <button 
                        class="manage-models-btn" 
                        onclick={() => modelManagerOpen = true}
                        title="Manage models"
                    >
                        +
                    </button>
                </div>
            </div>
        </div>

        <div class="input-area">
            <div class="monaco-host">
                <MonacoInput 
                    bind:value={inputValue} 
                    onSend={handleSend} 
                    onLineCountChange={handleLineCountChange} 
                    bind:this={editorComponent}
                    editorSettings={appState.editorSettings}
                    settingsVersion={appState.editorSettingsVersion}
                    sendOnEnter={appState.sendOnEnter}
                />
            </div>

            <div class="actions">
                <span class="counter">{inputValue.length} chars</span>
                <button class="send-btn" onclick={handleSend} disabled={!inputValue.trim() || !appState.currentAiPersona || !appState.currentUserPersona} title={appState.sendOnEnter ? 'Send message (Enter)' : 'Send message (Ctrl+Enter)'}>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 12L3.269 3.126A59.768 59.768 0 0 1 21.485 12 59.77 59.77 0 0 1 3.27 20.876L5.999 12Zm0 0h7.5" /></svg>
                </button>
            </div>
        </div>
    </div>
</div>

<ModelManager bind:isOpen={modelManagerOpen} />
<EditorConfig bind:isOpen={editorConfigOpen} />

<style>
    .wrapper {
        position: fixed;
        bottom: 0;
        /* When sidebar collapsed: offset by half of 3.5rem */
        left: calc(50% + 1.75rem);
        transform: translateX(-50%);
        display: flex;
        flex-direction: column;
        width: calc(86% - 3.5rem);
        max-width: calc(100% - 5rem);
        padding: 0 0 1rem 0;
        z-index: 40;
        transition: height 0.15s ease-out;
    }
    
    /* When sidebar is open, offset to center in chat area */
    .wrapper.sidebar-open {
        left: calc(50% + 9rem); /* 9rem = half of 18rem sidebar */
        width: calc(86% - 18rem);
        max-width: calc(100% - 20rem);
    }
    
    .wrapper.resizing {
        user-select: none;
        transition: none;
    }
    
    .resize-handle {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 16px;
        cursor: ns-resize;
        margin-bottom: 0.25rem;
    }
    
    .handle-line {
        width: 40px;
        height: 4px;
        background: rgba(232, 121, 249, 0.3);
        border-radius: 2px;
        transition: all 0.2s;
    }
    
    .resize-handle:hover .handle-line {
        background: rgba(232, 121, 249, 0.6);
        width: 60px;
    }

    .input-container {
        background: linear-gradient(135deg, rgb(64, 45, 69), rgba(24, 26, 27, 0.69));
        border: 1px solid rgba(232, 121, 249, 0.251);
        border-radius: 1rem;
        padding: 1rem;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3), inset 0 1px 2px rgba(255, 255, 255, 0.02);
        backdrop-filter: blur(10px);
        display: flex;
        flex-direction: column;
        flex: 1;
        min-height: 0;
    }

    .toolbar {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
        gap: 0.5rem;
        padding: 0 0 0.75rem 0;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
        margin-bottom: 0.75rem;
        position: relative;
        z-index: 100;
    }
    
    .toolbar-left {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }
    
    .toolbar-right {
        display: flex;
        gap: 0.75rem;
        position: relative;
        z-index: 100;
    }
    
    .config-btn {
        font-size: 1rem;
        padding: 0.35rem 0.5rem;
        background: transparent;
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.2s;
        line-height: 1;
    }
    
    .config-btn:hover {
        background: rgba(232, 121, 249, 0.15);
        border-color: rgba(232, 121, 249, 0.4);
    }
    
    .model-wrapper {
        display: flex;
        align-items: flex-end;
        gap: 0.25rem;
    }
    
    .manage-models-btn {
        padding: 0.4rem 0.5rem;
        background: rgba(139, 92, 246, 0.15);
        border: 1px solid rgba(139, 92, 246, 0.3);
        border-radius: 6px;
        color: #a78bfa;
        font-size: 0.9rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.15s;
        margin-bottom: 0.1rem;
    }
    
    .manage-models-btn:hover {
        background: rgba(139, 92, 246, 0.25);
        border-color: rgba(139, 92, 246, 0.5);
        color: #c4b5fd;
    }

    .input-area {
        display: flex;
        align-items: stretch;
        gap: 1rem;
        flex: 1;
        min-height: 0;
    }

    .monaco-host {
        flex: 1;
        height: 100%;
        min-height: 0;
        border-radius: 0.75rem;
        border: 1px solid rgba(232, 121, 249, 0.2);
        background: rgba(20, 22, 23, 0.8);
        transition: border-color 0.2s, box-shadow 0.2s, background 0.2s;
        overflow: hidden;
    }
    
    .monaco-host:focus-within {
        border-color: rgba(232, 121, 249, 0.5);
        box-shadow: 0 0 15px rgba(232, 121, 249, 0.15), inset 0 0 8px rgba(232, 121, 249, 0.05);
        background: rgba(20, 22, 23, 0.95);
    }

    .actions {
        display: flex;
        flex-direction: column;
        justify-content: flex-end;
        gap: 0.5rem;
        padding-bottom: 0.25rem;
    }

    .counter {
        font-size: 0.75rem;
        color: rgba(139, 146, 149, 0.5);
        font-family: monospace;
        text-align: right;
        transition: color 0.2s;
    }

    .send-btn {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.25), rgba(192, 80, 210, 0.2));
        color: rgba(240, 180, 255, 0.9);
        border: 1px solid rgba(232, 121, 249, 0.35);
        padding: 0.625rem;
        border-radius: 0.625rem;
        transition: all 0.2s;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        min-width: 44px;
        min-height: 44px;
    }

    .send-btn:hover:not(:disabled) {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.4), rgba(192, 80, 210, 0.35));
        color: rgba(250, 210, 255, 0.95);
        border-color: rgba(232, 121, 249, 0.6);
        box-shadow: 0 0 12px rgba(232, 121, 249, 0.25);
        transform: translateY(-2px);
    }
    
    .send-btn:disabled { 
        opacity: 0.4; 
        cursor: not-allowed;
    }

    .send-btn:active:not(:disabled) {
        transform: translateY(0);
    }

    .send-btn svg { 
        width: 1.25rem; 
        height: 1.25rem;
        transition: transform 0.2s;
    }

    .send-btn:hover svg {
        transform: scale(1.1);
    }
</style>