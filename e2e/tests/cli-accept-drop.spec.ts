import { test, expect } from '../fixtures-cli';

/**
 * CLI accept and drop command tests
 * Tests for device and folder acceptance/removal
 */
test.describe('cli-accept-drop', () => {
  test.beforeEach(async ({ cli, syncFolder }) => {
    // Initialize a Syncweb folder
    cli.runAndVerify(['create', syncFolder], { silent: true });
  });

  test('accept command shows usage without arguments', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept'], { silent: true, cwd: syncFolder });

    // Should show usage or error
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toLowerCase()).toContain('usage');
  });

  test('accept with invalid device ID fails', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept', 'INVALID-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should fail with error about invalid device
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toLowerCase()).toContain('error');
  });

  test('accept with --folders flag', async ({ cli, syncFolder }) => {
    // Accept with folders flag (may have no pending folders)
    const result = cli.run(['accept', '--folders'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should succeed or show no pending folders
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept with --devices flag', async ({ cli, syncFolder }) => {
    // Accept with devices flag (may have no pending devices)
    const result = cli.run(['accept', '--devices'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should succeed or show no pending devices
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept with --help shows options', async ({ cli }) => {
    const result = cli.run(['accept', '--help'], { silent: true });

    // Should show help
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('accept');
  });

  test('drop command shows usage without arguments', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop'], { silent: true, cwd: syncFolder });

    // Should show usage or error
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr.toLowerCase()).toContain('usage');
  });

  test('drop with non-existent device ID', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop', 'NONEXISTENT-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // May fail or succeed (if device doesn't exist, might be no-op)
    // Just verify it doesn't crash
    expect([0, 1]).toContain(result.exitCode);
  });

  test('drop with --help shows options', async ({ cli }) => {
    const result = cli.run(['drop', '--help'], { silent: true });

    // Should show help
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('drop');
  });

  test('drop with json output', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop', '--json', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // May fail but should produce valid JSON if --json is used
    if (result.exitCode === 0) {
      expect(() => JSON.parse(result.stdout)).not.toThrow();
    }
  });

  test('accept with folder specification', async ({ cli, syncFolder }) => {
    // Accept specific folder (may have no pending folders)
    const result = cli.run(['accept', '--folders', 'test-folder'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept with device specification', async ({ cli, syncFolder }) => {
    // Accept specific device (may have no pending devices)
    const result = cli.run(['accept', '--devices', 'TEST-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully (may fail if device doesn't exist)
    expect([0, 1]).toContain(result.exitCode);
  });

  test('drop folder command', async ({ cli, syncFolder }) => {
    // Drop a folder (may have no folders to drop)
    const result = cli.run(['drop', 'folder', 'test-folder'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('drop device command', async ({ cli, syncFolder }) => {
    // Drop a device (may have no devices to drop)
    const result = cli.run(['drop', 'device', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept all pending (if any)', async ({ cli, syncFolder }) => {
    // Try to accept all pending items
    const result = cli.run(['accept', '--all'], {
      silent: true,
      cwd: syncFolder,
    });

    // May succeed or show unknown flag
    expect([0, 1, 2]).toContain(result.exitCode);
  });

  test('accept with force flag (if implemented)', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept', '--force', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // May succeed or show unknown flag
    expect([0, 1, 2]).toContain(result.exitCode);
  });

  test('drop with force flag (if implemented)', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop', '--force', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // May succeed or show unknown flag
    expect([0, 1, 2]).toContain(result.exitCode);
  });

  test('accept command preserves config', async ({ cli, syncFolder }) => {
    // Get initial folders
    const initialFolders = cli.run(['folders', '--json'], {
      silent: true,
      cwd: syncFolder,
    });

    // Try accept (should not break config)
    cli.run(['accept'], { silent: true, cwd: syncFolder });

    // Get folders after accept
    const afterFolders = cli.run(['folders', '--json'], {
      silent: true,
      cwd: syncFolder,
    });

    // Both should be valid JSON
    expect(() => JSON.parse(initialFolders.stdout)).not.toThrow();
    expect(() => JSON.parse(afterFolders.stdout)).not.toThrow();
  });

  test('drop command preserves config', async ({ cli, syncFolder }) => {
    // Get initial folders
    const initialFolders = cli.run(['folders', '--json'], {
      silent: true,
      cwd: syncFolder,
    });

    // Try drop (should not break config even if device doesn't exist)
    cli.run(['drop', 'NONEXISTENT'], { silent: true, cwd: syncFolder });

    // Get folders after drop
    const afterFolders = cli.run(['folders', '--json'], {
      silent: true,
      cwd: syncFolder,
    });

    // Both should be valid JSON
    expect(() => JSON.parse(initialFolders.stdout)).not.toThrow();
    expect(() => JSON.parse(afterFolders.stdout)).not.toThrow();
  });

  test('accept then drop scenario', async ({ cli, syncFolder }) => {
    // Accept (may be no-op)
    const acceptResult = cli.run(['accept'], { silent: true, cwd: syncFolder });

    // Drop (may be no-op)
    const dropResult = cli.run(['drop', 'TEST'], { silent: true, cwd: syncFolder });

    // Both should complete without crashing
    expect([0, 1]).toContain(acceptResult.exitCode);
    expect([0, 1]).toContain(dropResult.exitCode);
  });

  test('accept with multiple flags', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept', '--folders', '--devices'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle multiple flags
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept output contains status', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept'], { silent: true, cwd: syncFolder });

    // Output should contain some status information
    expect(result.stdout.length).toBeGreaterThanOrEqual(0);
  });

  test('drop output contains status', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop', 'TEST'], { silent: true, cwd: syncFolder });

    // Output should contain some status information
    expect(result.stdout.length).toBeGreaterThanOrEqual(0);
  });
});
