<script lang="ts">
  interface Props {
    currentView: string;
    onViewChange: (view: string) => void;
  }

  let { currentView, onViewChange }: Props = $props();

  const views = [
    { id: "overview", label: "Overview" },
    { id: "frame-data", label: "Frame Data" },
    { id: "move-editor", label: "Move Editor" },
    { id: "cancel-graph", label: "Cancel Graph" },
  ];
</script>

<header class="header">
  <h1 class="title">Framesmith</h1>
  <nav class="nav">
    {#each views as view}
      <button
        class="nav-btn"
        class:active={currentView === view.id}
        onclick={() => onViewChange(view.id)}
      >
        {view.label}
      </button>
    {/each}
  </nav>
</header>

<style>
  .title {
    font-size: 18px;
    font-weight: 600;
    color: var(--accent);
  }

  .nav {
    display: flex;
    gap: 4px;
    margin-left: 24px;
  }

  .nav-btn {
    background: transparent;
    border: none;
    padding: 8px 12px;
    color: var(--text-secondary);
  }

  .nav-btn:hover {
    color: var(--text-primary);
    background: transparent;
  }

  .nav-btn.active {
    color: var(--accent);
    border-bottom: 2px solid var(--accent);
    border-radius: 0;
  }
</style>
