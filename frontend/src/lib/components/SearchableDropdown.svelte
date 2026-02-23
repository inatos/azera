<script lang="ts">
    import { tick } from 'svelte';
    
    interface Option {
        id: string;
        name: string;
        description?: string;
    }
    
    let { 
        options = [],
        value = $bindable(),
        placeholder = 'Select...',
        label = '',
        icon = '',
        onchange = undefined
    }: {
        options: Option[];
        value: string;
        placeholder?: string;
        label?: string;
        icon?: string;
        onchange?: (value: string) => void;
    } = $props();
    
    let isOpen = $state(false);
    let search = $state('');
    let inputEl: HTMLInputElement | undefined = $state(undefined);
    
    let filteredOptions = $derived(
        search.trim() 
            ? options.filter(o => 
                o.name.toLowerCase().includes(search.toLowerCase()) ||
                (o.description?.toLowerCase().includes(search.toLowerCase()))
              )
            : options
    );
    
    let selectedOption = $derived(options.find(o => o.id === value));
    
    function toggle() {
        isOpen = !isOpen;
        if (isOpen) {
            search = '';
            tick().then(() => inputEl?.focus());
        }
    }
    
    function select(option: Option) {
        value = option.id;
        isOpen = false;
        search = '';
        onchange?.(option.id);
    }
    
    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            isOpen = false;
        } else if (e.key === 'Enter' && filteredOptions.length > 0) {
            select(filteredOptions[0]);
        }
    }
</script>

<div class="dropdown-wrapper">
    {#if label}
        <span class="label">{label}</span>
    {/if}
    
    <button class="dropdown-trigger" onclick={toggle} type="button">
        {#if icon}
            <span class="icon">{icon}</span>
        {/if}
        <span class="selected-text">{selectedOption?.name || placeholder}</span>
        <svg class="chevron" class:open={isOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 9l6 6 6-6"/>
        </svg>
    </button>
    
    {#if isOpen}
        <div class="dropdown-menu">
            <div class="search-box">
                <input
                    bind:this={inputEl}
                    bind:value={search}
                    onkeydown={handleKeydown}
                    placeholder="Search..."
                    class="search-input"
                />
            </div>
            
            <div class="options-list">
                {#each filteredOptions as option (option.id)}
                    <button 
                        class="option" 
                        class:selected={option.id === value}
                        onclick={() => select(option)}
                        type="button"
                    >
                        <span class="option-name">{option.name}</span>
                        {#if option.description}
                            <span class="option-desc">{option.description}</span>
                        {/if}
                    </button>
                {:else}
                    <div class="no-results">No matches found</div>
                {/each}
            </div>
        </div>
    {/if}
</div>

<!-- Click outside to close -->
<svelte:window onclick={(e) => {
    if (isOpen && !(e.target as HTMLElement).closest('.dropdown-wrapper')) {
        isOpen = false;
    }
}} />

<style>
    .dropdown-wrapper {
        position: relative;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .label {
        font-size: 0.65rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: rgba(232, 121, 249, 0.6);
        font-weight: 600;
    }
    
    .dropdown-trigger {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.4rem 0.6rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.25);
        border-radius: 0.5rem;
        color: rgba(180, 210, 240, 0.9);
        font-size: 0.75rem;
        cursor: pointer;
        transition: all 0.2s;
        min-width: 120px;
    }
    
    .dropdown-trigger:hover {
        background: rgba(232, 121, 249, 0.18);
        border-color: rgba(232, 121, 249, 0.4);
    }
    
    .icon {
        font-size: 0.875rem;
    }
    
    .selected-text {
        flex: 1;
        text-align: left;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    
    .chevron {
        width: 0.875rem;
        height: 0.875rem;
        transition: transform 0.2s;
        opacity: 0.6;
    }
    
    .chevron.open {
        transform: rotate(180deg);
    }
    
    .dropdown-menu {
        position: absolute;
        bottom: 100%;
        right: 0;
        margin-bottom: 0.25rem;
        min-width: 200px;
        background: rgba(24, 26, 27, 0.98);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 0.5rem;
        box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.5);
        z-index: 1000;
        overflow: hidden;
    }
    
    .search-box {
        padding: 0.5rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
    }
    
    .search-input {
        width: 100%;
        padding: 0.4rem 0.6rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.375rem;
        color: rgba(232, 234, 236, 0.95);
        font-size: 0.75rem;
        outline: none;
    }
    
    .search-input::placeholder {
        color: rgba(139, 146, 149, 0.5);
    }
    
    .search-input:focus {
        border-color: rgba(232, 121, 249, 0.5);
        box-shadow: 0 0 8px rgba(232, 121, 249, 0.15);
    }
    
    .options-list {
        max-height: 200px;
        overflow-y: auto;
    }
    
    .option {
        width: 100%;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 0.125rem;
        padding: 0.5rem 0.75rem;
        background: transparent;
        border: none;
        color: rgba(200, 220, 240, 0.9);
        cursor: pointer;
        transition: all 0.15s;
        text-align: left;
    }
    
    .option:hover {
        background: rgba(232, 121, 249, 0.15);
    }
    
    .option.selected {
        background: rgba(232, 121, 249, 0.25);
    }
    
    .option-name {
        font-size: 0.8rem;
        font-weight: 500;
    }
    
    .option-desc {
        font-size: 0.65rem;
        color: rgba(139, 146, 149, 0.7);
    }
    
    .no-results {
        padding: 1rem;
        text-align: center;
        font-size: 0.75rem;
        color: rgba(139, 146, 149, 0.5);
    }
</style>
