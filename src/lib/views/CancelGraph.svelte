<script lang="ts">
  import { getCurrentCharacter, getRulesRegistry } from "$lib/stores/character.svelte";
  import type { State, CancelTable } from "$lib/types";

  const characterData = $derived(getCurrentCharacter());
  const moves = $derived(characterData?.moves ?? []);
  const cancelTable = $derived(characterData?.cancel_table);
  const registry = $derived(getRulesRegistry());

  // Default chain order if not specified in registry
  const defaultChainOrder = ["L", "M", "H"];

  // Get chain order from registry or use default
  const chainOrder = $derived(registry?.chain_order ?? defaultChainOrder);

  // Default filter groups for type detection
  const defaultSpecialTypes = ["special", "ex", "rekka"];
  const defaultSuperTypes = ["super"];

  // Get type groups from registry
  const specialTypes = $derived(
    registry?.move_types?.filter_groups?.["specials"] ?? defaultSpecialTypes
  );
  const superTypes = $derived(
    registry?.move_types?.filter_groups?.["supers"] ?? defaultSuperTypes
  );

  // Check if cancel table is effectively empty (tag-based system)
  const isCancelTableEmpty = $derived.by(() => {
    if (!cancelTable) return true;
    return (
      Object.keys(cancelTable.chains).length === 0 &&
      cancelTable.special_cancels.length === 0 &&
      cancelTable.super_cancels.length === 0 &&
      cancelTable.jump_cancels.length === 0
    );
  });

  // Extract button from input (e.g., "5L" -> "L", "j.H" -> "H", "2M" -> "M")
  function extractButton(input: string): string | null {
    // Match trailing alphabetic characters
    const match = input.match(/([A-Z]+)$/i);
    return match ? match[1].toUpperCase() : null;
  }

  // Check if a state has a specific tag (tags are resolved by rules)
  function hasTag(move: State, tag: string): boolean {
    return (move as any).tags?.includes(tag) ?? false;
  }

  // Check if state is a special type
  function isSpecialType(move: State): boolean {
    if (move.type) {
      return specialTypes.includes(move.type);
    }
    // Fallback to input pattern
    return /\d{3,}/.test(move.input);
  }

  // Check if state is a super type
  function isSuperType(move: State): boolean {
    if (move.type) {
      return superTypes.includes(move.type);
    }
    // Fallback to 6+ digit input pattern
    return /\d{6,}/.test(move.input);
  }

  // SVG dimensions
  const width = 800;
  const height = 600;
  const centerX = width / 2;
  const centerY = height / 2;
  const radius = Math.min(width, height) * 0.35;

  // Edge colors by type
  const edgeColors = {
    chain: "#4ade80", // green - chains
    special: "#60a5fa", // blue - special cancels
    super: "#fbbf24", // yellow - super cancels
    jump: "#c084fc", // purple - jump cancels
  };

  // Hover state
  let hoveredMove = $state<string | null>(null);

  // Calculate node positions in a circle
  interface NodePosition {
    input: string;
    name: string;
    x: number;
    y: number;
  }

  const nodePositions = $derived.by(() => {
    if (moves.length === 0) return [];

    const positions: NodePosition[] = [];
    const angleStep = (2 * Math.PI) / moves.length;

    moves.forEach((move, index) => {
      const angle = index * angleStep - Math.PI / 2; // Start from top
      positions.push({
        input: move.input,
        name: move.name,
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      });
    });

    return positions;
  });

  // Get position for a move by input
  function getNodePosition(input: string): NodePosition | undefined {
    return nodePositions.find((n) => n.input === input);
  }

  // Edge type definition
  interface Edge {
    from: string;
    to: string;
    type: "chain" | "special" | "super" | "jump";
  }

  // Build edges from cancel table OR from tags if cancel table is empty
  const edges = $derived.by(() => {
    const edgeList: Edge[] = [];
    const moveInputs = new Set(moves.map((m) => m.input));
    const movesByInput = new Map(moves.map((m) => [m.input, m]));

    // If cancel table has explicit data, use it
    if (!isCancelTableEmpty && cancelTable) {
      // Chain edges (from chains object)
      for (const [fromMove, targets] of Object.entries(cancelTable.chains)) {
        if (!moveInputs.has(fromMove)) continue;
        for (const toMove of targets) {
          if (moveInputs.has(toMove)) {
            edgeList.push({ from: fromMove, to: toMove, type: "chain" });
          }
        }
      }

      // Special cancel edges
      const specialMovesList = moves.filter((m) => isSpecialType(m));
      for (const fromInput of cancelTable.special_cancels) {
        if (!moveInputs.has(fromInput)) continue;
        for (const specialMove of specialMovesList) {
          if (fromInput !== specialMove.input) {
            const existingChain = edgeList.find(
              (e) => e.from === fromInput && e.to === specialMove.input && e.type === "chain"
            );
            if (!existingChain) {
              edgeList.push({ from: fromInput, to: specialMove.input, type: "special" });
            }
          }
        }
      }

      // Super cancel edges
      const superMovesList = moves.filter((m) => isSuperType(m));
      for (const fromInput of cancelTable.super_cancels) {
        if (!moveInputs.has(fromInput)) continue;
        for (const superMove of superMovesList) {
          if (fromInput !== superMove.input) {
            edgeList.push({ from: fromInput, to: superMove.input, type: "super" });
          }
        }
      }

      return edgeList;
    }

    // Tag-based edge derivation (when cancel table is empty)
    // Build index of moves by button for chain lookup
    const movesByButton = new Map<string, State[]>();
    for (const move of moves) {
      const button = extractButton(move.input);
      if (button) {
        const list = movesByButton.get(button) ?? [];
        list.push(move);
        movesByButton.set(button, list);
      }
    }

    // Get special and super moves for cancel targets
    const specialMovesList = moves.filter((m) => isSpecialType(m));
    const superMovesList = moves.filter((m) => isSuperType(m));

    for (const move of moves) {
      const moveButton = extractButton(move.input);
      const buttonIndex = moveButton ? chainOrder.indexOf(moveButton) : -1;

      // Chain tag: can cancel into moves with buttons later in chain order
      if (hasTag(move, "chain") && buttonIndex >= 0) {
        for (let i = buttonIndex + 1; i < chainOrder.length; i++) {
          const targetButton = chainOrder[i];
          const targetMoves = movesByButton.get(targetButton) ?? [];
          for (const targetMove of targetMoves) {
            // Only chain into normals (same position: standing/crouching/jumping)
            // For simplicity, just add the edge
            if (move.input !== targetMove.input) {
              edgeList.push({ from: move.input, to: targetMove.input, type: "chain" });
            }
          }
        }
      }

      // Self-gatling tag: can cancel into itself
      if (hasTag(move, "self_gatling")) {
        edgeList.push({ from: move.input, to: move.input, type: "chain" });
      }

      // Special cancel tag: can cancel into any special move
      if (hasTag(move, "special_cancel")) {
        for (const specialMove of specialMovesList) {
          if (move.input !== specialMove.input) {
            // Avoid duplicate if already added as chain
            const exists = edgeList.some(
              (e) => e.from === move.input && e.to === specialMove.input
            );
            if (!exists) {
              edgeList.push({ from: move.input, to: specialMove.input, type: "special" });
            }
          }
        }
      }

      // Super cancel tag: can cancel into any super move
      if (hasTag(move, "super_cancel")) {
        for (const superMove of superMovesList) {
          if (move.input !== superMove.input) {
            edgeList.push({ from: move.input, to: superMove.input, type: "super" });
          }
        }
      }

      // Jump cancel tag: mark for visual indicator (handled separately)
      // No edges needed since jump is not a move
    }

    return edgeList;
  });

  // Calculate edge path with curve for better visibility
  function getEdgePath(from: NodePosition, to: NodePosition): string {
    // Calculate control point for a curved line
    const midX = (from.x + to.x) / 2;
    const midY = (from.y + to.y) / 2;

    // Offset the control point toward the center for inward curve
    const dx = midX - centerX;
    const dy = midY - centerY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curveAmount = 0.3;
    const controlX = midX - (dx / dist) * radius * curveAmount;
    const controlY = midY - (dy / dist) * radius * curveAmount;

    return `M ${from.x} ${from.y} Q ${controlX} ${controlY} ${to.x} ${to.y}`;
  }

  // Calculate arrow marker position along the curve
  function getArrowTransform(from: NodePosition, to: NodePosition): string {
    // Position arrow at 70% along the path
    const t = 0.7;
    const midX = (from.x + to.x) / 2;
    const midY = (from.y + to.y) / 2;
    const dx = midX - centerX;
    const dy = midY - centerY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curveAmount = 0.3;
    const controlX = midX - (dx / dist) * radius * curveAmount;
    const controlY = midY - (dy / dist) * radius * curveAmount;

    // Quadratic bezier point at t
    const x = (1 - t) * (1 - t) * from.x + 2 * (1 - t) * t * controlX + t * t * to.x;
    const y = (1 - t) * (1 - t) * from.y + 2 * (1 - t) * t * controlY + t * t * to.y;

    // Calculate angle
    const tangentX = 2 * (1 - t) * (controlX - from.x) + 2 * t * (to.x - controlX);
    const tangentY = 2 * (1 - t) * (controlY - from.y) + 2 * t * (to.y - controlY);
    const angle = (Math.atan2(tangentY, tangentX) * 180) / Math.PI;

    return `translate(${x}, ${y}) rotate(${angle})`;
  }

  // Check if edge is highlighted (connected to hovered node)
  function isEdgeHighlighted(edge: Edge): boolean {
    if (!hoveredMove) return true;
    return edge.from === hoveredMove || edge.to === hoveredMove;
  }

  // Check if node is highlighted
  function isNodeHighlighted(input: string): boolean {
    if (!hoveredMove) return true;
    if (input === hoveredMove) return true;
    // Also highlight connected nodes
    return edges.some(
      (e) =>
        (e.from === hoveredMove && e.to === input) ||
        (e.to === hoveredMove && e.from === input)
    );
  }

  // Check if move has jump cancel (from cancel table or tags)
  function hasJumpCancel(input: string): boolean {
    // Check explicit cancel table first
    if (cancelTable?.jump_cancels?.includes(input)) {
      return true;
    }
    // Check for jump_cancel tag
    const move = moves.find((m) => m.input === input);
    return move ? hasTag(move, "jump_cancel") : false;
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
              class:dimmed={!isNodeHighlighted(node.input)}
              class:hovered={hoveredMove === node.input}
              onmouseenter={() => (hoveredMove = node.input)}
              onmouseleave={() => (hoveredMove = null)}
              role="button"
              tabindex="0"
            >
              <!-- Node circle -->
              <circle
                cx={node.x}
                cy={node.y}
                r="24"
                class="node-circle"
              />

              <!-- Jump cancel indicator -->
              {#if hasJumpCancel(node.input)}
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
                {node.input}
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
      {@const hoveredNode = nodePositions.find((n) => n.input === hoveredMove)}
      {@const outgoing = edges.filter((e) => e.from === hoveredMove)}
      {@const incoming = edges.filter((e) => e.to === hoveredMove)}
      {#if hoveredNode}
        <div class="hover-info">
          <strong>{hoveredNode.input}</strong> - {hoveredNode.name}
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
