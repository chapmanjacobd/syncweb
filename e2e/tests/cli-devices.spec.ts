import { test, expect } from '../fixtures-cli';

/**
 * CLI devices command tests
 * Tests for device listing and management
 */
test.describe('cli-devices', () => {
  test('devices list returns device information', async ({ cli }) => {
    const result = cli.run(['devices'], { silent: true });

    // Command should succeed (even with no devices)
    expect(result.exitCode).toBe(0);
  });

  test('devices with json flag returns parseable output', async ({ runJson }) => {
    const devices = await runJson(['devices']);

    // Should return an array
    expect(Array.isArray(devices)).toBe(true);
  });
});
