import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for ToM Protocol E2E testing
 *
 * Tests multi-browser scenarios:
 * - Group creation with 3 users
 * - Relay disconnect scenarios
 * - Invitation flow validation
 */
export default defineConfig({
  testDir: './tests',

  // Run tests in parallel across browsers
  fullyParallel: false, // Sequential for multi-user coordination

  // Fail the build on CI if you accidentally left test.only in the source code
  forbidOnly: !!process.env.CI,

  // Retry on CI only
  retries: process.env.CI ? 2 : 0,

  // Limit workers for controlled multi-user scenarios
  workers: 1,

  // Reporter configuration
  reporter: [
    ['html', { outputFolder: './reports/html' }],
    ['json', { outputFile: './reports/results.json' }],
    ['list'],
  ],

  // Shared settings for all projects
  use: {
    // Base URL for the demo app
    baseURL: process.env.DEMO_URL || 'http://localhost:5173',

    // Collect trace on failure
    trace: 'on-first-retry',

    // Screenshot on failure
    screenshot: 'only-on-failure',

    // Video recording for debugging
    video: 'on-first-retry',
  },

  // Configure projects for major browsers
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    // Mobile viewports for responsive testing
    {
      name: 'mobile-chrome',
      use: { ...devices['Pixel 5'] },
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 12'] },
    },
  ],

  // Local dev server configuration
  webServer: {
    // E2E tests require BOTH the demo (5173) and the signaling server (3001).
    // Start the full local demo stack so the UI can transition from #login → #chat.
    command: './scripts/start-demo.sh',
    url: 'http://localhost:5173',
    // Keep deterministic environment even on local runs.
    // start-demo.sh is idempotent and will reuse running ports if already up.
    reuseExistingServer: false,
    timeout: 120000,
    cwd: '../../', // Root of monorepo
  },

  // Global timeout settings
  timeout: 60000, // 60s per test (groups need time)
  expect: {
    timeout: 10000, // 10s for assertions
  },

  // Output directory for test artifacts
  outputDir: './reports/artifacts',
});
