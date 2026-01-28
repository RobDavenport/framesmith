<script lang="ts">
  import {
    getCharacterList,
    getCurrentCharacter,
    loadCharacterList,
    selectCharacter,
    isLoading,
  } from "$lib/stores/character.svelte";
  import { onMount } from "svelte";

  onMount(() => {
    loadCharacterList();
  });

  const characterList = $derived(getCharacterList());
  const currentCharacter = $derived(getCurrentCharacter());
  const loading = $derived(isLoading());
</script>

<aside class="sidebar">
  <div class="section">
    <h2 class="section-title">Characters</h2>
    {#if loading}
      <p class="loading">Loading...</p>
    {:else if characterList.length === 0}
      <p class="empty">No characters found</p>
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
              <span class="meta">{char.archetype} · {char.move_count} moves</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</aside>

<style>
  .section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .loading, .empty {
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
