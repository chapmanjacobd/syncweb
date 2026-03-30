import { test as base, expect } from '@playwright/test';
import { TestServer, TestServerOptions } from './utils/test-server';
import { globalServers } from './global-setup';
import { SidebarPage } from './pages/sidebar-page';
import { FilesPage } from './pages/files-page';
import { CompletionPage } from './pages/completion-page';

/**
 * Test fixtures for Syncweb E2E tests
 * Extends Playwright's test with custom fixtures for server management and page objects
 */
export const test = base.extend<{
  server: TestServer;
  serverOptions: TestServerOptions;
  sidebarPage: SidebarPage;
  filesPage: FilesPage;
  completionPage: CompletionPage;
}>({
  serverOptions: async ({}, use) => {
    await use({});
  },

  server: async ({ page, serverOptions }, use) => {
    const workerId = process.env.TEST_WORKER_INDEX || 'default';
    const project = process.env.PLAYWRIGHT_PROJECT || 'desktop';
    const serverKey = `${project}-${workerId}`;

    let server: TestServer;

    // Use global server for parallel test efficiency
    if (!globalServers.has(serverKey)) {
      server = new TestServer(serverOptions);
      await server.start();
      globalServers.set(serverKey, server);
    } else {
      server = globalServers.get(serverKey)!;
    }

    // Set base URL for tests
    process.env.SYNCWEB_BASE_URL = server.getBaseUrl();

    // Set up cookie for authentication if needed
    const url = new URL(server.getBaseUrl());
    await page.context().addCookies([{
      name: 'syncweb_token',
      value: 'e2e-test-token',
      domain: url.hostname,
      path: '/',
    }]);

    // Optional: Log console errors during tests
    if (process.env.DEBUG) {
      page.on('console', msg => {
        const msgText = msg.text();
        if (msg.type() === 'error') {
          console.error(`console.error:`, msgText);
        }
      });

      page.on('requestfailed', request => {
        const errorText = request.failure()?.errorText;
        if (errorText && !['net::ERR_ABORTED'].includes(errorText)) {
          console.error(`request.failed: ${request.url()} - ${errorText}`);
        }
      });

      page.on('response', response => {
        if (response.status() >= 400) {
          console.error(`response.error: ${response.url()} - status ${response.status()}`);
        }
      });
    }

    await use(server);

    // Server lifecycle is managed globally for efficiency
    // Individual tests should not stop the server
  },

  sidebarPage: async ({ page }, use) => {
    await use(new SidebarPage(page));
  },

  filesPage: async ({ page }, use) => {
    await use(new FilesPage(page));
  },

  completionPage: async ({ page }, use) => {
    await use(new CompletionPage(page));
  },
});

export { expect };
