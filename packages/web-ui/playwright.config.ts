import { defineConfig, devices } from '@playwright/test';

const isCloud = process.env.TEST_MODE === 'cloud';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  use: {
    baseURL: isCloud ? 'http://localhost:8080/' : 'http://localhost:5173/actualised-ai/',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: isCloud 
      ? 'pnpm run build:cloud && pnpm -C ../sdk exec tsx examples/server.ts' 
      : 'pnpm run dev',
    url: isCloud ? 'http://localhost:8080/' : 'http://localhost:5173/actualised-ai/',
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
