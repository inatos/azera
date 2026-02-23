<script lang="ts">
    import { appState } from '$lib/store.svelte';
    import { slide } from 'svelte/transition';
    
    let isOpen = $state(false);
    let searchQuery = $state('');
    let editingBranchId = $state<string | null>(null);
    let editingBranchName = $state('');
    
    let branches = $derived(() => {
        if (!appState.currentChat) return [];
        return appState.currentChat.branches;
    });
    
    let filteredBranches = $derived(() => {
        const all = branches();
        if (!searchQuery.trim()) return all;
        const query = searchQuery.toLowerCase();
        return all.filter(b => b.name.toLowerCase().includes(query));
    });
    
    let currentBranch = $derived(() => {
        const branchList = branches();
        if (!appState.currentChat) return null;
        return branchList.find(b => b.id === appState.currentChat!.currentBranchId);
    });
    
    let hasMultipleBranches = $derived(() => branches().length > 1);
    
    function toggleDropdown() {
        isOpen = !isOpen;
        if (!isOpen) {
            searchQuery = '';
            editingBranchId = null;
        }
    }
    
    function selectBranch(branchId: string) {
        if (editingBranchId) return; // Don't switch while editing
        appState.switchBranch(branchId);
        isOpen = false;
        searchQuery = '';
    }
    
    function startEditing(e: Event, branch: { id: string; name: string }) {
        e.stopPropagation();
        editingBranchId = branch.id;
        editingBranchName = branch.name;
    }
    
    function saveEdit(e: Event) {
        e.stopPropagation();
        if (editingBranchId && editingBranchName.trim()) {
            appState.renameBranch(editingBranchId, editingBranchName.trim());
        }
        editingBranchId = null;
        editingBranchName = '';
    }
    
    function cancelEdit(e: Event) {
        e.stopPropagation();
        editingBranchId = null;
        editingBranchName = '';
    }
    
    function handleEditKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter') {
            saveEdit(e);
        } else if (e.key === 'Escape') {
            cancelEdit(e);
        }
    }
    
    function deleteBranch(e: Event, branch: { id: string; name: string; parentBranchId: string | null }) {
        e.stopPropagation();
        if (branch.parentBranchId === null) return; // Can't delete main branch
        
        if (confirm(`Delete path "${branch.name}"? This cannot be undone.`)) {
            appState.deleteBranch(branch.id);
        }
    }
</script>

