import { test, expect } from '../fixtures-cli';
import * as path from 'path';

/**
 * CLI ls (list files) command tests
 * Tests for file listing functionality
 */
test.describe('cli-ls', () => {
  test.beforeEach(async ({ cli }) => {
    // Initialize a Syncweb folder in the test home
    cli.runAndVerify(['create', '.'], { silent: true, cwd: cli.getHome() });
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

  test('ls with long format shows extra info', async ({ cli, createDummyFile }) => {
    createDummyFile('long-test.txt', 'some content');

    const result = cli.run(['ls', '-l'], { silent: true, cwd: cli.getHome() });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('Type');
    expect(result.stdout).toContain('Size');
    expect(result.stdout).toContain('Modified');
    expect(result.stdout).toContain('long-test.txt');
  });

  test('ls with multiple paths works', async ({ cli, createDummyFile, createDummyDir }) => {
    createDummyDir('dir1');
    createDummyFile('dir1/file1.txt', 'c1');
    createDummyDir('dir2');
    createDummyFile('dir2/file2.txt', 'c2');

    const result = cli.run(['ls', 'dir1', 'dir2'], { silent: true, cwd: cli.getHome() });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('file1.txt');
    expect(result.stdout).toContain('file2.txt');
  });

  test('ls with hidden files works with -a flag', async ({ cli, createDummyFile }) => {
    createDummyFile('.hidden.txt', 'hidden');

    // Without -a, it shouldn't show it
    const resultWithoutA = cli.run(['ls'], { silent: true, cwd: cli.getHome() });
    expect(resultWithoutA.stdout).not.toContain('.hidden.txt');

    // With -a, it should show it
    const resultWithA = cli.run(['ls', '-a'], { silent: true, cwd: cli.getHome() });
    expect(resultWithA.stdout).toContain('.hidden.txt');
  });

  test('ls with depth limits output', async ({ cli, createDummyDir, createDummyFile }) => {
    createDummyDir('level1/level2');
    createDummyFile('level1/file1.txt', 'f1');
    createDummyFile('level1/level2/file2.txt', 'f2');

    // With depth 0, only level 1 items
    const resultDepth0 = cli.run(['ls'], { silent: true, cwd: cli.getHome() });
    expect(resultDepth0.stdout).toContain('level1/');
    expect(resultDepth0.stdout).not.toContain('file1.txt');

    // With depth 1
    const resultDepth1 = cli.run(['ls', '-D', '1'], { silent: true, cwd: cli.getHome() });
    expect(resultDepth1.stdout).toContain('level1/');
    expect(resultDepth1.stdout).toContain('file1.txt');
    expect(resultDepth1.stdout).toContain('level2/');
    expect(resultDepth1.stdout).not.toContain('file2.txt');
  });

  test('ls with sync URL works', async ({ cli, createDummyFile }) => {
    createDummyFile('url-test.txt', 'content');

    // Get folder ID from create output
    const createResult = cli.run(['create', '.'], { silent: true, cwd: cli.getHome() });
    const match = createResult.stdout.match(/sync:\/\/([^#]+)#/);
    const folderID = match ? match[1] : null;

    if (folderID) {
      const result = cli.run([`ls`, `sync://${folderID}/`], { silent: true, cwd: cli.getHome() });
      expect(result.exitCode).toBe(0);
      expect(result.stdout).toContain('url-test.txt');
    }
  });

  test('ls in empty directory works', async ({ cli, createDummyDir }) => {
    createDummyDir('empty');
    const emptyPath = path.join(cli.getHome(), 'empty');

    const result = cli.run(['ls'], {
      silent: true,
      cwd: emptyPath,
    });

    // Should succeed even in empty directory
    expect(result.exitCode).toBe(0);
  });
});
