<script lang="ts">
  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open, onClose }: Props = $props();

  let activeSection = $state("overview");

  const sections = [
    { id: "overview", label: "Overview" },
    { id: "format", label: "File Format" },
    { id: "apply", label: "Apply Rules" },
    { id: "validate", label: "Validate Rules" },
    { id: "match", label: "Match Criteria" },
    { id: "constraints", label: "Constraints" },
    { id: "builtin", label: "Built-in Validation" },
    { id: "hierarchy", label: "Rule Hierarchy" },
    { id: "examples", label: "Examples" },
  ];

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleBackdropClick}>
    <div class="help-panel">
      <div class="help-header">
        <h2>Rules Documentation</h2>
        <button class="close-btn" onclick={onClose}>&times;</button>
      </div>

      <div class="help-body">
        <nav class="help-nav">
          {#each sections as section}
            <button
              class="nav-item"
              class:active={activeSection === section.id}
              onclick={() => (activeSection = section.id)}
            >
              {section.label}
            </button>
          {/each}
        </nav>

        <div class="help-content">
          {#if activeSection === "overview"}
            <h3>Overview</h3>
            <p>
              Framesmith supports a flexible rules system that lets you:
            </p>
            <ul>
              <li><strong>Apply</strong> default values to moves based on matching criteria</li>
              <li><strong>Validate</strong> moves to enforce constraints with configurable severity</li>
            </ul>
            <p>
              Rules are defined in JSON files named <code>framesmith.rules.json</code>. They can exist at two levels:
            </p>
            <ol>
              <li><strong>Project-level:</strong> At the project root (applies to all characters)</li>
              <li><strong>Character-level:</strong> Inside a character directory (overrides project rules)</li>
            </ol>
          {:else if activeSection === "format"}
            <h3>Rules File Format</h3>
            <pre><code>{`{
  "version": 1,
  "apply": [...],
  "validate": [...]
}`}</code></pre>
            <table>
              <thead>
                <tr><th>Field</th><th>Type</th><th>Required</th><th>Description</th></tr>
              </thead>
              <tbody>
                <tr><td><code>version</code></td><td>number</td><td>Yes</td><td>Schema version. Must be 1.</td></tr>
                <tr><td><code>apply</code></td><td>ApplyRule[]</td><td>No</td><td>Rules that set default values.</td></tr>
                <tr><td><code>validate</code></td><td>ValidateRule[]</td><td>No</td><td>Rules that enforce constraints.</td></tr>
              </tbody>
            </table>
          {:else if activeSection === "apply"}
            <h3>Apply Rules</h3>
            <p>
              Apply rules set default values on moves that match certain criteria.
              They only fill in values that are <strong>unset</strong> (null, empty, or zero).
            </p>
            <pre><code>{`{
  "match": { "type": "normal" },
  "set": {
    "hitstop": 8,
    "pushback": { "hit": 2, "block": 2 }
  }
}`}</code></pre>
            <h4>How Apply Rules Work</h4>
            <ol>
              <li>Rules are evaluated in order (project rules first, then character rules)</li>
              <li>Character rules with the same match spec <strong>replace</strong> project rules</li>
              <li>For each matching rule, only <strong>unset</strong> fields are filled in</li>
              <li>Later rules can override earlier defaults (if the field is still unset)</li>
            </ol>
          {:else if activeSection === "validate"}
            <h3>Validate Rules</h3>
            <p>
              Validate rules enforce constraints on moves, producing errors or warnings.
            </p>
            <pre><code>{`{
  "match": { "type": "special" },
  "require": {
    "startup": { "min": 1 },
    "animation": { "exists": true }
  },
  "severity": "error",
  "message": "Special moves must have startup and animation"
}`}</code></pre>
            <table>
              <thead>
                <tr><th>Field</th><th>Type</th><th>Required</th><th>Description</th></tr>
              </thead>
              <tbody>
                <tr><td><code>match</code></td><td>MatchSpec</td><td>Yes</td><td>Which moves this rule applies to.</td></tr>
                <tr><td><code>require</code></td><td>object</td><td>Yes</td><td>Constraint definitions.</td></tr>
                <tr><td><code>severity</code></td><td>"error" | "warning"</td><td>Yes</td><td>How to report violations.</td></tr>
                <tr><td><code>message</code></td><td>string</td><td>No</td><td>Custom message for violations.</td></tr>
              </tbody>
            </table>
          {:else if activeSection === "match"}
            <h3>Match Criteria</h3>
            <p>
              The <code>match</code> object determines which moves a rule applies to.
              All specified fields must match (<strong>AND</strong> logic).
              Within a single field, multiple values use <strong>OR</strong> logic.
            </p>
            <table>
              <thead>
                <tr><th>Field</th><th>Type</th><th>Description</th></tr>
              </thead>
              <tbody>
                <tr><td><code>type</code></td><td>string | string[]</td><td>normal, command_normal, special, super, movement, throw</td></tr>
                <tr><td><code>button</code></td><td>string | string[]</td><td>Button from input (e.g., "236P" → "P")</td></tr>
                <tr><td><code>guard</code></td><td>string | string[]</td><td>high, mid, low, unblockable</td></tr>
                <tr><td><code>tags</code></td><td>string[]</td><td>Tags that must ALL be present (AND logic)</td></tr>
                <tr><td><code>input</code></td><td>string | string[]</td><td>Input notation with glob patterns</td></tr>
              </tbody>
            </table>
            <h4>Glob Patterns for Input</h4>
            <ul>
              <li><code>*</code> matches any sequence of characters (including empty)</li>
              <li><code>?</code> matches exactly one character</li>
            </ul>
            <p>Examples: <code>5*</code> matches 5L, 5M, 5H. <code>236*</code> matches 236P, 236K.</p>
          {:else if activeSection === "constraints"}
            <h3>Constraint Types</h3>
            <p>Constraints are used in validate rules to check move properties.</p>
            <h4><code>exists</code></h4>
            <p>Checks whether a field is set (non-null, non-empty, non-zero).</p>
            <pre><code>{`{ "animation": { "exists": true } }`}</code></pre>

            <h4><code>min</code> / <code>max</code></h4>
            <p>Numeric bounds (inclusive).</p>
            <pre><code>{`{ "startup": { "min": 1, "max": 30 } }`}</code></pre>

            <h4><code>equals</code></h4>
            <p>Exact value match.</p>
            <pre><code>{`{ "guard": { "equals": "mid" } }`}</code></pre>

            <h4><code>in</code></h4>
            <p>Value must be one of the specified options.</p>
            <pre><code>{`{ "guard": { "in": ["mid", "low"] } }`}</code></pre>
          {:else if activeSection === "builtin"}
            <h3>Built-in Validation</h3>
            <p>These validations <strong>always run</strong> and cannot be disabled.</p>
            <table>
              <thead>
                <tr><th>Field</th><th>Constraint</th><th>Error Message</th></tr>
              </thead>
              <tbody>
                <tr><td><code>startup</code></td><td>must be ≥ 1</td><td>startup must be at least 1 frame</td></tr>
                <tr><td><code>active</code></td><td>must be ≥ 1</td><td>active must be at least 1 frame</td></tr>
                <tr><td><code>input</code></td><td>must be non-empty</td><td>input cannot be empty</td></tr>
                <tr><td><code>hitboxes[i].frames</code></td><td>start ≤ end</td><td>start frame cannot be after end frame</td></tr>
                <tr><td><code>costs[i].amount</code></td><td>must be &gt; 0</td><td>cost amount must be greater than 0</td></tr>
                <tr><td><code>super_freeze.frames</code></td><td>must be &gt; 0</td><td>super_freeze frames must be greater than 0</td></tr>
                <tr><td><code>super_freeze.darken</code></td><td>0.0 to 1.0</td><td>darken must be between 0.0 and 1.0</td></tr>
              </tbody>
            </table>
          {:else if activeSection === "hierarchy"}
            <h3>Rule Hierarchy</h3>
            <p>Rules are merged from project and character levels:</p>
            <pre><code>{`Project: framesmith.rules.json
    │
    ├── apply rules (global defaults)
    └── validate rules (global constraints)
         │
         ▼
Character: characters/{id}/framesmith.rules.json
    │
    ├── apply rules (character-specific)
    └── validate rules (character-specific)`}</code></pre>
            <h4>Merge Behavior</h4>
            <ol>
              <li>Project rules are loaded first</li>
              <li>Character rules are loaded second</li>
              <li>If a character rule has the <strong>same match spec</strong> as a project rule, the character rule <strong>replaces</strong> it entirely</li>
              <li>Different match specs coexist (both apply)</li>
            </ol>
          {:else if activeSection === "examples"}
            <h3>Complete Examples</h3>
            <h4>Project-Level Rules (Sensible Defaults)</h4>
            <pre><code>{`{
  "version": 1,
  "apply": [
    {
      "match": { "type": "normal" },
      "set": {
        "hitstop": 8,
        "pushback": { "hit": 2, "block": 2 },
        "meter_gain": { "hit": 5, "whiff": 2 }
      }
    },
    {
      "match": { "button": "L" },
      "set": { "damage": 30 }
    },
    {
      "match": { "button": "M" },
      "set": { "damage": 50 }
    },
    {
      "match": { "button": "H" },
      "set": { "damage": 80 }
    }
  ],
  "validate": [
    {
      "match": {},
      "require": {
        "animation": { "exists": true }
      },
      "severity": "warning",
      "message": "Moves should have animation defined"
    }
  ]
}`}</code></pre>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .help-panel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 900px;
    max-width: 95vw;
    max-height: 85vh;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
  }

  .help-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .help-header h2 {
    font-size: 16px;
    font-weight: 600;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 20px;
    padding: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--text-primary);
    background: transparent;
  }

  .help-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .help-nav {
    width: 180px;
    border-right: 1px solid var(--border);
    padding: 8px;
    flex-shrink: 0;
    overflow-y: auto;
  }

  .nav-item {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    padding: 8px 12px;
    color: var(--text-secondary);
    font-size: 13px;
    border-radius: 4px;
    cursor: pointer;
  }

  .nav-item:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--bg-tertiary);
    color: var(--accent);
  }

  .help-content {
    flex: 1;
    padding: 20px;
    overflow-y: auto;
  }

  .help-content h3 {
    font-size: 18px;
    font-weight: 600;
    margin-bottom: 16px;
    color: var(--text-primary);
  }

  .help-content h4 {
    font-size: 14px;
    font-weight: 600;
    margin: 20px 0 12px 0;
    color: var(--text-primary);
  }

  .help-content p {
    margin-bottom: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
  }

  .help-content ul,
  .help-content ol {
    margin-bottom: 16px;
    padding-left: 24px;
    color: var(--text-secondary);
  }

  .help-content li {
    margin-bottom: 6px;
    line-height: 1.6;
  }

  .help-content code {
    background: var(--bg-primary);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: "JetBrains Mono", monospace;
    font-size: 12px;
    color: var(--accent);
  }

  .help-content pre {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 16px;
    overflow-x: auto;
  }

  .help-content pre code {
    background: transparent;
    padding: 0;
    font-size: 12px;
    color: var(--text-primary);
  }

  .help-content table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 16px;
    font-size: 13px;
  }

  .help-content th,
  .help-content td {
    text-align: left;
    padding: 8px 12px;
    border: 1px solid var(--border);
  }

  .help-content th {
    background: var(--bg-tertiary);
    color: var(--text-primary);
    font-weight: 600;
  }

  .help-content td {
    color: var(--text-secondary);
  }

  .help-content strong {
    color: var(--text-primary);
  }
</style>
