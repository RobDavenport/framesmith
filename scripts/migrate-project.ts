#!/usr/bin/env npx ts-node
/**
 * Framesmith Project Migration Script
 *
 * Migrates old project files to the new State/Tags format:
 * - State files: Converts `type` field to a tag in the `tags` array
 * - Cancel tables: Adds `tag_rules` and `deny` fields if missing
 *
 * Usage: npx ts-node scripts/migrate-project.ts /path/to/project
 *
 * This script is idempotent - safe to run multiple times.
 */

import * as fs from "fs";
import * as path from "path";

interface MigrationResult {
  filesModified: string[];
  filesSkipped: string[];
  errors: string[];
}

/**
 * Migrate a state (move) file.
 *
 * Converts the old `type` field (e.g., "normal", "special") to a tag
 * in the `tags` array if not already present.
 *
 * @param filePath - Absolute path to the state JSON file
 * @returns true if the file was modified, false otherwise
 */
function migrateStateFile(filePath: string): boolean {
  const content = fs.readFileSync(filePath, "utf-8");
  const state = JSON.parse(content);

  let modified = false;

  // Ensure tags array exists
  if (!Array.isArray(state.tags)) {
    state.tags = [];
    modified = true;
  }

  // Convert type field to a tag
  if (state.type && typeof state.type === "string") {
    const typeTag = state.type.toLowerCase().replace(/\s+/g, "_");

    // Only add if not already in tags
    if (!state.tags.includes(typeTag)) {
      state.tags.push(typeTag);
      modified = true;
    }

    // Remove the type field after migrating
    delete state.type;
    modified = true;
  }

  if (modified) {
    fs.writeFileSync(filePath, JSON.stringify(state, null, 2) + "\n", "utf-8");
  }

  return modified;
}

/**
 * Migrate a cancel table file.
 *
 * Adds `tag_rules` and `deny` fields if missing, preserving legacy fields
 * like `special_cancels`, `super_cancels`, and `jump_cancels`.
 *
 * @param filePath - Absolute path to the cancel_table.json file
 * @returns true if the file was modified, false otherwise
 */
function migrateCancelTable(filePath: string): boolean {
  const content = fs.readFileSync(filePath, "utf-8");
  const cancelTable = JSON.parse(content);

  let modified = false;

  // Add tag_rules if missing
  if (!Array.isArray(cancelTable.tag_rules)) {
    cancelTable.tag_rules = [];
    modified = true;
  }

  // Add deny if missing
  if (
    cancelTable.deny === undefined ||
    cancelTable.deny === null ||
    typeof cancelTable.deny !== "object" ||
    Array.isArray(cancelTable.deny)
  ) {
    cancelTable.deny = {};
    modified = true;
  }

  // Ensure chains exists as an object (it should already, but be safe)
  if (
    cancelTable.chains === undefined ||
    cancelTable.chains === null ||
    typeof cancelTable.chains !== "object" ||
    Array.isArray(cancelTable.chains)
  ) {
    cancelTable.chains = {};
    modified = true;
  }

  if (modified) {
    // Reorder fields for consistency: tag_rules, chains, deny, then legacy fields
    const orderedTable: Record<string, unknown> = {};

    orderedTable.tag_rules = cancelTable.tag_rules;
    orderedTable.chains = cancelTable.chains;
    orderedTable.deny = cancelTable.deny;

    // Preserve legacy fields
    if (
      Array.isArray(cancelTable.special_cancels) &&
      cancelTable.special_cancels.length > 0
    ) {
      orderedTable.special_cancels = cancelTable.special_cancels;
    }
    if (
      Array.isArray(cancelTable.super_cancels) &&
      cancelTable.super_cancels.length > 0
    ) {
      orderedTable.super_cancels = cancelTable.super_cancels;
    }
    if (
      Array.isArray(cancelTable.jump_cancels) &&
      cancelTable.jump_cancels.length > 0
    ) {
      orderedTable.jump_cancels = cancelTable.jump_cancels;
    }

    fs.writeFileSync(
      filePath,
      JSON.stringify(orderedTable, null, 2) + "\n",
      "utf-8"
    );
  }

  return modified;
}

