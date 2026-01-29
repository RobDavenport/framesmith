<script lang="ts">
  import {
    openProject,
    createProject,
    getProjectError,
    clearProjectError,
  } from "$lib/stores/project.svelte";
  import { showError } from "$lib/stores/toast.svelte";

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

<div class="empty-state">
  <div class="content">
    <h1 class="title">Framesmith</h1>
    <p class="subtitle">Fighting Game Character Authoring Tool</p>

    <div class="actions">
      <button class="primary-btn" onclick={handleOpenProject}>
        Open Existing Project
      </button>
      <button class="secondary-btn" onclick={handleCreateProject}>
        Create New Project
      </button>
    </div>

    <div class="info">
      <p>
        A <strong>project</strong> is a folder containing your game's character
        data:
      </p>
      <pre class="structure">my-game/
  framesmith.rules.json
  characters/
    character-1/
    character-2/</pre>
    </div>
  </div>
</div>

<style>
  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
  }

  .content {
    max-width: 400px;
  }

  .title {
    font-size: 32px;
    font-weight: 700;
    color: var(--accent);
    margin-bottom: 8px;
  }

  .subtitle {
    font-size: 14px;
    color: var(--text-secondary);
    margin-bottom: 32px;
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 32px;
  }

  .primary-btn {
    background: var(--accent);
    border-color: var(--accent);
    padding: 12px 24px;
    font-size: 14px;
    font-weight: 500;
  }

  .primary-btn:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .secondary-btn {
    background: transparent;
    border-color: var(--border);
  }

  .secondary-btn:hover {
    background: var(--bg-tertiary);
    border-color: var(--accent);
  }

  .info {
    text-align: left;
    padding: 16px;
    background: var(--bg-secondary);
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  .info p {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }

  .structure {
    font-family: "JetBrains Mono", monospace;
    font-size: 12px;
    color: var(--text-primary);
    background: var(--bg-primary);
    padding: 12px;
    border-radius: 4px;
    white-space: pre;
  }
</style>
