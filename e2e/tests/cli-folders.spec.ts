import { test, expect } from '../fixtures-cli';

/**
 * CLI folders command tests
 * Tests for folder listing and management
 */
test.describe('cli-folders', () => {
  test('folders list returns folder information', async ({ cli, runJson }) => {
    // List folders (may be empty initially)
    const result = cli.run(['folders'], { silent: true });

    // Command should succeed (even with no folders)
    expect(result.exitCode).toBe(0);
  });

  test('folders with json flag returns parseable output', async ({ runJson }) => {
    const folders = await runJson(['folders']);

    // Should return an array
    expect(Array.isArray(folders)).toBe(true);
  });
});
