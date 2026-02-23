<script lang="ts">
    import { appState } from '$lib/store.svelte';
    import { onMount, onDestroy } from 'svelte';
    import type * as Monaco from 'monaco-editor';
    import 'monaco-editor/min/vs/editor/editor.main.css';
    import SearchableDropdown from './SearchableDropdown.svelte';
    
    let editorContainer: HTMLDivElement | undefined = $state(undefined);
    let editorInstance: Monaco.editor.IStandaloneCodeEditor | null = null;
    let monaco: typeof Monaco;
    
    // User persona options
    let userPersonaOptions = $derived(
        appState.userPersonas.map(p => ({ id: p.id, name: p.name }))
    );
    
    // Bind persona with default value
    let selectedPersona = $state('');
    
    $effect(() => {
        if (appState.messageEditorOpen) {
            selectedPersona = appState.editingMessagePersona || appState.currentUserPersonaId;
        }
    });
    
    $effect(() => {
        appState.editingMessagePersona = selectedPersona || null;
    });

    let monacoLoaded = $state(false);
    
    onMount(async () => {
        // Import Monaco
        const mod = await import('monaco-editor');
        monaco = mod;
        
        // Configure Monaco to suppress worker warnings
        if (typeof window !== 'undefined') {
            const workerCode = `
                self.onmessage = function(e) {
                    self.postMessage({ id: e.data.id });
                };
            `;
            const blob = new Blob([workerCode], { type: 'application/javascript' });
            const workerUrl = URL.createObjectURL(blob);
            
            (window as any).MonacoEnvironment = {
                getWorkerUrl: () => workerUrl
            };
        }
        
        monacoLoaded = true;
    });
    
    // Track if editor has been created for this session
    let editorCreated = $state(false);
    
    // Branch name for the new fork
    let branchName = $state('');
    
    // Reset state when panel closes/opens
    $effect(() => {
        if (!appState.messageEditorOpen) {
            editorCreated = false;
            // Dispose editor when modal closes
            if (editorInstance) {
                editorInstance.dispose();
                editorInstance = null;
            }
        } else {
            // Set default branch name when opening
            const branchCount = appState.currentChat?.branches?.length || 0;
            branchName = `Path ${branchCount + 1}`;
        }
    });
    
    // Create editor when panel opens and container is available
    $effect(() => {
        if (!appState.messageEditorOpen || !editorContainer || !monacoLoaded || !monaco || editorCreated) return;
        
        // Mark as created to prevent re-creation
        editorCreated = true;
        
        // Dispose old instance if exists
        if (editorInstance) {
            editorInstance.dispose();
        }
        
        // Get initial content before creating editor
        const initialContent = appState.editingMessageContent;

        // Create Editor with consistent styling
        editorInstance = monaco.editor.create(editorContainer, {
            value: initialContent,
            language: 'markdown',
            theme: 'vs-dark',
            minimap: { enabled: false },
            lineNumbers: 'on',
            wordWrap: 'on',
            automaticLayout: true,
            fontFamily: 'Consolas, "Courier New", monospace',
            fontSize: 13,
            lineHeight: 20,
            scrollBeyondLastLine: false,
            padding: { top: 12, bottom: 12 },
            renderLineHighlight: 'line',
            scrollbar: {
                vertical: 'auto',
                horizontal: 'hidden',
                verticalScrollbarSize: 8,
            },
            overviewRulerBorder: false,
            hideCursorInOverviewRuler: true,
        });

        // Bind changes back to state
        editorInstance.onDidChangeModelContent(() => {
            appState.editingMessageContent = editorInstance!.getValue();
        });
    });

    onDestroy(() => {
        if (editorInstance) editorInstance.dispose();
    });
    
    function handleCommit() {
        appState.commitMessageEdit(branchName.trim() || undefined);
    }
    
    function handleCancel() {
        appState.closeMessageEditor();
    }
</script>

