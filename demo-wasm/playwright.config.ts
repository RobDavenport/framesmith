import { defineConfig } from '@playwright/test';

const external = process.env.FRAMESMITH_DEMO_URL;
export default defineConfig({
  testDir: './tests',
  outputDir: './test-results',
  timeout: 60_000,
  workers: 1,
  retries: 0,
  reporter: 'list',
  use: { baseURL: external || 'http://127.0.0.1:4179/framesmith/', viewport: { width: 1280, height: 720 }, trace: 'retain-on-failure', screenshot: 'only-on-failure' },
  webServer: external ? undefined : {
    command: 'python demo-wasm/serve.py --port 4179',
    cwd: '..',
    url: 'http://127.0.0.1:4179/framesmith/',
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
