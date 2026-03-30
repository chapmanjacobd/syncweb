import { test, expect } from '../fixtures-cli';

/**
 * CLI find command tests
 * Tests for file search functionality
 */
test.describe('cli-find', () => {
  test.beforeEach(async ({ cli, syncFolder }) => {
    // Initialize a Syncweb folder in the syncFolder directory
    cli.runAndVerify(['create', syncFolder], { silent: true });
  });

  test('find searches by filename', async ({ cli, createDummyFile, syncFolder }) => {
    // Create test files
    createDummyFile('apple.txt', 'content');
    createDummyFile('banana.txt', 'content');
    createDummyFile('cherry.txt', 'content');

    const result = cli.run(['find', 'apple'], { silent: true, cwd: syncFolder });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('apple.txt');
    expect(result.stdout).not.toContain('banana.txt');
  });

  test('find with json flag returns parseable output', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('search-test.txt', 'test content');

    const result = cli.run(['find', 'search-test', '--json'], { silent: true, cwd: syncFolder });
    expect(result.exitCode).toBe(0);
    const files = JSON.parse(result.stdout);

    // Should return an array
    expect(Array.isArray(files)).toBe(true);
    expect(files.length).toBeGreaterThan(0);
  });

  test('find with no results returns empty', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('unique-file.txt', 'content');

    const result = cli.run(['find', 'nonexistent'], { silent: true, cwd: syncFolder });

    expect(result.exitCode).toBe(0);
    // Should not contain any file paths
    expect(result.stdout).not.toMatch(/\.txt/);
  });

  test('find is case-insensitive', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('CaseSensitive.txt', 'content');

    const result = cli.run(['find', 'casesensitive'], { silent: true, cwd: syncFolder });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('CaseSensitive.txt');
  });

  test('find with extension filter', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('file1.txt', 'content');
    createDummyFile('file2.mp3', 'content');
    createDummyFile('file3.txt', 'content');

    const result = cli.run(['find', '--ext', 'txt'], { silent: true, cwd: syncFolder });

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('file1.txt');
    expect(result.stdout).toContain('file3.txt');
    expect(result.stdout).not.toContain('file2.mp3');
  });
});
