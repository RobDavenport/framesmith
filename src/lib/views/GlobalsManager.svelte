<script lang="ts">
  import GlobalStateList from '$lib/components/GlobalStateList.svelte';
  import GlobalStateEditor from '$lib/components/GlobalStateEditor.svelte';
  import { loadGlobalStateList, resetGlobalsStore } from '$lib/stores/globals.svelte';
  import { onMount, onDestroy } from 'svelte';

  onMount(() => {
    loadGlobalStateList();
  });

  onDestroy(() => {
    resetGlobalsStore();
  });
</script>

<div class="globals-manager">
  <aside class="sidebar">
    <GlobalStateList />
  </aside>
  <main class="editor-panel">
    <GlobalStateEditor />
  </main>
</div>

<style>
  .globals-manager {
    display: flex;
    height: 100%;
    background: var(--bg-primary, #1a1a1a);
  }

  .sidebar {
    width: 280px;
    border-right: 1px solid var(--border-color, #333);
    overflow-y: auto;
  }

  .editor-panel {
    flex: 1;
    overflow: hidden;
  }
</style>
