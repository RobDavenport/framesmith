import { expect, test, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import { isAbsolute, join } from 'node:path';

type JsonValue = unknown;

const projectPath = process.cwd();
const characterId = 'test_char';
const fixtureDir = join(projectPath, 'test-results', 'e2e');
const fspkPath = join(projectPath, 'test-results', 'e2e', `${characterId}.fspk`);
const jsonBlobPath = join(projectPath, 'test-results', 'e2e', `${characterId}.json`);
let fixtureCache: { characterData: CharacterData; fspkBase64: string } | null = null;

function readJson<T>(relativePath: string): T {
  const path = isAbsolute(relativePath) ? relativePath : join(projectPath, relativePath);
  return JSON.parse(readFileSync(path, 'utf8')) as T;
}

interface CharacterData {
  character: {
    id: string;
    name: string;
    properties: Record<string, JsonValue>;
  };
  moves: Array<Record<string, JsonValue>>;
  cancel_table: Record<string, JsonValue>;
}

function ensureExportFixtures(): { characterData: CharacterData; fspkBase64: string } {
  if (fixtureCache) {
    return fixtureCache;
  }

  mkdirSync(fixtureDir, { recursive: true });

  execFileSync(
    'cargo',
    [
      'run',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      '--bin',
      'framesmith-cli',
      '--',
      'export',
      '--project',
      '.',
      '--character',
      characterId,
      '--out',
      fspkPath,
    ],
    { cwd: projectPath, stdio: 'inherit' }
  );

  execFileSync(
    'cargo',
    [
      'run',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      '--bin',
      'framesmith-cli',
      '--',
      'export',
      '--project',
      '.',
      '--character',
      characterId,
      '--adapter',
      'json-blob',
      '--out',
      jsonBlobPath,
    ],
    { cwd: projectPath, stdio: 'inherit' }
  );

  fixtureCache = {
    characterData: readJson<CharacterData>(jsonBlobPath),
    fspkBase64: readFileSync(fspkPath).toString('base64'),
  };
  return fixtureCache;
}

async function installTauriMock(page: Page) {
  const { characterData, fspkBase64 } = ensureExportFixtures();
  const archetype = characterData.character.properties.archetype;

  await page.addInitScript(
    (payload: Record<string, unknown>) => {
      const { characterData, fspkBase64, projectPath, characterId, archetype } = payload as {
        characterData: CharacterData;
        fspkBase64: string;
        projectPath: string;
        characterId: string;
        archetype: unknown;
      };
      const currentCharacter = structuredClone(characterData);
      const dummyCharacter = structuredClone(characterData);
      dummyCharacter.character.id = 'dummy_char';
      dummyCharacter.character.name = 'Training Dummy';
      dummyCharacter.character.properties.health = 750;
      const globals = [
        {
          id: 'burst',
          name: 'Burst',
          type: 'system',
        },
      ];
      const burstState = {
        input: 'burst',
        name: 'Burst',
        type: 'system',
        tags: ['system'],
        startup: 1,
        active: 1,
        recovery: 0,
        damage: 0,
        hitstun: 0,
        blockstun: 0,
        hitstop: 0,
        guard: 'mid',
        hitboxes: [],
        hurtboxes: [],
        pushback: { hit: 0, block: 0 },
        meter_gain: { hit: 0, whiff: 0 },
        animation: 'burst',
      };

      (window as any).__framesmithLastSave = null;
      (window as any).__framesmithLastExport = null;

      const stateKey = (move: { input: string; id?: string }) => move.id ?? move.input;

      (window as any).__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          switch (cmd) {
            case 'open_folder_dialog':
              return projectPath;
            case 'validate_project':
              return { name: 'framesmith', path: projectPath, character_count: 2 };
            case 'list_characters':
              return [
                {
                  id: characterId,
                  name: currentCharacter.character.name,
                  archetype,
                  move_count: currentCharacter.moves.length,
                },
                {
                  id: 'dummy_char',
                  name: dummyCharacter.character.name,
                  archetype: 'training',
                  move_count: dummyCharacter.moves.length,
                },
              ];
            case 'load_character':
              return structuredClone(args.characterId === 'dummy_char' ? dummyCharacter : currentCharacter);
            case 'load_rules_registry':
              return {
                resources: ['heat', 'ammo', 'level', 'install_active'],
                move_types: {
                  types: ['system', 'normal', 'command_normal', 'special', 'super', 'movement', 'throw'],
                  filter_groups: {
                    normals: ['normal', 'command_normal'],
                    specials: ['special', 'ex', 'rekka'],
                    supers: ['super'],
                  },
                },
                chain_order: ['L', 'M', 'H'],
              };
            case 'load_character_assets':
              return { version: 1, textures: {}, models: {}, animations: {} };
            case 'read_character_asset_base64':
              return '';
            case 'save_move': {
              const mv = args.mv as { input: string; id?: string };
              if (mv.id && mv.id !== mv.input) {
                throw new Error('Resolved variant states are read-only via save_move');
              }
              const index = currentCharacter.moves.findIndex((move) => stateKey(move as { input: string; id?: string }) === stateKey(mv));
              if (index >= 0) currentCharacter.moves[index] = structuredClone(mv);
              (window as any).__framesmithLastSave = structuredClone(mv);
              return null;
            }
            case 'export_character':
              (window as any).__framesmithLastExport = structuredClone(args);
              return null;
            case 'get_character_fspk':
              return fspkBase64;
            case 'list_global_states':
              return structuredClone(globals);
            case 'get_global_state':
              return structuredClone(burstState);
            case 'save_global_state':
            case 'delete_global_state':
            case 'open_training_window':
              return null;
            default:
              throw new Error(`Unhandled mocked Tauri command: ${cmd}`);
          }
        },
        transformCallback: () => 1,
        unregisterCallback: () => null,
        runCallback: () => null,
        callbacks: new Map(),
        convertFileSrc: (filePath: string) => filePath,
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { label: 'main' },
        },
      };
    },
    { characterData, fspkBase64, projectPath, characterId, archetype } as Record<string, unknown>
  );
}

