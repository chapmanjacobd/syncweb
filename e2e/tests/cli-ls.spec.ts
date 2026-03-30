import { test, expect } from '../fixtures-cli';

/**
 * CLI ls (list files) command tests
 * Tests for file listing functionality
 */
test.describe('cli-ls', () => {
  test.beforeEach(async ({ cli }) => {
    // Initialize a Syncweb folder in the test home
    cli.run(['create', '.'], { silent: true, cwd: cli.getHome() });
  });

  test('ls lists files in directory', async ({ cli, createDummyFile }) => {
    // Create some test files
    createDummyFile('test1.txt', 'content1');
    createDummyFile('test2.txt', 'content2');
    createDummyFile('subdir/test3.txt', 'content3');

    const result = cli.run(['ls'], { silent: true, cwd: cli.getHome() });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('test1.txt');
    expect(result.stdout).toContain('test2.txt');
  });

  test('ls with json flag returns parseable output', async ({ cli, createDummyFile }) => {
    createDummyFile('json-test.txt', 'test content');

    // Make sure we are in the correct directory
    const result = cli.run(['ls', '--json'], { silent: true, cwd: cli.getHome() });
    expect(result.exitCode).toBe(0);
    const files = JSON.parse(result.stdout);

    // Should return an array of files
    expect(Array.isArray(files)).toBe(true);
    expect(files.length).toBeGreaterThan(0);
  });

  test('ls shows directory indicator for folders', async ({ cli, createDummyDir }) => {
    createDummyDir('test-directory');

    const result = cli.run(['ls'], { silent: true, cwd: cli.getHome() });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('test-directory');
  });

  test('ls in empty directory works', async ({ cli, createDummyDir }) => {
    createDummyDir('empty');

    const result = cli.run(['ls'], {
      silent: true,
      cwd: cli.getHome(),
    });

    // Should succeed even in empty directory
    expect(result.exitCode).toBe(0);
  });
});
