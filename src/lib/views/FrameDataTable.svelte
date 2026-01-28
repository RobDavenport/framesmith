<script lang="ts">
  import { getCurrentCharacter, selectMove } from "$lib/stores/character.svelte";
  import type { Move } from "$lib/types";

  interface Props {
    onEditMove: (input: string) => void;
  }

  let { onEditMove }: Props = $props();

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);

  // Only include sortable columns (string or number types)
  type SortableColumn = "input" | "name" | "startup" | "active" | "recovery" | "damage" | "hitstun" | "blockstun" | "hitstop" | "guard" | "animation" | "total" | "advantage_hit" | "advantage_block";

  let sortColumn = $state<SortableColumn>("input");
  let sortDirection = $state<"asc" | "desc">("asc");
  let filterType = $state<string>("all");

  const filterOptions = [
    { value: "all", label: "All Moves" },
    { value: "normal", label: "Normals" },
    { value: "special", label: "Specials" },
  ];

  function isSpecialMove(input: string): boolean {
    return /\d{3,}/.test(input); // Contains 3+ consecutive digits (motion input)
  }

  function getTotal(move: Move): number {
    return move.startup + move.active + move.recovery;
  }

  function getAdvantageHit(move: Move): number {
    return move.hitstun - move.recovery;
  }

  function getAdvantageBlock(move: Move): number {
    return move.blockstun - move.recovery;
  }

  const filteredMoves = $derived.by(() => {
    let filtered = moves;
    if (filterType === "normal") {
      filtered = moves.filter((m) => !isSpecialMove(m.input));
    } else if (filterType === "special") {
      filtered = moves.filter((m) => isSpecialMove(m.input));
    }
    return filtered;
  });

  const sortedMoves = $derived.by(() => {
    return [...filteredMoves].sort((a, b) => {
      let aVal: number | string;
      let bVal: number | string;

      if (sortColumn === "total") {
        aVal = getTotal(a);
        bVal = getTotal(b);
      } else if (sortColumn === "advantage_hit") {
        aVal = getAdvantageHit(a);
        bVal = getAdvantageHit(b);
      } else if (sortColumn === "advantage_block") {
        aVal = getAdvantageBlock(a);
        bVal = getAdvantageBlock(b);
      } else {
        // All other sortable columns are direct Move properties
        aVal = a[sortColumn] as string | number;
        bVal = b[sortColumn] as string | number;
      }

      if (typeof aVal === "string" && typeof bVal === "string") {
        return sortDirection === "asc" ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
      }
      return sortDirection === "asc" ? Number(aVal) - Number(bVal) : Number(bVal) - Number(aVal);
    });
  });

  function toggleSort(column: typeof sortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = "asc";
    }
  }

  function handleRowClick(move: Move) {
    selectMove(move.input);
    onEditMove(move.input);
  }

  function formatAdvantage(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
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
            <td class="input-cell">{move.input}</td>
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

  .advantage {
    color: var(--accent);
  }

  .advantage.positive {
    color: var(--success);
  }

  .guard {
    text-transform: capitalize;
  }
</style>