async function installMainWindowSyncResponder(page: Page) {
  const { characterData } = ensureExportFixtures();

  await page.evaluate(
    (payload: Record<string, unknown>) => {
      const { characterData, projectPath } = payload as {
        characterData: CharacterData;
        projectPath: string;
      };
      const channel = new BroadcastChannel('framesmith-training-sync');
      channel.onmessage = (event: MessageEvent) => {
        const message = event.data as { type?: string };
        if (message.type === 'request-sync') {
          channel.postMessage({
            type: 'character-change',
            character: structuredClone(characterData),
          });
          channel.postMessage({ type: 'project-path', path: projectPath });
        }
        if (message.type === 'ping') {
          channel.postMessage({ type: 'pong' });
        }
      };
      (window as any).__framesmithTrainingChannel = channel;
    },
    { characterData, projectPath } as Record<string, unknown>
  );
}

async function openMockedEditor(page: Page) {
  await installTauriMock(page);
  await page.goto('/');
}

test('loads the sample project and exercises core editor workflows', async ({ page }) => {
  await openMockedEditor(page);

  await page.getByRole('button', { name: 'Open...' }).click();

  await expect(page.locator('.project-name')).toHaveText('framesmith');
  await expect(page.getByRole('button', { name: /TEST_CHAR/ })).toBeVisible();

  await page.getByRole('button', { name: /TEST_CHAR/ }).click();
  await expect(page.getByRole('heading', { name: 'TEST_CHAR' })).toBeVisible();
  await expect(page.locator('.archetype-badge')).toHaveText('all-rounder');
  await expect(page.getByText('Tag Rules')).toBeVisible();

  await page.getByRole('button', { name: 'Frame Data' }).click();
  await expect(page.getByText('Standing Light')).toBeVisible();

  await page.getByRole('button', { name: 'State Editor' }).click();
  await page.getByLabel('Move:').selectOption('5L');
  await expect(page.getByLabel('Name')).toHaveValue('Standing Light');

  await page.getByLabel('Startup').fill('8');
  await page.getByRole('button', { name: /Pushboxes/ }).click();
  await page.getByRole('button', { name: '+ Add Pushbox' }).click();
  await page.getByRole('button', { name: 'Save Move' }).click();
  await expect(page.getByText('Saved!')).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => (window as any).__framesmithLastSave?.startup))
    .toBe(8);

  await page.getByRole('button', { name: 'Cancel Graph' }).click();
  await expect(page.getByText('Edge Types')).toBeVisible();
  await expect(page.getByText('5L')).toBeVisible();

  await page.getByRole('button', { name: 'Globals' }).click();
  await expect(page.getByText('Global States')).toBeVisible();
  await page.getByRole('option', { name: /burst/i }).click();
  await expect(page.getByLabel('Name')).toHaveValue('Burst');

  await page.getByRole('button', { name: 'Overview' }).click();
  await page.getByRole('button', { name: 'Export Character' }).click();
  await expect(page.getByText('Exported to exports/test_char.json')).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => (window as any).__framesmithLastExport?.adapter))
    .toBe('json-blob');
});