{#if appState.messageEditorOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="overlay" role="presentation" onclick={handleCancel}>
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="editor-panel" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
            <div class="header">
                <h2>✏️ Edit Message</h2>
                <p class="subtitle">Changes will create a new branch from this point</p>
                <button class="close-btn" onclick={handleCancel} title="Close editor">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>
            
            <div class="options-row">
                <div class="form-group">
                    <label for="branch-name">Branch Name</label>
                    <input 
                        id="branch-name"
                        type="text" 
                        bind:value={branchName}
                        placeholder="Path name..."
                    />
                </div>
                <div class="form-group persona-group">
                    <span class="label-text">User Persona</span>
                    <SearchableDropdown 
                        options={userPersonaOptions}
                        bind:value={selectedPersona}
                        label=""
                        icon="👤"
                    />
                </div>
            </div>
            
            <div class="editor-container" bind:this={editorContainer}></div>
            
            <div class="actions">
                <button class="cancel-btn" onclick={handleCancel}>Cancel</button>
                <button class="commit-btn" onclick={handleCommit}>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                    Commit & Fork
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    .overlay {
        position: fixed;
        inset: 0;
        background: rgba(10, 5, 20, 0.85);
        backdrop-filter: blur(8px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
        animation: fadeIn 0.2s ease-out;
    }
    
    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }
    
    .editor-panel {
        width: 90%;
        max-width: 800px;
        height: 70vh;
        max-height: 600px;
        background: linear-gradient(135deg, rgba(30, 35, 40, 0.98), rgba(24, 26, 27, 0.98));
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 1rem;
        box-shadow: 0 25px 80px rgba(0, 0, 0, 0.5), inset 0 1px 2px rgba(255, 255, 255, 0.02);
        display: flex;
        flex-direction: column;
        overflow: hidden;
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
    
    .header {
        padding: 1.25rem 1.5rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 0.5rem;
        position: relative;
    }
    
    .header h2 {
        margin: 0;
        font-size: 1.25rem;
        color: rgba(232, 234, 236, 0.95);
        font-weight: 600;
    }
    
    .subtitle {
        width: 100%;
        margin: 0.25rem 0 0 0;
        font-size: 0.875rem;
        color: rgba(139, 146, 149, 0.7);
    }
    
    .close-btn {
        position: absolute;
        top: 1rem;
        right: 1rem;
        background: none;
        border: none;
        padding: 0.5rem;
        cursor: pointer;
        color: rgba(139, 146, 149, 0.6);
        transition: all 0.2s;
    }
    
    .close-btn:hover {
        color: rgba(232, 234, 236, 0.9);
        transform: scale(1.1);
    }
    
    .close-btn svg {
        width: 20px;
        height: 20px;
    }
    
    .options-row {
        display: flex;
        gap: 1rem;
        padding: 1rem 1.5rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.1);
    }
    
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
    }
    
    .form-group:first-child {
        flex: 1;
    }
    
    .persona-group {
        min-width: 180px;
    }
    
    .form-group label,
    .form-group .label-text {
        font-size: 0.6875rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: rgba(232, 121, 249, 0.7);
    }
    
    .form-group input {
        padding: 0.5rem 0.75rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(232, 234, 236, 0.95);
        font-size: 0.875rem;
        outline: none;
        transition: all 0.2s;
    }
    
    .form-group input:focus {
        border-color: rgba(232, 121, 249, 0.5);
        box-shadow: 0 0 12px rgba(232, 121, 249, 0.15);
    }
    
    .form-group input::placeholder {
        color: rgba(139, 146, 149, 0.5);
    }
    
    .editor-container {
        flex: 1;
        min-height: 0;
        margin: 0;
        border-radius: 0;
        overflow: hidden;
    }
    
    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.75rem;
        padding: 1rem 1.5rem;
        border-top: 1px solid rgba(232, 121, 249, 0.15);
        background: rgba(20, 22, 23, 0.5);
    }
    
    .cancel-btn {
        padding: 0.625rem 1.25rem;
        background: rgba(50, 55, 60, 0.5);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(180, 210, 240, 0.9);
        font-size: 0.875rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .cancel-btn:hover {
        background: rgba(60, 65, 70, 0.6);
        border-color: rgba(232, 121, 249, 0.35);
    }
    
    .commit-btn {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.625rem 1.25rem;
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.5), rgba(192, 80, 210, 0.5));
        border: 1px solid rgba(232, 121, 249, 0.4);
        border-radius: 0.5rem;
        color: rgba(232, 234, 236, 0.95);
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .commit-btn:hover {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.7), rgba(192, 80, 210, 0.7));
        box-shadow: 0 4px 20px rgba(232, 121, 249, 0.3);
        transform: translateY(-1px);
    }
    
    .commit-btn svg {
        width: 16px;
        height: 16px;
    }
</style>
