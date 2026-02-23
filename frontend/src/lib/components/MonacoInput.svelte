<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import type * as Monaco from 'monaco-editor';
    import type { EditorSettings } from '$lib/store.svelte';
    import 'monaco-editor/min/vs/editor/editor.main.css';

    let { 
        value = $bindable(), 
        onSend, 
        onLineCountChange,
        editorSettings,
        settingsVersion = 0,
        sendOnEnter = false
    }: {
        value?: string;
        onSend?: () => void;
        onLineCountChange?: (count: number) => void;
        editorSettings: EditorSettings;
        settingsVersion?: number;
        sendOnEnter?: boolean;
    } = $props();
    
    let editorContainer: HTMLDivElement;
    let editorInstance: Monaco.editor.IStandaloneCodeEditor | null = null;
    let monaco: typeof Monaco;
    
    // Start with empty content (no initial newlines)
    const INITIAL_CONTENT = '';

    onMount(async () => {
        // Import Monaco and configure it
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

        // Create Editor with settings from props
        editorInstance = monaco.editor.create(editorContainer, {
            value: value || INITIAL_CONTENT,
            language: 'markdown',
            theme: 'vs-dark',
            minimap: { enabled: false },
            lineNumbers: editorSettings.lineNumbers ? 'on' : 'off',
            wordWrap: editorSettings.wordWrap ? 'on' : 'off',
            automaticLayout: true,
            fontFamily: 'Consolas, \"Courier New\", monospace',
            fontSize: editorSettings.fontSize,
            lineHeight: 20,
            scrollBeyondLastLine: false,
            padding: { top: 12, bottom: 12 },
            renderLineHighlight: editorSettings.lineHighlight ? 'line' : 'none',
            scrollbar: {
                vertical: 'auto',
                horizontal: 'hidden',
                verticalScrollbarSize: 8,
            },
            overviewRulerBorder: false,
            hideCursorInOverviewRuler: true,
            glyphMargin: false,
            folding: false,
            lineDecorationsWidth: 8,
            lineNumbersMinChars: 3,
            quickSuggestions: editorSettings.quickSuggestions,
            tabSize: editorSettings.tabSize,
        });

        // Bind changes back to 'value' prop and report line count
        editorInstance.onDidChangeModelContent(() => {
            value = editorInstance!.getValue();
            const lineCount = editorInstance!.getModel()?.getLineCount() || 1;
            if (onLineCountChange) onLineCountChange(lineCount);
        });

        // Keybinding for Cmd/Ctrl + Enter to send message (always active)
        editorInstance.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
            if (onSend) onSend();
        });
        
        // Keybinding for Enter to send message (when sendOnEnter is enabled)
        editorInstance.addCommand(monaco.KeyCode.Enter, () => {
            if (sendOnEnter && onSend) {
                onSend();
            } else {
                // Default behavior: insert newline
                editorInstance!.trigger('keyboard', 'type', { text: '\n' });
            }
        });
        
        // Set initial value for binding and report initial line count
        value = INITIAL_CONTENT;
        if (onLineCountChange) onLineCountChange(1);
    });

    onDestroy(() => {
        if (editorInstance) editorInstance.dispose();
        // Clean up event listener
        if (typeof window !== 'undefined') {
            window.removeEventListener('azera:editorSettingsChanged', handleSettingsChanged as EventListener);
        }
    });
    
    // Handle settings change event
    function handleSettingsChanged(e: CustomEvent<typeof editorSettings>) {
        if (!editorInstance) return;
        const settings = e.detail;
        console.log('Settings changed event received:', settings);
        
        editorInstance.updateOptions({
            lineNumbers: settings.lineNumbers ? 'on' : 'off',
            wordWrap: settings.wordWrap ? 'on' : 'off',
            fontSize: settings.fontSize,
            renderLineHighlight: settings.lineHighlight ? 'line' : 'none',
            quickSuggestions: settings.quickSuggestions,
            tabSize: settings.tabSize,
        });
    }
    
    // Set up event listener on mount
    $effect(() => {
        if (typeof window !== 'undefined') {
            window.addEventListener('azera:editorSettingsChanged', handleSettingsChanged as EventListener);
        }
    });

    // Helper to clear externally - reset to empty
    export function clear() {
        if (editorInstance) {
            editorInstance.setValue(INITIAL_CONTENT);
            editorInstance.setPosition({ lineNumber: 1, column: 1 });
            if (onLineCountChange) onLineCountChange(1);
        }
    }
</script>

<div class="monaco-wrapper" bind:this={editorContainer}></div>

<style>
    .monaco-wrapper {
        width: 100%;
        height: 100%;
        border-radius: var(--radius-lg);
        overflow: hidden;
    }
</style>