test('selects resolved variants by id and keeps them read-only in the editor', async ({ page }) => {
  await openMockedEditor(page);

  await page.getByRole('button', { name: 'Open...' }).click();
  await page.getByRole('button', { name: /TEST_CHAR/ }).click();
  await page.getByRole('button', { name: 'State Editor' }).click();

  await page.getByLabel('Move:').selectOption('5H~level1');

  await expect(page.getByLabel('Move:')).toHaveValue('5H~level1');
  await expect(page.getByText('Resolved variants are read-only here.')).toBeVisible();
  await expect(page.getByLabel('Input')).toHaveValue('5H');
  await expect(page.getByRole('button', { name: 'Save Move' })).toBeDisabled();
});

test('loads training mode from rebuilt WASM and exported FSPK data', async ({ page }) => {
  await openMockedEditor(page);

  await page.getByRole('button', { name: 'Open...' }).click();
  await page.getByRole('button', { name: /TEST_CHAR/ }).click();
  await page.getByRole('button', { name: 'Training', exact: true }).click();

  await expect(page.getByText('Initializing training mode...')).toBeVisible();
  await expect(page.getByText('Failed to initialize training mode')).not.toBeVisible({ timeout: 15_000 });
  await expect(page.getByText('P1', { exact: true })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText('CPU', { exact: true })).toBeVisible();

  const dummyCharacterSelect = page.getByLabel('Character', { exact: true });
  await expect(dummyCharacterSelect).toHaveValue(characterId);
  await dummyCharacterSelect.selectOption('dummy_char');
  await expect(page.getByText('Failed to initialize training mode')).not.toBeVisible({ timeout: 15_000 });
  await expect(dummyCharacterSelect).toHaveValue('dummy_char');
  await expect(page.getByText('CPU', { exact: true })).toBeVisible();
});

test('loads detached training mode through BroadcastChannel sync', async ({ page, context }) => {
  test.setTimeout(60_000);

  await installTauriMock(page);

  const mainPage = await context.newPage();
  await installTauriMock(mainPage);
  await mainPage.goto('/', { waitUntil: 'domcontentloaded' });
  await installMainWindowSyncResponder(mainPage);

  await page.goto(`/training?detached=true&character=${characterId}`, {
    waitUntil: 'domcontentloaded',
  });

  await expect(page.getByText('P1', { exact: true })).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText('CPU', { exact: true })).toBeVisible();
  await expect(page.getByText('Frame:')).toBeVisible();

  await mainPage.close();
});
