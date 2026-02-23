<script>
    let { title, description, iconColor, icon } = $props();
</script>

<button class="card">
    <div class="header">
        <div class="icon-wrapper" style="background-color: {iconColor.bg}; color: {iconColor.text}">
            {@render icon()}
        </div>
        <div class="arrow-circle">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 4.5 21 12m0 0-7.5 7.5M21 12H3" /></svg>
        </div>
    </div>
    <h3>{title}</h3>
    <p>{description}</p>
</button>

<style>
    .card {
        background: var(--bg-white);
        border: 1px solid var(--border-subtle);
        border-radius: var(--radius-xl);
        padding: 1.25rem;
        text-align: left;
        transition: all 0.2s ease;
        width: 100%;
        display: flex;
        flex-direction: column;
    }

    .card:hover {
        border-color: var(--border-hover);
        box-shadow: var(--shadow-sm);
        transform: translateY(-1px);
    }

    .header {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.75rem;
        width: 100%;
    }

    .icon-wrapper {
        width: 2.5rem;
        height: 2.5rem;
        border-radius: var(--radius-lg);
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    /* Deep selector to target the SVG passed in the snippet */
    .icon-wrapper :global(svg) {
        width: 1.25rem;
        height: 1.25rem;
    }

    .arrow-circle {
        width: 1.5rem;
        height: 1.5rem;
        border-radius: 50%;
        background: var(--bg-panel);
        color: var(--text-tertiary);
        display: flex;
        align-items: center;
        justify-content: center;
        transition: color 0.2s;
    }

    .card:hover .arrow-circle {
        background: #F3F4F6;
        color: var(--text-primary);
    }

    .arrow-circle svg {
        width: 0.75rem;
        height: 0.75rem;
    }

    h3 {
        font-weight: 700;
        margin: 0 0 0.25rem 0;
        font-size: 1rem;
    }

    p {
        font-size: 0.75rem;
        color: var(--text-secondary);
        line-height: 1.5;
        margin: 0;
    }
</style>