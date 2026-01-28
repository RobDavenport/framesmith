<script lang="ts">
  import "$lib/../app.css";
  import Header from "$lib/components/Header.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import CharacterOverview from "$lib/views/CharacterOverview.svelte";
  import FrameDataTable from "$lib/views/FrameDataTable.svelte";
  import MoveEditor from "$lib/views/MoveEditor.svelte";
  import { getCurrentCharacter } from "$lib/stores/character.svelte";

  let currentView = $state("overview");

  function handleViewChange(view: string) {
    currentView = view;
  }

  function handleEditMove(input: string) {
    currentView = "move-editor";
  }

  const currentCharacter = $derived(getCurrentCharacter());
</script>

<div class="app-container">
  <Header {currentView} onViewChange={handleViewChange} />
  <Sidebar />
  <main class="main-content">
    {#if !currentCharacter}
      <div class="placeholder">
        <p>Select a character from the sidebar to begin editing.</p>
      </div>
    {:else if currentView === "overview"}
      <CharacterOverview />
    {:else if currentView === "frame-data"}
      <FrameDataTable onEditMove={handleEditMove} />
    {:else if currentView === "move-editor"}
      <MoveEditor />
    {:else if currentView === "cancel-graph"}
      <p>Cancel Graph view coming soon...</p>
    {/if}
  </main>
</div>

<style>
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
