<script lang="ts">
  import type { PropertyValue } from '$lib/types';

  interface Props {
    /** The properties map to edit */
    properties: Record<string, PropertyValue>;
    /** Callback when properties change */
    onChange: (properties: Record<string, PropertyValue>) => void;
    /** Whether the editor is disabled */
    disabled?: boolean;
  }

  let { properties, onChange, disabled = false }: Props = $props();

  // State for the "add property" dialog
  let showAddDialog = $state(false);
  let newPropertyKey = $state('');
  let newPropertyType = $state<'number' | 'boolean' | 'string'>('number');
  let addError = $state<string | null>(null);

  /**
   * Detect the type of a property value
   */
  function getValueType(value: PropertyValue): 'number' | 'boolean' | 'string' {
    if (typeof value === 'number') return 'number';
    if (typeof value === 'boolean') return 'boolean';
    return 'string';
  }

  /**
   * Get the default value for a given type
   */
  function getDefaultValue(type: 'number' | 'boolean' | 'string'): PropertyValue {
    switch (type) {
      case 'number': return 0;
      case 'boolean': return false;
      case 'string': return '';
    }
  }

  /**
   * Format property key from snake_case to Title Case for display
   */
  function formatPropertyKey(key: string): string {
    return key
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  /**
   * Update a property value
   */
  function updateProperty(key: string, value: PropertyValue) {
    const updated = { ...properties, [key]: value };
    onChange(updated);
  }

  /**
   * Remove a property
   */
  function removeProperty(key: string) {
    const updated = { ...properties };
    delete updated[key];
    onChange(updated);
  }

  /**
   * Open the add property dialog
   */
  function openAddDialog() {
    newPropertyKey = '';
    newPropertyType = 'number';
    addError = null;
    showAddDialog = true;
  }

  /**
   * Close the add property dialog
   */
  function closeAddDialog() {
    showAddDialog = false;
    newPropertyKey = '';
    addError = null;
  }

  /**
   * Add a new property
   */
  function addProperty() {
    // Validate key
    const trimmedKey = newPropertyKey.trim();
    if (!trimmedKey) {
      addError = 'Property key is required';
      return;
    }

    // Convert to snake_case for consistency
    const snakeKey = trimmedKey.toLowerCase().replace(/\s+/g, '_');

    // Check for duplicates
    if (snakeKey in properties) {
      addError = `Property "${snakeKey}" already exists`;
      return;
    }

    // Add the property with default value
    const updated = { ...properties, [snakeKey]: getDefaultValue(newPropertyType) };
    onChange(updated);
    closeAddDialog();
  }

  /**
   * Handle number input change
   */
  function handleNumberChange(key: string, event: Event) {
    const input = event.target as HTMLInputElement;
    const value = parseFloat(input.value);
    if (!isNaN(value)) {
      updateProperty(key, value);
    }
  }

  /**
   * Handle boolean input change
   */
  function handleBooleanChange(key: string, event: Event) {
    const input = event.target as HTMLInputElement;
    updateProperty(key, input.checked);
  }

  /**
   * Handle string input change
   */
  function handleStringChange(key: string, event: Event) {
    const input = event.target as HTMLInputElement;
    updateProperty(key, input.value);
  }

  // Derive sorted property entries for consistent display
  const sortedEntries = $derived(
    Object.entries(properties).sort(([a], [b]) => a.localeCompare(b))
  );
</script>

<div class="property-editor">
  <div class="editor-header">
    <h4>Properties</h4>
    <button
      type="button"
      class="add-btn"
      onclick={openAddDialog}
      {disabled}
      aria-label="Add property"
    >
      + Add
    </button>
  </div>

  {#if sortedEntries.length === 0}
    <p class="empty-message">No properties defined. Click "Add" to create one.</p>
  {:else}
    <div class="property-list" role="list">
      {#each sortedEntries as [key, value] (key)}
        {@const valueType = getValueType(value)}
        <div class="property-row" role="listitem">
          <label class="property-label" for="prop-{key}">
            {formatPropertyKey(key)}
          </label>
          <div class="property-input">
            {#if valueType === 'number'}
              <input
                id="prop-{key}"
                type="number"
                value={value as number}
                oninput={(e) => handleNumberChange(key, e)}
                {disabled}
                step="any"
              />
            {:else if valueType === 'boolean'}
              <label class="checkbox-wrapper">
                <input
                  id="prop-{key}"
                  type="checkbox"
                  checked={value as boolean}
                  onchange={(e) => handleBooleanChange(key, e)}
                  {disabled}
                />
                <span class="checkbox-label">{value ? 'Yes' : 'No'}</span>
              </label>
            {:else}
              <input
                id="prop-{key}"
                type="text"
                value={value as string}
                oninput={(e) => handleStringChange(key, e)}
                {disabled}
              />
            {/if}
          </div>
          <button
            type="button"
            class="remove-btn"
            onclick={() => removeProperty(key)}
            {disabled}
            aria-label="Remove {formatPropertyKey(key)}"
          >
            Remove
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if showAddDialog}
    <div class="add-dialog-overlay" role="dialog" aria-modal="true" aria-labelledby="add-dialog-title">
      <div class="add-dialog">
        <h5 id="add-dialog-title">Add Property</h5>

        {#if addError}
          <div class="error-message" role="alert">{addError}</div>
        {/if}

        <div class="dialog-field">
          <label for="new-prop-key">Property Key</label>
          <input
            id="new-prop-key"
            type="text"
            bind:value={newPropertyKey}
            placeholder="e.g., walk_speed"
            onkeydown={(e) => e.key === 'Enter' && addProperty()}
          />
        </div>

        <div class="dialog-field">
          <label for="new-prop-type">Type</label>
          <select id="new-prop-type" bind:value={newPropertyType}>
            <option value="number">Number</option>
            <option value="boolean">Boolean</option>
            <option value="string">String</option>
          </select>
        </div>

        <div class="dialog-actions">
          <button type="button" class="cancel-btn" onclick={closeAddDialog}>
            Cancel
          </button>
          <button type="button" class="confirm-btn" onclick={addProperty}>
            Add
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .property-editor {
    border: 1px solid var(--border, #2a2a4a);
    border-radius: 8px;
    background: var(--bg-secondary, #16213e);
    overflow: hidden;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border, #2a2a4a);
    background: var(--bg-tertiary, #0f3460);
  }

  .editor-header h4 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary, #a0a0a0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .add-btn {
    padding: 4px 12px;
    font-size: 12px;
    background: var(--accent, #e94560);
    border: none;
    color: white;
  }

  .add-btn:hover:not(:disabled) {
    background: var(--accent-hover, #ff6b6b);
  }

  .empty-message {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-secondary, #a0a0a0);
    font-size: 13px;
  }

  .property-list {
    display: flex;
    flex-direction: column;
  }

  .property-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border, #2a2a4a);
  }

  .property-row:last-child {
    border-bottom: none;
  }

  .property-label {
    flex: 0 0 140px;
    font-size: 13px;
    color: var(--text-secondary, #a0a0a0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .property-input {
    flex: 1;
    min-width: 0;
  }

  .property-input input[type="number"],
  .property-input input[type="text"] {
    width: 100%;
    padding: 6px 10px;
    font-variant-numeric: tabular-nums;
  }

  .checkbox-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .checkbox-wrapper input[type="checkbox"] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  .checkbox-label {
    font-size: 13px;
    color: var(--text-primary, #eaeaea);
  }

  .remove-btn {
    flex-shrink: 0;
    padding: 4px 10px;
    font-size: 12px;
    background: transparent;
    border: 1px solid var(--accent, #e94560);
    color: var(--accent, #e94560);
  }

  .remove-btn:hover:not(:disabled) {
    background: var(--accent, #e94560);
    color: white;
  }

  /* Add Dialog */
  .add-dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .add-dialog {
    background: var(--bg-secondary, #16213e);
    border: 1px solid var(--border, #2a2a4a);
    border-radius: 8px;
    padding: 20px;
    width: 320px;
    max-width: 90vw;
  }

  .add-dialog h5 {
    margin: 0 0 16px;
    font-size: 16px;
    font-weight: 600;
  }

  .error-message {
    padding: 8px 12px;
    margin-bottom: 12px;
    background: rgba(233, 69, 96, 0.1);
    border: 1px solid var(--accent, #e94560);
    border-radius: 4px;
    color: var(--accent, #e94560);
    font-size: 13px;
  }

  .dialog-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 16px;
  }

  .dialog-field label {
    font-size: 13px;
    color: var(--text-secondary, #a0a0a0);
  }

  .dialog-field input,
  .dialog-field select {
    width: 100%;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--border, #2a2a4a);
  }

  .cancel-btn:hover {
    background: var(--bg-tertiary, #0f3460);
    border-color: var(--border, #2a2a4a);
  }

  .confirm-btn {
    background: var(--accent, #e94560);
    border: none;
    color: white;
  }

  .confirm-btn:hover {
    background: var(--accent-hover, #ff6b6b);
  }
</style>
