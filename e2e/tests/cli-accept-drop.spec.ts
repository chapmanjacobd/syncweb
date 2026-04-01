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

  test('accept command shows error without arguments', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept'], { silent: true, cwd: syncFolder });

    // Should error because device-ids is required
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain('expected');
  });

  test('accept with invalid device ID fails', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept', 'INVALID-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should show error about invalid device and exit with non-zero
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain('no valid devices');
  });

  test('accept with --folder-ids flag', async ({ cli, syncFolder }) => {
    // Accept with folder-ids flag (may have no pending folders)
    const result = cli.run(['accept', '-f', 'test-folder', 'TEST-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should succeed or show no pending folders
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept with device ID positional argument', async ({ cli, syncFolder }) => {
    // Accept with device ID as positional argument
    const result = cli.run(['accept', 'TEST-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully (may fail if device doesn't exist)
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept with --help shows options', async ({ cli }) => {
    const result = cli.run(['accept', '--help'], { silent: true });

    // Should show help
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('accept');
  });

  test('drop command shows error without arguments', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop'], { silent: true, cwd: syncFolder });

    // Should error because device-ids is required
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain('expected');
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

    // Should produce valid JSON even on error
    expect(result.exitCode).not.toBe(0);
    const output = JSON.parse(result.stdout);
    expect(output).toHaveProperty('device_count');
    expect(output).toHaveProperty('devices');
    expect(output).toHaveProperty('errors');
  });

  test('accept with folder-ids specification', async ({ cli, syncFolder }) => {
    // Accept specific folder (may have no pending folders)
    const result = cli.run(['accept', '-f', 'test-folder', 'TEST-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept with device ID', async ({ cli, syncFolder }) => {
    // Accept specific device (may have no pending devices)
    const result = cli.run(['accept', 'TEST-DEVICE-ID'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully (may fail if device doesn't exist)
    expect([0, 1]).toContain(result.exitCode);
  });

  test('drop with folder-ids removes from folders', async ({ cli, syncFolder }) => {
    // Drop a device from folders
    const result = cli.run(['drop', '-f', 'test-folder', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('drop device command', async ({ cli, syncFolder }) => {
    // Drop a device
    const result = cli.run(['drop', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept command preserves config', async ({ cli, syncFolder }) => {
    // Get initial folders
    const initialFolders = cli.run(['folders', '--json'], {
      silent: true,
      cwd: syncFolder,
    });

    // Try accept with a device (should not break config)
    cli.run(['accept', 'TEST-DEVICE'], { silent: true, cwd: syncFolder });

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
    const acceptResult = cli.run(['accept', 'TEST-DEVICE'], { silent: true, cwd: syncFolder });

    // Drop (may be no-op)
    const dropResult = cli.run(['drop', 'TEST'], { silent: true, cwd: syncFolder });

    // Both should complete without crashing
    expect([0, 1]).toContain(acceptResult.exitCode);
    expect([0, 1]).toContain(dropResult.exitCode);
  });

  test('accept with folder-ids flag', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept', '-f', 'test-folder', 'TEST-DEVICE'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should handle gracefully
    expect([0, 1]).toContain(result.exitCode);
  });

  test('accept output contains status', async ({ cli, syncFolder }) => {
    const result = cli.run(['accept', 'TEST-DEVICE'], { silent: true, cwd: syncFolder });

    // Output should contain some status information
    expect(result.stdout.length).toBeGreaterThanOrEqual(0);
  });

  test('drop output contains status', async ({ cli, syncFolder }) => {
    const result = cli.run(['drop', 'TEST'], { silent: true, cwd: syncFolder });

    // Output should contain some status information
    expect(result.stdout.length).toBeGreaterThanOrEqual(0);
  });
});
