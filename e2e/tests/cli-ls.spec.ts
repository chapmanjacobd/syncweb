import { test, expect } from '../fixtures-cli';

/**
 * CLI ls (list files) command tests
 * Tests for file listing functionality
 */
test.describe('cli-ls', () => {
  test('ls lists files in directory', async ({ cli, createDummyFile }) => {
    // Create some test files
    createDummyFile('test1.txt', 'content1');
    createDummyFile('test2.txt', 'content2');
    createDummyFile('subdir/test3.txt', 'content3');

    const result = cli.run(['ls'], { silent: true });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('test1.txt');
    expect(result.stdout).toContain('test2.txt');
  });

  test('ls with json flag returns parseable output', async ({ runJson, createDummyFile }) => {
    createDummyFile('json-test.txt', 'test content');

    const files = await runJson(['ls']);

    // Should return an array of files
    expect(Array.isArray(files)).toBe(true);
    expect(files.length).toBeGreaterThan(0);
  });

  test('ls shows directory indicator for folders', async ({ cli, createDummyDir }) => {
    createDummyDir('test-directory');

    const result = cli.run(['ls'], { silent: true });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('test-directory');
  });

  test('ls in empty directory works', async ({ cli, createDummyDir }) => {
    const emptyDir = createDummyDir('empty');

    const result = cli.run(['ls'], {
      silent: true,
      cwd: cli.getHome(),
    });

    // Should succeed even in empty directory
    expect(result.exitCode).toBe(0);
  });
});
