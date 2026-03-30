import { test, expect } from '../fixtures-cli';
import * as path from 'path';

/**
 * CLI error handling tests
 * Tests for invalid arguments, missing folders, and other error conditions
 */
test.describe('cli-errors', () => {
  test('ls on non-syncweb folder shows error', async ({ cli, createDummyDir, syncFolder }) => {
    createDummyDir('not-a-folder');
    const folderPath = path.join(syncFolder, 'not-a-folder');

    const result = cli.run(['ls'], { silent: true, cwd: folderPath });

    expect(result.exitCode).toBe(0); // Ls command itself might return 0 but print error to stdout/stderr
    expect(result.stdout).toContain('is not inside of a Syncweb folder');
  });

  test('create on existing syncweb folder handles it gracefully', async ({ cli, syncFolder }) => {
    // First create
    cli.run(['create', syncFolder], { silent: true });

    // Second create on same path
    const result = cli.run(['create', syncFolder], { silent: true });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('sync://'); // Should still output the URL
  });

  test('invalid flag returns error', async ({ cli }) => {
    const result = cli.run(['ls', '--non-existent-flag'], { silent: true });

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain('unknown flag');
  });

  test('missing required argument returns error', async ({ cli }) => {
    // join requires at least one URL
    const result = cli.run(['join'], { silent: true });
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain('expected');
  });
});
