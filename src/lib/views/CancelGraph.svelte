<script lang="ts">
  import { getCurrentCharacter, getRulesRegistry } from "$lib/stores/character.svelte";
  import type { State, CancelCondition, CancelTagRule } from "$lib/types";
  import { getStateKey } from "$lib/utils";

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const cancelTable = $derived(characterData?.cancel_table);
  const registry = $derived(getRulesRegistry());

  const defaultChainOrder = ["L", "M", "H"];
  const chainOrder = $derived(registry?.chain_order ?? defaultChainOrder);

  const defaultSpecialTypes = ["special", "ex", "rekka"];
  const defaultSuperTypes = ["super"];

  const specialTypes = $derived(
    registry?.move_types?.filter_groups?.["specials"] ?? defaultSpecialTypes
  );
  const superTypes = $derived(
    registry?.move_types?.filter_groups?.["supers"] ?? defaultSuperTypes
  );

  const isCancelTableEmpty = $derived.by(() => {
    if (!cancelTable) return true;
    return cancelTable.tag_rules.length === 0 && Object.keys(cancelTable.deny).length === 0;
  });

  function extractButton(input: string): string | null {
    const match = input.match(/([A-Z]+)$/i);
    return match ? match[1].toUpperCase() : null;
  }

  function hasTag(move: State, tag: string): boolean {
    return move.tags?.includes(tag) ?? false;
  }

  function isSpecialType(move: State): boolean {
    if (move.type) {
      return specialTypes.includes(move.type);
    }
    return /\d{3,}/.test(move.input);
  }

  function isSuperType(move: State): boolean {
    if (move.type) {
      return superTypes.includes(move.type);
    }
    return /\d{6,}/.test(move.input);
  }

  function normalizeSelector(selector: string): string {
    return selector.toLowerCase();
  }

  function stateTokens(move: State): Set<string> {
    const tokens = new Set<string>([normalizeSelector(move.input), normalizeSelector(getStateKey(move))]);
    if (move.type) tokens.add(normalizeSelector(move.type));
    for (const tag of move.tags ?? []) {
      tokens.add(normalizeSelector(tag));
    }
    return tokens;
  }

  function matchesSelector(selector: string, move: State): boolean {
    const normalized = normalizeSelector(selector);
    return normalized === "any" || stateTokens(move).has(normalized);
  }

  function conditionTokens(condition: CancelCondition | undefined): string[] {
    if (!condition) return ["always"];
    return Array.isArray(condition) ? condition : [condition];
  }

  const width = 800;
  const height = 600;
  const centerX = width / 2;
  const centerY = height / 2;
  const radius = Math.min(width, height) * 0.35;

  const edgeColors = {
    chain: "#4ade80",
    special: "#60a5fa",
    super: "#fbbf24",
    jump: "#c084fc",
  };

  let hoveredMove = $state<string | null>(null);

  interface NodePosition {
    key: string;
    input: string;
    label: string;
    name: string;
    x: number;
    y: number;
  }

  const nodePositions = $derived.by(() => {
    if (moves.length === 0) return [];

    const positions: NodePosition[] = [];
    const angleStep = (2 * Math.PI) / moves.length;

    moves.forEach((move, index) => {
      const angle = index * angleStep - Math.PI / 2;
      const key = getStateKey(move);
      positions.push({
        key,
        input: move.input,
        label: key === move.input ? move.input : key,
        name: move.name,
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      });
    });

    return positions;
  });

  function getNodePosition(key: string): NodePosition | undefined {
    return nodePositions.find((n) => n.key === key);
  }

  interface Edge {
    from: string;
    to: string;
    type: "chain" | "special" | "super" | "jump";
    label?: string;
  }

  function addEdge(edgeList: Edge[], seen: Set<string>, edge: Edge) {
    const key = `${edge.from}\0${edge.to}\0${edge.type}`;
    if (seen.has(key)) return;
    seen.add(key);
    edgeList.push(edge);
  }

  function isDenied(from: State, to: State): boolean {
    if (!cancelTable) return false;
    const fromKey = getStateKey(from);
    const toKey = getStateKey(to);
    const denyLists = [
      cancelTable.deny[fromKey],
      cancelTable.deny[from.input],
      cancelTable.deny[normalizeSelector(fromKey)],
      cancelTable.deny[normalizeSelector(from.input)],
    ].filter(Boolean) as string[][];

    return denyLists.some((targets) =>
      targets.some((target) =>
        target === toKey ||
        target === to.input ||
        normalizeSelector(target) === normalizeSelector(toKey) ||
        normalizeSelector(target) === normalizeSelector(to.input)
      )
    );
  }

  function inferEdgeType(rule: CancelTagRule, targetMove: State): Edge["type"] {
    const to = normalizeSelector(rule.to);
    if (to === "super" || isSuperType(targetMove)) return "super";
    if (to === "special" || isSpecialType(targetMove)) return "special";
    return "chain";
  }

  function ruleLabel(rule: CancelTagRule): string {
    return conditionTokens(rule.on).join("+");
  }

  const edges = $derived.by(() => {
    const edgeList: Edge[] = [];
    const seen = new Set<string>();

    if (!isCancelTableEmpty && cancelTable) {
      for (const rule of cancelTable.tag_rules) {
        const fromMoves = moves.filter((move) => matchesSelector(rule.from, move));
        const toMoves = moves.filter((move) => matchesSelector(rule.to, move));

        for (const fromMove of fromMoves) {
          for (const toMove of toMoves) {
            if (isDenied(fromMove, toMove)) continue;
            addEdge(edgeList, seen, {
              from: getStateKey(fromMove),
              to: getStateKey(toMove),
              type: inferEdgeType(rule, toMove),
              label: ruleLabel(rule),
            });
          }
        }
      }

      return edgeList;
    }

    const movesByButton = new Map<string, State[]>();
    for (const move of moves) {
      const button = extractButton(move.input);
      if (button) {
        const list = movesByButton.get(button) ?? [];
        list.push(move);
        movesByButton.set(button, list);
      }
    }

    const specialMovesList = moves.filter((m) => isSpecialType(m));
    const superMovesList = moves.filter((m) => isSuperType(m));

    for (const move of moves) {
      const moveButton = extractButton(move.input);
      const buttonIndex = moveButton ? chainOrder.indexOf(moveButton) : -1;

      if (hasTag(move, "chain") && buttonIndex >= 0) {
        for (let i = buttonIndex + 1; i < chainOrder.length; i++) {
          const targetButton = chainOrder[i];
          const targetMoves = movesByButton.get(targetButton) ?? [];
          for (const targetMove of targetMoves) {
            if (getStateKey(move) !== getStateKey(targetMove)) {
              addEdge(edgeList, seen, { from: getStateKey(move), to: getStateKey(targetMove), type: "chain" });
            }
          }
        }
      }

      if (hasTag(move, "self_gatling")) {
        const key = getStateKey(move);
        addEdge(edgeList, seen, { from: key, to: key, type: "chain" });
      }

      if (hasTag(move, "special_cancel")) {
        for (const specialMove of specialMovesList) {
          if (getStateKey(move) !== getStateKey(specialMove)) {
            addEdge(edgeList, seen, { from: getStateKey(move), to: getStateKey(specialMove), type: "special" });
          }
        }
      }

      if (hasTag(move, "super_cancel")) {
        for (const superMove of superMovesList) {
          if (getStateKey(move) !== getStateKey(superMove)) {
            addEdge(edgeList, seen, { from: getStateKey(move), to: getStateKey(superMove), type: "super" });
          }
        }
      }
    }

    return edgeList;
  });

  function getEdgePath(from: NodePosition, to: NodePosition): string {
    if (from.key === to.key) {
      return `M ${from.x} ${from.y - 24} C ${from.x + 52} ${from.y - 78}, ${from.x + 78} ${from.y + 20}, ${from.x + 24} ${from.y + 24}`;
    }

    const midX = (from.x + to.x) / 2;
    const midY = (from.y + to.y) / 2;
    const dx = midX - centerX;
    const dy = midY - centerY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curveAmount = 0.3;
    const controlX = dist === 0 ? midX : midX - (dx / dist) * radius * curveAmount;
    const controlY =
      dist === 0 ? midY - radius * curveAmount : midY - (dy / dist) * radius * curveAmount;

    return `M ${from.x} ${from.y} Q ${controlX} ${controlY} ${to.x} ${to.y}`;
  }

  function getArrowTransform(from: NodePosition, to: NodePosition): string {
    if (from.key === to.key) {
      return `translate(${from.x + 24}, ${from.y + 24}) rotate(140)`;
    }

    const t = 0.7;
    const midX = (from.x + to.x) / 2;
    const midY = (from.y + to.y) / 2;
    const dx = midX - centerX;
    const dy = midY - centerY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curveAmount = 0.3;
    const controlX = dist === 0 ? midX : midX - (dx / dist) * radius * curveAmount;
    const controlY =
      dist === 0 ? midY - radius * curveAmount : midY - (dy / dist) * radius * curveAmount;

    const x = (1 - t) * (1 - t) * from.x + 2 * (1 - t) * t * controlX + t * t * to.x;
    const y = (1 - t) * (1 - t) * from.y + 2 * (1 - t) * t * controlY + t * t * to.y;
    const tangentX = 2 * (1 - t) * (controlX - from.x) + 2 * t * (to.x - controlX);
    const tangentY = 2 * (1 - t) * (controlY - from.y) + 2 * t * (to.y - controlY);
    const angle = (Math.atan2(tangentY, tangentX) * 180) / Math.PI;

    return `translate(${x}, ${y}) rotate(${angle})`;
  }

  function isEdgeHighlighted(edge: Edge): boolean {
    if (!hoveredMove) return true;
    return edge.from === hoveredMove || edge.to === hoveredMove;
  }

  function isNodeHighlighted(key: string): boolean {
    if (!hoveredMove) return true;
    if (key === hoveredMove) return true;
    return edges.some(
      (e) =>
        (e.from === hoveredMove && e.to === key) ||
        (e.to === hoveredMove && e.from === key)
    );
  }

  function hasJumpCancel(key: string): boolean {
    const move = moves.find((m) => getStateKey(m) === key);
    if (!move) return false;
    if (hasTag(move, "jump_cancel")) return true;

    return cancelTable?.tag_rules.some((rule) => {
      const to = normalizeSelector(rule.to);
      return (to === "jump" || to === "jump_cancel") && matchesSelector(rule.from, move);
    }) ?? false;
  }
