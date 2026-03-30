import { test, expect } from '../fixtures-cli';

/**
 * CLI basic command tests
 * Tests for version, help, and basic CLI functionality
 */
test.describe('cli-basic', () => {
  test('version command returns version info', async ({ cli }) => {
    const result = cli.run(['version'], { silent: true });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('Version');
  });

  test('help command shows available commands', async ({ cli }) => {
    const result = cli.run(['--help'], { silent: true });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('Syncweb');
    expect(result.stdout).toContain('Commands:');
  });

  test('unknown command returns error', async ({ cli }) => {
    const result = cli.run(['unknown-command'], { silent: true });

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain('unexpected argument');
  });
});
