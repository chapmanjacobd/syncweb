import { TestServer } from './utils/test-server';

/**
 * Global server instances shared across test workers
 * This allows for efficient parallel test execution
 */
export const globalServers = new Map<string, TestServer>();

/**
 * Global setup function - runs once before all tests
 * Starts the syncweb server for E2E testing
 */
export default async function globalSetup() {
  // Servers are started on-demand in fixtures
  // This function can be used for one-time setup tasks

  // Store base URL in process.env for access in tests
  const baseUrl = process.env.SYNCWEB_BASE_URL || 'http://localhost:8889';

  // Return teardown function (optional - servers are stopped in fixtures)
  return async () => {
    // Clean up all global servers
    for (const [, server] of globalServers) {
      await server.stop();
    }
    globalServers.clear();
  };
}
