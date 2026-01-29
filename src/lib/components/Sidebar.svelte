<script lang="ts">
  import {
    getCharacterList,
    getCurrentCharacter,
    isLoading,
  } from "$lib/stores/character.svelte";
  import { isProjectOpen } from "$lib/stores/project.svelte";
  import { selectCharacter } from "$lib/stores/character.svelte";
  import CreateCharacterModal from "./CreateCharacterModal.svelte";

  const characterList = $derived(getCharacterList());
  const currentCharacter = $derived(getCurrentCharacter());
  const loading = $derived(isLoading());
  const projectOpen = $derived(isProjectOpen());

  let showMenu = $state(false);
  let showModal = $state(false);
  let modalMode = $state<"new" | "clone">("new");

  function toggleMenu() {
    showMenu = !showMenu;
  }

  function handleNewCharacter() {
    modalMode = "new";
    showModal = true;
    showMenu = false;
  }

  function handleCloneCharacter() {
    modalMode = "clone";
    showModal = true;
    showMenu = false;
  }

  function closeModal() {
    showModal = false;
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest(".add-menu-container")) {
      showMenu = false;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<aside class="sidebar">
  {#if projectOpen}
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">Characters</h2>
        <div class="add-menu-container">
          <button class="add-btn" onclick={toggleMenu}>+</button>
          {#if showMenu}
            <div class="add-menu">
              <button class="menu-item" onclick={handleNewCharacter}>
                New Empty Character...
              </button>
              <button
                class="menu-item"
                onclick={handleCloneCharacter}
                disabled={characterList.length === 0}
              >
                Clone From...
              </button>
            </div>
          {/if}
        </div>
      </div>
      {#if loading}
        <p class="loading">Loading...</p>
      {:else if characterList.length === 0}
        <p class="empty">No characters yet</p>
      {:else}
        <ul class="character-list">
          {#each characterList as char}
            <li>
              <button
                class="character-btn"
                class:active={currentCharacter?.character.id === char.id}
                onclick={() => selectCharacter(char.id)}
              >
                <span class="name">{char.name}</span>
                <span class="meta"
                  >{char.archetype || "No archetype"} · {char.move_count} moves</span
                >
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else}
    <div class="section">
      <p class="empty">Open a project to see characters</p>
    </div>
  {/if}
</aside>

<CreateCharacterModal open={showModal} mode={modalMode} onClose={closeModal} />

<style>
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .add-menu-container {
    position: relative;
  }

  .add-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    font-size: 16px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--border);
  }

  .add-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
  }

  .add-menu {
    position: absolute;
    top: 100%;
    right: 0;
    margin-top: 4px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    min-width: 180px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 10;
  }

  .menu-item {
    width: 100%;
    text-align: left;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: 0;
    font-size: 13px;
  }

  .menu-item:hover:not(:disabled) {
    background: var(--bg-tertiary);
  }

  .menu-item:disabled {
    color: var(--text-secondary);
    opacity: 0.5;
  }

  .loading,
  .empty {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .character-list {
    list-style: none;
  }

  .character-btn {
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    padding: 8px 10px;
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .character-btn:hover {
    background: var(--bg-tertiary);
  }

  .character-btn.active {
    background: var(--accent);
  }

  .name {
    font-weight: 500;
  }

  .meta {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .character-btn.active .meta {
    color: var(--text-primary);
  }
</style>
