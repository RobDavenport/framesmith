<script lang="ts">
  import {
    openProject,
    createProject,
    getProjectName,
    getProjectError,
    clearProjectError,
    isProjectOpen,
  } from "$lib/stores/project.svelte";
  import { showError } from "$lib/stores/toast.svelte";
  import HelpPanel from "./HelpPanel.svelte";

  interface Props {
    currentView: string;
    onViewChange: (view: string) => void;
  }

  let { currentView, onViewChange }: Props = $props();

  let helpOpen = $state(false);

  const views = [
    { id: "overview", label: "Overview" },
    { id: "frame-data", label: "Frame Data" },
    { id: "state-editor", label: "State Editor" },
    { id: "cancel-graph", label: "Cancel Graph" },
    { id: "training", label: "Training" },
  ];

  const projectName = $derived(getProjectName());
  const projectOpen = $derived(isProjectOpen());

  async function handleOpenProject() {
    const success = await openProject();
    if (!success) {
      const error = getProjectError();
      if (error) {
        showError(error);
        clearProjectError();
      }
    }
  }

  async function handleCreateProject() {
    const success = await createProject();
    if (!success) {
      const error = getProjectError();
      if (error) {
        showError(error);
        clearProjectError();
      }
    }
  }
</script>

<header class="header">
  <div class="project-controls">
    <button class="project-btn" onclick={handleOpenProject}>Open...</button>
    <button class="project-btn" onclick={handleCreateProject}>New...</button>
  </div>
  {#if projectName}
    <span class="project-name">{projectName}</span>
  {/if}
  <h1 class="title">Framesmith</h1>
  {#if projectOpen}
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
  {/if}
  <button class="help-btn" onclick={() => (helpOpen = true)} title="Rules Documentation">
    ?
  </button>
</header>

<HelpPanel open={helpOpen} onClose={() => (helpOpen = false)} />

<style>
  .project-controls {
    display: flex;
    gap: 4px;
  }

  .project-btn {
    background: transparent;
    border: none;
    padding: 6px 10px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .project-btn:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .project-name {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border-radius: 4px;
  }

  .title {
    font-size: 18px;
    font-weight: 600;
    color: var(--accent);
    margin-left: auto;
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

  .help-btn {
    background: transparent;
    border: 1px solid var(--border);
    width: 28px;
    height: 28px;
    border-radius: 50%;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-left: auto;
    padding: 0;
  }

  .help-btn:hover {
    color: var(--text-primary);
    border-color: var(--accent);
    background: transparent;
  }
</style>