</script>

<div class="cancel-graph-container">
  <div class="graph-header">
    <h2>Cancel Graph</h2>
    <p class="graph-description">
      Nodes represent moves. Edges show cancel relationships.
      Hover over a move to highlight its connections.
    </p>
  </div>

  {#if moves.length === 0}
    <div class="empty-state">
      <p>No moves defined for this character.</p>
    </div>
  {:else}
    <div class="graph-wrapper">
      <svg viewBox="0 0 {width} {height}" class="graph-svg">
        <title>Cancel graph showing move relationships</title>
        <!-- Definitions for arrow markers -->
        <defs>
          {#each Object.entries(edgeColors) as [type, color]}
            <marker
              id="arrow-{type}"
              viewBox="0 0 10 10"
              refX="5"
              refY="5"
              markerWidth="4"
              markerHeight="4"
              orient="auto-start-reverse"
            >
              <path d="M 0 0 L 10 5 L 0 10 z" fill={color} />
            </marker>
          {/each}
        </defs>

        <!-- Edges -->
        <g class="edges">
          {#each edges as edge}
            {@const from = getNodePosition(edge.from)}
            {@const to = getNodePosition(edge.to)}
            {#if from && to}
              <path
                d={getEdgePath(from, to)}
                stroke={edgeColors[edge.type]}
                stroke-width="2"
                fill="none"
                marker-end="url(#arrow-{edge.type})"
                class="edge"
                class:dimmed={!isEdgeHighlighted(edge)}
              />
            {/if}
          {/each}
        </g>

        <!-- Nodes -->
        <g class="nodes">
          {#each nodePositions as node}
            <g
              class="node"
              class:dimmed={!isNodeHighlighted(node.key)}
              class:hovered={hoveredMove === node.key}
              onmouseenter={() => (hoveredMove = node.key)}
              onmouseleave={() => (hoveredMove = null)}
              onfocus={() => (hoveredMove = node.key)}
              onblur={() => (hoveredMove = null)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  hoveredMove = hoveredMove === node.key ? null : node.key;
                }
              }}
              role="button"
              tabindex="0"
              aria-label={`${node.label} - ${node.name}`}
            >
              <!-- Node circle -->
              <circle
                cx={node.x}
                cy={node.y}
                r="24"
                class="node-circle"
              />

              <!-- Jump cancel indicator -->
              {#if hasJumpCancel(node.key)}
                <circle
                  cx={node.x}
                  cy={node.y}
                  r="28"
                  class="jump-cancel-ring"
                />
              {/if}

              <!-- Node label -->
              <text
                x={node.x}
                y={node.y}
                class="node-label"
                dominant-baseline="central"
                text-anchor="middle"
              >
                {node.label}
              </text>
            </g>
          {/each}
        </g>
      </svg>

      <!-- Legend -->
      <div class="legend">
        <h4>Edge Types</h4>
        <div class="legend-items">
          <div class="legend-item">
            <div class="legend-line" style="background: {edgeColors.chain}"></div>
            <span>Chain</span>
          </div>
          <div class="legend-item">
            <div class="legend-line" style="background: {edgeColors.special}"></div>
            <span>Special Cancel</span>
          </div>
          <div class="legend-item">
            <div class="legend-line" style="background: {edgeColors.super}"></div>
            <span>Super Cancel</span>
          </div>
          <div class="legend-item">
            <div class="legend-ring"></div>
            <span>Jump Cancel</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Move info on hover -->
    {#if hoveredMove}
      {@const hoveredNode = nodePositions.find((n) => n.key === hoveredMove)}
      {@const outgoing = edges.filter((e) => e.from === hoveredMove)}
      {@const incoming = edges.filter((e) => e.to === hoveredMove)}
      {#if hoveredNode}
        <div class="hover-info">
          <strong>{hoveredNode.label}</strong> - {hoveredNode.name}
          {#if hoveredNode.label !== hoveredNode.input}
            <span class="hover-input">({hoveredNode.input})</span>
          {/if}
          <div class="hover-connections">
            {#if outgoing.length > 0}
              <div class="connection-list">
                <span class="connection-label">Cancels into:</span>
                {outgoing.map((e) => e.to).join(", ")}
              </div>
            {/if}
            {#if incoming.length > 0}
              <div class="connection-list">
                <span class="connection-label">Canceled from:</span>
                {incoming.map((e) => e.from).join(", ")}
              </div>
            {/if}
            {#if hasJumpCancel(hoveredMove)}
              <div class="connection-list">
                <span class="connection-label jump">Jump cancelable</span>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .cancel-graph-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 16px;
  }

  .graph-header h2 {
    margin: 0 0 4px 0;
    font-size: 20px;
  }

  .graph-description {
    color: var(--text-secondary);
    font-size: 13px;
    margin: 0;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-secondary);
  }

  .graph-wrapper {
    display: flex;
    flex: 1;
    gap: 16px;
    min-height: 0;
  }

  .graph-svg {
    flex: 1;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    min-height: 400px;
    max-height: 600px;
  }

  .edges .edge {
    transition: opacity 0.2s ease;
  }

  .edges .edge.dimmed {
    opacity: 0.15;
  }

  .nodes .node {
    cursor: pointer;
    transition: opacity 0.2s ease;
  }

  .nodes .node:focus-visible .node-circle {
    stroke: var(--accent);
    stroke-width: 3;
    outline: none;
  }

  .nodes .node:focus {
    outline: none;
  }

  .nodes .node.dimmed {
    opacity: 0.3;
  }

  .nodes .node.hovered .node-circle {
    fill: var(--accent);
  }

  .node-circle {
    fill: var(--bg-tertiary);
    stroke: var(--border);
    stroke-width: 2;
    transition: fill 0.15s ease;
  }

  .node:hover .node-circle {
    fill: var(--accent);
  }

  .jump-cancel-ring {
    fill: none;
    stroke: #c084fc;
    stroke-width: 2;
    stroke-dasharray: 4 2;
  }

  .node-label {
    fill: var(--text-primary);
    font-size: 11px;
    font-weight: 600;
    pointer-events: none;
    user-select: none;
  }

  .legend {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    width: 160px;
    flex-shrink: 0;
  }

  .legend h4 {
    margin: 0 0 12px 0;
    font-size: 13px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .legend-items {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .legend-line {
    width: 24px;
    height: 3px;
    border-radius: 2px;
  }

  .legend-ring {
    width: 16px;
    height: 16px;
    border: 2px dashed #c084fc;
    border-radius: 50%;
  }

  .hover-info {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 16px;
  }

  .hover-info strong {
    font-size: 15px;
  }

  .hover-input {
    color: var(--text-secondary);
    font-family: monospace;
    font-size: 12px;
    margin-left: 4px;
  }

  .hover-connections {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .connection-list {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .connection-label {
    font-weight: 600;
    margin-right: 4px;
  }

  .connection-label.jump {
    color: #c084fc;
  }
</style>
