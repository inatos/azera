<script lang="ts">
    let { value = $bindable('#6b7280'), size = 'small' } = $props();
    
    let isOpen = $state(false);
    let pickerRef = $state<HTMLDivElement | undefined>(undefined);
    
    // Preset colors that match the app theme
    const presetColors = [
        // Magentas/Purples (accent colors)
        '#e879f9', '#d946ef', '#c026d3', '#a855f7', '#8b5cf6',
        // Blues
        '#3b82f6', '#0ea5e9', '#06b6d4', '#14b8a6', '#10b981',
        // Greens/Yellows
        '#22c55e', '#84cc16', '#eab308', '#f59e0b', '#f97316',
        // Reds/Pinks
        '#ef4444', '#f43f5e', '#ec4899', '#db2777', '#be185d',
        // Neutrals
        '#6b7280', '#9ca3af', '#d1d5db', '#374151', '#1f2937',
    ];
    
    function selectColor(color: string) {
        value = color;
        isOpen = false;
    }
    
    function handleClickOutside(e: MouseEvent) {
        if (pickerRef && !pickerRef.contains(e.target as Node)) {
            isOpen = false;
        }
    }
    
    $effect(() => {
        if (isOpen) {
            document.addEventListener('click', handleClickOutside);
            return () => document.removeEventListener('click', handleClickOutside);
        }
    });
</script>

<div class="color-picker-wrapper" class:small={size === 'small'} bind:this={pickerRef}>
    <button 
        type="button"
        class="color-swatch"
        style="background-color: {value};"
        onclick={(e) => { e.stopPropagation(); isOpen = !isOpen; }}
        title="Pick a color"
    ></button>
</div>

{#if isOpen}
    <!-- Portal: render outside of wrapper to avoid stacking context issues -->
    <div class="color-dropdown-portal" style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; z-index: 99999;">
        <div class="color-dropdown" style="position: absolute; top: {pickerRef?.getBoundingClientRect().top - 240}px; left: {pickerRef?.getBoundingClientRect().left - 85}px; pointer-events: auto;">
            <div class="preset-grid">
                {#each presetColors as color}
                    <button
                        type="button"
                        class="preset-color"
                        class:selected={value === color}
                        style="background-color: {color};"
                        onclick={() => selectColor(color)}
                        title={color}
                    ></button>
                {/each}
            </div>
            <div class="custom-input">
                <input 
                    type="text" 
                    bind:value={value}
                    placeholder="#hex"
                    maxlength="7"
                />
                <div class="preview" style="background-color: {value};"></div>
            </div>
        </div>
    </div>
{/if}

<style>
    .color-picker-wrapper {
        position: relative;
        display: inline-block;
    }
    
    .color-swatch {
        width: 32px;
        height: 32px;
        border: 2px solid rgba(255, 255, 255, 0.1);
        border-radius: 6px;
        cursor: pointer;
        transition: all 0.15s;
        box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.2);
    }
    
    .color-picker-wrapper.small .color-swatch {
        width: 28px;
        height: 28px;
        border-radius: 4px;
    }
    
    .color-swatch:hover {
        border-color: rgba(232, 121, 249, 0.5);
        transform: scale(1.05);
    }
    
    .color-dropdown {
        background: linear-gradient(135deg, #1a1b1c, #2a2b2d);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 12px;
        padding: 12px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5), 0 0 20px rgba(232, 121, 249, 0.1);
        min-width: 200px;
    }
    
    .preset-grid {
        display: grid;
        grid-template-columns: repeat(5, 1fr);
        gap: 6px;
        margin-bottom: 10px;
    }
    
    .preset-color {
        width: 28px;
        height: 28px;
        border: 2px solid transparent;
        border-radius: 4px;
        cursor: pointer;
        transition: all 0.15s;
        box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.2);
    }
    
    .preset-color:hover {
        transform: scale(1.15);
        border-color: rgba(255, 255, 255, 0.3);
    }
    
    .preset-color.selected {
        border-color: white;
        box-shadow: 0 0 8px rgba(255, 255, 255, 0.3), inset 0 0 0 1px rgba(0, 0, 0, 0.2);
    }
    
    .custom-input {
        display: flex;
        gap: 8px;
        align-items: center;
        padding-top: 10px;
        border-top: 1px solid rgba(255, 255, 255, 0.1);
    }
    
    .custom-input input {
        flex: 1;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 6px;
        padding: 6px 10px;
        color: #e8eaec;
        font-family: monospace;
        font-size: 0.85rem;
    }
    
    .custom-input input:focus {
        outline: none;
        border-color: rgba(232, 121, 249, 0.5);
    }
    
    .custom-input .preview {
        width: 28px;
        height: 28px;
        border-radius: 4px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        flex-shrink: 0;
    }
</style>
