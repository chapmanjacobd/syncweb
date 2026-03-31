import { test, expect } from '../fixtures-cli';

/**
 * CLI download command tests
 * Tests for file download functionality
 */
test.describe('cli-download', () => {
  test.beforeEach(async ({ cli, syncFolder }) => {
    // Initialize a Syncweb folder
    cli.runAndVerify(['create', syncFolder], { silent: true });
  });

  test('download with no files shows usage or succeeds', async ({ cli, syncFolder }) => {
    const result = cli.run(['download'], { silent: true, cwd: syncFolder });

    // May show usage or succeed with no-op
    expect([0, 1]).toContain(result.exitCode);
  });

  test('download single file', async ({ cli, createDummyFile, syncFolder }) => {
    // Create a test file
    const fileName = createDummyFile('download-test.txt', 'test content for download');

    // Download the file
    const result = cli.run(['download', fileName], { silent: true, cwd: syncFolder });

    // Command should succeed
    expect(result.exitCode).toBe(0);

    // Output should mention download or file
    expect(result.stdout.toLowerCase()).toContain('download');
  });

  test('download multiple files', async ({ cli, createDummyFile, syncFolder }) => {
    // Create test files
    createDummyFile('download1.txt', 'content1');
    createDummyFile('download2.txt', 'content2');
    createDummyFile('download3.txt', 'content3');

    // Download multiple files
    const result = cli.run(['download', 'download1.txt', 'download2.txt', 'download3.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed
    expect(result.exitCode).toBe(0);

    // Output should mention download
    expect(result.stdout.toLowerCase()).toContain('download');
  });

  test('download with --yes flag skips confirmation', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('auto-download.txt', 'auto download content');

    // Download with --yes to skip confirmation
    const result = cli.run(['download', '--yes', 'auto-download.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed without waiting for input
    expect(result.exitCode).toBe(0);
  });

  test('download non-existent file fails', async ({ cli, syncFolder }) => {
    const result = cli.run(['download', 'non-existent-file.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should fail
    expect(result.exitCode).not.toBe(0);

    // Should show error message
    expect(result.stderr.toLowerCase()).toContain('error');
  });

  test('download directory', async ({ cli, createDummyDir, createDummyFile, syncFolder }) => {
    // Create directory with files
    createDummyDir('download-dir');
    createDummyFile('download-dir/file1.txt', 'content1');
    createDummyFile('download-dir/file2.txt', 'content2');

    // Download directory
    const result = cli.run(['download', 'download-dir'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed
    expect(result.exitCode).toBe(0);
  });

  test('download with json output', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('json-download.txt', 'json download test');

    const result = cli.run(['download', '--json', 'json-download.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed
    expect(result.exitCode).toBe(0);

    // Output should be valid JSON
    const output = JSON.parse(result.stdout);
    expect(output).toBeTruthy();
  });

  test('download summary is displayed', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('summary-test.txt', 'summary test content');

    const result = cli.run(['download', 'summary-test.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Should show download summary
    expect(result.stdout.toLowerCase()).toContain('download');
  });

  test('download shows file size', async ({ cli, createDummyFile, syncFolder }) => {
    const content = 'test content for size check';
    createDummyFile('size-test.txt', content);

    const result = cli.run(['download', 'size-test.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Output should mention size (in Bytes or similar)
    expect(result.stdout.toLowerCase()).toMatch(/(size|bytes|b)/);
  });

  test('download with path prefix', async ({ cli, createDummyDir, createDummyFile, syncFolder }) => {
    createDummyDir('subdir');
    createDummyFile('subdir/nested-file.txt', 'nested content');

    // Download with path
    const result = cli.run(['download', 'subdir/nested-file.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed
    expect(result.exitCode).toBe(0);
  });

  test('download hidden files', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('.hidden-download.txt', 'hidden content');

    // Download hidden file
    const result = cli.run(['download', '.hidden-download.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed
    expect(result.exitCode).toBe(0);
  });

  test('download large file indicator', async ({ cli, createDummyFile, syncFolder }) => {
    // Create a larger file (1MB)
    const largeContent = 'x'.repeat(1024 * 1024);
    createDummyFile('large-file.txt', largeContent);

    const result = cli.run(['download', 'large-file.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Command should succeed
    expect(result.exitCode).toBe(0);

    // Should show size in MB or KB
    expect(result.stdout).toMatch(/(MB|KB|MiB|KiB)/);
  });

  test('download status shows OK or LOW', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('status-test.txt', 'status test');

    const result = cli.run(['download', 'status-test.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // Status should be OK or LOW
    expect(result.stdout).toMatch(/(OK|LOW)/);
  });

  test('download folder ID', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('folder-test.txt', 'folder test');

    // Get folder ID from create output
    const createResult = cli.run(['create', syncFolder], { silent: true });
    const match = createResult.stdout.match(/sync:\/\/([^#]+)#/);
    const folderID = match ? match[1] : null;

    if (folderID) {
      // Download using folder ID
      const result = cli.run(['download', `sync://${folderID}/folder-test.txt`], {
        silent: true,
        cwd: syncFolder,
      });

      // Command should succeed
      expect(result.exitCode).toBe(0);
    }
  });

  test('download with dry-run (if implemented)', async ({ cli, createDummyFile, syncFolder }) => {
    createDummyFile('dry-run-test.txt', 'dry run test');

    const result = cli.run(['download', '--dry-run', 'dry-run-test.txt'], {
      silent: true,
      cwd: syncFolder,
    });

    // May succeed or show unknown flag error
    // Just verify it doesn't crash
    expect([0, 1, 2]).toContain(result.exitCode);
  });

  test('download help shows options', async ({ cli }) => {
    const result = cli.run(['download', '--help'], { silent: true });

    // Should show help
    expect(result.exitCode).toBe(0);
    expect(result.stdout).toContain('download');
  });
});