/**
 * Migrate an entire Framesmith project.
 *
 * Iterates through all characters and their states (moves), applying migrations.
 *
 * @param projectPath - Absolute path to the project root
 * @returns MigrationResult with lists of modified files and errors
 */
function migrateProject(projectPath: string): MigrationResult {
  const result: MigrationResult = {
    filesModified: [],
    filesSkipped: [],
    errors: [],
  };

  // Check if project exists
  if (!fs.existsSync(projectPath)) {
    result.errors.push(`Project path does not exist: ${projectPath}`);
    return result;
  }

  // Look for characters directory
  const charactersDir = path.join(projectPath, "characters");
  if (!fs.existsSync(charactersDir)) {
    result.errors.push(`No characters directory found at: ${charactersDir}`);
    return result;
  }

  // Iterate through each character
  const characters = fs.readdirSync(charactersDir);
  for (const charName of characters) {
    const charPath = path.join(charactersDir, charName);

    // Skip non-directories
    if (!fs.statSync(charPath).isDirectory()) {
      continue;
    }

    console.log(`Processing character: ${charName}`);

    // Migrate cancel table if it exists
    const cancelTablePath = path.join(charPath, "cancel_table.json");
    if (fs.existsSync(cancelTablePath)) {
      try {
        if (migrateCancelTable(cancelTablePath)) {
          result.filesModified.push(cancelTablePath);
          console.log(`  Modified: cancel_table.json`);
        } else {
          result.filesSkipped.push(cancelTablePath);
        }
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Unknown error occurred";
        result.errors.push(`Error migrating ${cancelTablePath}: ${message}`);
      }
    }

    // Migrate state files in moves/ directory
    const movesDir = path.join(charPath, "moves");
    if (fs.existsSync(movesDir)) {
      const moveFiles = fs.readdirSync(movesDir);
      for (const moveFile of moveFiles) {
        if (!moveFile.endsWith(".json")) {
          continue;
        }

        const movePath = path.join(movesDir, moveFile);
        try {
          if (migrateStateFile(movePath)) {
            result.filesModified.push(movePath);
            console.log(`  Modified: moves/${moveFile}`);
          } else {
            result.filesSkipped.push(movePath);
          }
        } catch (err) {
          const message =
            err instanceof Error ? err.message : "Unknown error occurred";
          result.errors.push(`Error migrating ${movePath}: ${message}`);
        }
      }
    }
  }

  return result;
}

// CLI entry point
function main(): void {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error(
      "Usage: npx ts-node scripts/migrate-project.ts /path/to/project"
    );
    console.error("");
    console.error("Migrates a Framesmith project to the new State/Tags format:");
    console.error("  - Converts state type field to tags array");
    console.error("  - Adds tag_rules and deny fields to cancel tables");
    console.error("");
    console.error("This script is idempotent - safe to run multiple times.");
    process.exit(1);
  }

  const projectPath = path.resolve(args[0]);
  console.log(`Migrating project: ${projectPath}\n`);

  const result = migrateProject(projectPath);

  console.log("\n=== Migration Summary ===");
  console.log(`Files modified: ${result.filesModified.length}`);
  console.log(`Files skipped (already up to date): ${result.filesSkipped.length}`);
  console.log(`Errors: ${result.errors.length}`);

  if (result.errors.length > 0) {
    console.log("\nErrors:");
    for (const error of result.errors) {
      console.error(`  - ${error}`);
    }
    process.exit(1);
  }

  if (result.filesModified.length === 0) {
    console.log("\nNo changes needed - project is already up to date.");
  } else {
    console.log("\nMigration complete!");
  }
}

main();