{#if appState.currentChat}
    <div class="branch-selector">
        <button class="selector-btn" onclick={toggleDropdown} class:open={isOpen} class:has-branches={branches().length > 1}>
            <svg class="branch-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 3v12M6 3l3 3M6 3L3 6M18 9a3 3 0 100-6 3 3 0 000 6zM6 21a3 3 0 100-6 3 3 0 000 6zM18 21a3 3 0 100-6 3 3 0 000 6zM18 9v3a3 3 0 01-3 3H9" />
            </svg>
            <span class="branch-name">{currentBranch()?.name || 'Main'}</span>
            <svg class="chevron" class:rotate={isOpen} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M6 9l6 6 6-6"/>
            </svg>
        </button>
        
        {#if isOpen}
            <div class="dropdown" transition:slide={{ duration: 200 }}>
                <!-- Search input -->
                <div class="search-wrapper">
                    <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="11" cy="11" r="8"/>
                        <path d="M21 21l-4.35-4.35"/>
                    </svg>
                    <input 
                        type="text" 
                        class="search-input" 
                        placeholder="Search paths..." 
                        bind:value={searchQuery}
                        onclick={(e) => e.stopPropagation()}
                    />
                </div>
                
                <!-- Branch list -->
                <div class="branch-list">
                    {#each filteredBranches() as branch}
                        <div 
                            class="branch-option" 
                            class:active={branch.id === currentBranch()?.id}
                            onclick={() => selectBranch(branch.id)}
                            onkeydown={(e) => e.key === 'Enter' && selectBranch(branch.id)}
                            role="button"
                            tabindex="0"
                        >
                            {#if editingBranchId === branch.id}
                                <!-- Edit mode -->
                                <!-- svelte-ignore a11y_autofocus -->
                                <input
                                    type="text"
                                    class="edit-input"
                                    bind:value={editingBranchName}
                                    onkeydown={handleEditKeydown}
                                    onclick={(e) => e.stopPropagation()}
                                    autofocus
                                />
                                <button class="action-btn save" onclick={saveEdit} title="Save">
                                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M5 13l4 4L19 7"/>
                                    </svg>
                                </button>
                                <button class="action-btn cancel" onclick={cancelEdit} title="Cancel">
                                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            {:else}
                                <!-- Display mode - buttons first -->
                                <div class="action-btns">
                                    <button class="action-btn edit" onclick={(e) => startEditing(e, branch)} title="Rename">
                                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
                                            <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
                                        </svg>
                                    </button>
                                    {#if branch.parentBranchId !== null}
                                        <button class="action-btn delete" onclick={(e) => deleteBranch(e, branch)} title="Delete">
                                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                <path d="M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
                                            </svg>
                                        </button>
                                    {/if}
                                </div>
                                <span class="branch-indicator" class:main={!branch.parentBranchId}>
                                    {branch.parentBranchId ? '↳' : '●'}
                                </span>
                                <span class="option-name">{branch.name}</span>
                                <span class="message-count">{branch.messages.length}</span>
                            {/if}
                        </div>
                    {/each}
                    
                    {#if filteredBranches().length === 0}
                        <div class="no-results">No paths found</div>
                    {/if}
                </div>
            </div>
        {/if}
    </div>
{/if}

<style>
    .branch-selector {
        position: relative;
        display: inline-block;
    }
    
    .selector-btn {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.75rem;
        background: rgba(30, 35, 40, 0.6);
        border: 1px solid rgba(232, 121, 249, 0.25);
        border-radius: 0.5rem;
        color: rgba(180, 210, 240, 0.9);
        font-size: 0.8125rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .selector-btn:hover, .selector-btn.open {
        background: rgba(232, 121, 249, 0.18);
        border-color: rgba(232, 121, 249, 0.4);
    }
    
    .selector-btn.has-branches {
        border-color: rgba(232, 121, 249, 0.4);
        box-shadow: 0 0 8px rgba(232, 121, 249, 0.15);
    }
    
    .selector-btn.has-branches .branch-icon {
        color: rgba(232, 121, 249, 1);
    }
    
    .branch-icon {
        width: 14px;
        height: 14px;
        color: rgba(232, 121, 249, 0.8);
    }
    
    .branch-name {
        max-width: 120px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    
    .chevron {
        width: 14px;
        height: 14px;
        transition: transform 0.2s;
        opacity: 0.7;
    }
    
    .chevron.rotate {
        transform: rotate(180deg);
    }
    
    .dropdown {
        position: absolute;
        top: calc(100% + 4px);
        left: 0;
        min-width: 340px;
        background: rgba(24, 26, 27, 0.98);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 0.5rem;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
        backdrop-filter: blur(10px);
        overflow: hidden;
        z-index: 50;
    }
    
    .search-wrapper {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.625rem 0.75rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
    }
    
    .search-icon {
        width: 14px;
        height: 14px;
        color: rgba(232, 121, 249, 0.5);
        flex-shrink: 0;
    }
    
    .search-input {
        flex: 1;
        background: none;
        border: none;
        color: rgba(232, 234, 236, 0.9);
        font-size: 0.8125rem;
        outline: none;
    }
    
    .search-input::placeholder {
        color: rgba(139, 146, 149, 0.5);
    }
    
    .branch-list {
        max-height: 240px;
        overflow-y: auto;
    }
    
    .branch-option {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.75rem;
        background: none;
        border: none;
        color: rgba(200, 220, 240, 0.85);
        font-size: 0.8125rem;
        cursor: pointer;
        text-align: left;
        transition: all 0.15s;
    }
    
    .branch-option:hover {
        background: rgba(232, 121, 249, 0.12);
    }
    
    .branch-option.active {
        background: rgba(232, 121, 249, 0.2);
        color: rgba(232, 234, 236, 0.95);
    }
    
    .branch-indicator {
        font-size: 0.75rem;
        color: rgba(232, 121, 249, 0.6);
        width: 16px;
        flex-shrink: 0;
    }
    
    .branch-indicator.main {
        color: rgba(232, 121, 249, 0.9);
    }
    
    .option-name {
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    
    .message-count {
        font-size: 0.6875rem;
        color: rgba(139, 146, 149, 0.6);
        padding: 0.125rem 0.375rem;
        background: rgba(232, 121, 249, 0.1);
        border-radius: 0.25rem;
        flex-shrink: 0;
    }
    
    .action-btns {
        display: flex;
        gap: 0.25rem;
        flex-shrink: 0;
        opacity: 0;
        transition: opacity 0.15s;
    }
    
    .branch-option:hover .action-btns {
        opacity: 1;
    }
    
    .action-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 22px;
        height: 22px;
        padding: 0;
        background: rgba(232, 121, 249, 0.1);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.25rem;
        color: rgba(232, 121, 249, 0.7);
        cursor: pointer;
        transition: all 0.15s;
        flex-shrink: 0;
    }
    
    .action-btn:hover {
        background: rgba(232, 121, 249, 0.2);
        border-color: rgba(232, 121, 249, 0.4);
        color: rgba(200, 230, 255, 0.9);
    }
    
    .action-btn svg {
        width: 12px;
        height: 12px;
    }
    
    .action-btn.delete:hover {
        background: rgba(239, 68, 68, 0.2);
        border-color: rgba(239, 68, 68, 0.4);
        color: rgba(252, 165, 165, 0.9);
    }
    
    .action-btn.save:hover {
        background: rgba(34, 197, 94, 0.2);
        border-color: rgba(34, 197, 94, 0.4);
        color: rgba(134, 239, 172, 0.9);
    }
    
    .action-btn.cancel:hover {
        background: rgba(239, 68, 68, 0.2);
        border-color: rgba(239, 68, 68, 0.4);
        color: rgba(252, 165, 165, 0.9);
    }
    
    .edit-input {
        flex: 1;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.25);
        border-radius: 0.25rem;
        color: rgba(232, 234, 236, 0.95);
        font-size: 0.8125rem;
        padding: 0.25rem 0.5rem;
        outline: none;
    }
    
    .edit-input:focus {
        border-color: rgba(232, 121, 249, 0.5);
        box-shadow: 0 0 0 2px rgba(232, 121, 249, 0.1);
    }
    
    .no-results {
        padding: 1rem;
        text-align: center;
        color: rgba(139, 146, 149, 0.5);
        font-size: 0.8125rem;
    }
</style>
