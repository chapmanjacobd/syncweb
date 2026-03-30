import { TestServer } from './utils/test-server';

/**
 * Global server instances shared across test workers
 * This allows for efficient parallel test execution
 */
export const globalServers = new Map<string, TestServer>();

/**
 * Kill any stale syncweb processes from previous runs
 */
async function cleanupStaleProcesses(): Promise<void> {
  try {
    const { execSync } = require('child_process');
    // Kill any syncweb serve processes
    execSync("pkill -f 'syncweb serve' 2>/dev/null || true", { stdio: 'ignore' });
    // Small delay to ensure ports are released
    await new Promise(resolve => setTimeout(resolve, 500));
  } catch (e) {
    // Ignore cleanup errors
  }
}

/**
 * Global setup function - runs once before all tests
 * Starts the syncweb server for E2E testing
 */
export default async function globalSetup() {
  // Clean up any stale processes before starting
  await cleanupStaleProcesses();
  
  // Servers are started on-demand in fixtures
  // This function can be used for one-time setup tasks

  // Store base URL in process.env for access in tests
  process.env.SYNCWEB_BASE_URL = process.env.SYNCWEB_BASE_URL || 'http://localhost:8889';

  // Return teardown function (optional - servers are stopped in fixtures)
  return async () => {
    // Clean up all global servers
    for (const [, server] of globalServers) {
      await server.stop();
    }
    globalServers.clear();
    
    // Ensure all syncweb processes are killed after tests
    await cleanupStaleProcesses();
  };
}
