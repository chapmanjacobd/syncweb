import { defineConfig, devices } from '@playwright/test';
import path from 'path';

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,

  maxFailures: process.env.CI ? 100 : 10,

  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? '85%' : '35%',

  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: [
    ['dot'],
    ['json', { outputFile: path.join(__dirname, 'test-results', 'results.json') }]
  ],

  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: process.env.SYNCWEB_BASE_URL || 'http://localhost:8889',
    viewport: { width: 1280, height: 720 },

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',

    /* Screenshot on failure */
    screenshot: 'only-on-failure',

    /* Video on failure */
    video: 'retain-on-failure',

    /* Maximum time each action can take */
    actionTimeout: 60000,
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'desktop',
      use: {
       ...devices['Desktop Chrome'],
        launchOptions: {
          headless: true,
        },
      },
    },
  ],

  /* Folder for test artifacts such as screenshots, videos, traces, etc. */
  outputDir: path.join(__dirname, 'test-results'),

  /* Global setup and teardown */
  globalSetup: require.resolve('./global-setup'),
});
