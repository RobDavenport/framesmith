<script lang="ts">
  import { getCurrentCharacter, getRulesRegistry, selectMove } from "$lib/stores/character.svelte";
  import CreateMoveModal from "$lib/components/CreateMoveModal.svelte";
  import type { State } from "$lib/types";
  import { getTotal, getAdvantageHit, getAdvantageBlock, formatAdvantage, sortMoves, filterMoves, buildFilterOptions, type SortableColumn } from "./frameDataUtils";

  interface Props {
    onEditMove: (input: string) => void;
  }

  let { onEditMove }: Props = $props();
  let showCreateModal = $state(false);

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const registry = $derived(getRulesRegistry());

  let sortColumn = $state<SortableColumn>("input");
  let sortDirection = $state<"asc" | "desc">("asc");
  let filterType = $state<string>("all");

  const filterOptions = $derived(buildFilterOptions(registry));
  const filteredMoves = $derived(filterMoves(moves, filterType, registry));
  const sortedMoves = $derived(sortMoves(filteredMoves, sortColumn, sortDirection));

  function toggleSort(column: typeof sortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = "asc";
    }
  }

  function handleRowClick(move: State) {
    selectMove(move.input);
    onEditMove(move.input);
  }

  function handleMoveCreated(input: string) {
    selectMove(input);
    onEditMove(input);
  }
</script>

<div class="frame-data-container">
  <div class="toolbar">
    <select bind:value={filterType}>
      {#each filterOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    <span class="count">{filteredMoves.length} moves</span>
    <div class="toolbar-spacer"></div>
    <button class="new-move-btn" onclick={() => showCreateModal = true}>
      + New Move
    </button>
  </div>

  <div class="table-wrapper">
    <table class="frame-table">
      <thead>
        <tr>
          <th class="sortable" onclick={() => toggleSort("input")}>
            Input {sortColumn === "input" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable" onclick={() => toggleSort("name")}>
            Name {sortColumn === "name" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("startup")}>
            Startup {sortColumn === "startup" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("active")}>
            Active {sortColumn === "active" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("recovery")}>
            Recovery {sortColumn === "recovery" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("total")}>
            Total {sortColumn === "total" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("damage")}>
            Damage {sortColumn === "damage" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("advantage_hit")}>
            On Hit {sortColumn === "advantage_hit" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th class="sortable num" onclick={() => toggleSort("advantage_block")}>
            On Block {sortColumn === "advantage_block" ? (sortDirection === "asc" ? "↑" : "↓") : ""}
          </th>
          <th>Guard</th>
        </tr>
      </thead>
      <tbody>
        {#each sortedMoves as move}
          <tr onclick={() => handleRowClick(move)}>
            <td class="input-cell">
              {#if move.type === 'system'}
                <span class="origin-badge global" title="From global state">🌐</span>
              {/if}
              {move.input}
            </td>
            <td>{move.name}</td>
            <td class="num">{move.startup}</td>
            <td class="num">{move.active}</td>
            <td class="num">{move.recovery}</td>
            <td class="num">{getTotal(move)}</td>
            <td class="num">{move.damage}</td>
            <td class="num advantage" class:positive={getAdvantageHit(move) >= 0}>
              {formatAdvantage(getAdvantageHit(move))}
            </td>
            <td class="num advantage" class:positive={getAdvantageBlock(move) >= 0}>
              {formatAdvantage(getAdvantageBlock(move))}
            </td>
            <td class="guard">{move.guard}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <CreateMoveModal
    open={showCreateModal}
    onClose={() => showCreateModal = false}
    onCreated={handleMoveCreated}
  />
</div>

<style>
  .frame-data-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .count {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .table-wrapper {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .frame-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  th, td {
    padding: 8px 12px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  th {
    background: var(--bg-secondary);
    font-weight: 600;
    position: sticky;
    top: 0;
  }

  th.sortable {
    cursor: pointer;
    user-select: none;
  }

  th.sortable:hover {
    background: var(--bg-tertiary);
  }

  th.num, td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  tbody tr {
    cursor: pointer;
  }

  tbody tr:hover {
    background: var(--bg-tertiary);
  }

  .input-cell {
    font-family: monospace;
    font-weight: 600;
    color: var(--accent);
  }

  .origin-badge {
    margin-right: 4px;
    font-size: 11px;
  }

  .advantage {
    color: var(--accent);
  }

  .advantage.positive {
    color: var(--success);
  }

  .guard {
    text-transform: capitalize;
  }

  .toolbar-spacer {
    flex: 1;
  }

  .new-move-btn {
    background: var(--accent);
    border-color: var(--accent);
    padding: 6px 12px;
    font-size: 13px;
  }

  .new-move-btn:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }
</style>
