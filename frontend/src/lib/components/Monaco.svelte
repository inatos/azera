<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import type * as Monaco from 'monaco-editor';
    import { setupMonacoEnv } from '$lib/monaco_workers';

    let { 
        value = $bindable(''), 
        language = 'markdown', 
        onSubmit 
    }: { 
        value: string, 
        language?: string, 
        onSubmit?: () => void
    } = $props();

    let editorContainer: HTMLElement;
    let editor: Monaco.editor.IStandaloneCodeEditor | undefined = $state(); // Use state for editor to track it
    let monaco: typeof Monaco;

    onMount(async () => {
        setupMonacoEnv();
        monaco = await import('monaco-editor');

        // Define Custom Theme to match Tailwind Zinc-900
        monaco.editor.defineTheme('zinc-dark', {
            base: 'vs-dark',
            inherit: true,
            rules: [
                { token: '', foreground: 'e4e4e7' },       // zinc-200 (default text)
                { token: 'comment', foreground: '71717a' }, // zinc-500
                { token: 'keyword', foreground: 'c084fc' }, // purple-400
                { token: 'string', foreground: '4ade80' },  // green-400
                { token: 'number', foreground: 'fbbf24' },  // amber-400
                { token: 'delimiter', foreground: 'a1a1aa' } // zinc-400
            ],
            colors: {
                'editor.background': '#18181b',          // zinc-900
                'editor.foreground': '#e4e4e7',          // zinc-200
                'editor.lineHighlightBackground': '#27272a', // zinc-800
                'editorCursor.foreground': '#3b82f6',    // blue-500
                'editor.selectionBackground': '#3b82f640', // blue-500 with opacity
                'scrollbarSlider.background': '#3f3f4680', // zinc-700
                'scrollbarSlider.hoverBackground': '#52525b', // zinc-600
            }
        });

        const newEditor = monaco.editor.create(editorContainer, {
            value,
            language,
            theme: 'zinc-dark', // Apply custom theme
            automaticLayout: true, // Critical for resizing
            minimap: { enabled: true },
            fontSize: 15,
            fontFamily: '"JetBrains Mono", "Fira Code", monospace',
            lineNumbers: 'off',
            glyphMargin: false,
            folding: false,
            overviewRulerBorder: false,
            hideCursorInOverviewRuler: true,
            scrollBeyondLastLine: false,
            wordWrap: 'on',
			wrappingStrategy: 'advanced',
            padding: { top: 16, bottom: 16 }, // Breathing room
            scrollbar: {
                vertical: 'auto', // Show only when needed
                horizontal: 'hidden'
            }
        });

        // Editor -> Parent Binding
        newEditor.onDidChangeModelContent(() => {
            // Prevent infinite loops by checking if value actually changed
            const content = newEditor.getValue();
            if (content !== value) {
                value = content;
            }
        });

        // Submit Handler
        newEditor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
            if (onSubmit) onSubmit();
        });

		let ignoreEvent = false;
		const width = 1920;
		const height = 1080;
		const container = editorContainer;
		const updateHeight = () => {
			if (!container) return;
			const contentHeight = Math.min(1000, newEditor.getContentHeight());
			container.style.width = `${width}px`;
			container.style.height = `${contentHeight}px`;
			try {
				ignoreEvent = true;
				newEditor.layout({ width, height: contentHeight });
			} finally {
				ignoreEvent = false;
			}
		};
		newEditor.onDidContentSizeChange(updateHeight);
		updateHeight();

		// Assign to state variable last so the effect picks it up
        editor = newEditor;
    });

    onDestroy(() => {
        if (editor) editor.dispose();
    });

	// This watches the 'value' prop. If the parent clears it (sets it to ""), we update the editor instance to match.
	$effect(() => {
        // Guard clause: wait until editor is initialized
        if (!editor) return;

        // Update editor only if the incoming prop is different from current content
        // This prevents the cursor from jumping when you type
        if (value !== editor.getValue()) {
            editor.setValue(value);
        }
    });
</script>

<div id="container" class="w-full h-full min-h-0 relative group rounded-b-xl overflow-hidden bg-zinc-900">
    <div bind:this={editorContainer} class="absolute inset-0"></div>
</div>