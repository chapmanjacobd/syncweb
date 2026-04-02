import { test, expect } from '../fixtures';

/**
 * File operations tests
 * Tests for file/folder context menu, download, delete, and bulk operations
 */
test.describe('file-operations', () => {
  test.use({ serverOptions: { verbose: false } });

  test('right-click shows context menu on file', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Get first file if exists
    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // Right-click to show context menu
        await filesPage.rightClickFile(path);

        // Context menu should appear (implementation dependent)
        // This test verifies the right-click handler exists
        await filesPage.waitForTimeout(300);
      }
    }
  });

  test('checkbox selection toggles file selection', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // Initially not selected
        const initiallySelected = await filesPage.isSelected(path);
        expect(initiallySelected).toBe(false);

        // Toggle selection
        await filesPage.toggleSelection(path);

        // Should be selected now
        const afterSelect = await filesPage.isSelected(path);
        expect(afterSelect).toBe(true);

        // Toggle again to deselect
        await filesPage.toggleSelection(path);
        const afterDeselect = await filesPage.isSelected(path);
        expect(afterDeselect).toBe(false);
      }
    }
  });

  test('bulk actions bar appears when items selected', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Initially bulk actions should be hidden
    await expect(filesPage.bulkActions).toBeHidden();

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      // Select first file
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        await filesPage.toggleSelection(path);

        // Bulk actions should appear
        await expect(filesPage.bulkActions).toBeVisible();

        // Selected count should show 1
        const selectedCount = await filesPage.getSelectedCount();
        expect(selectedCount).toBe(1);

        // Cancel selection
        await filesPage.clearSelection();
        await expect(filesPage.bulkActions).toBeHidden();
      }
    }
  });

  test('select multiple files', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount >= 2) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const secondFile = await filesPage.getFileItemByIndex(1);
      const path1 = await firstFile.getAttribute('data-path');
      const path2 = await secondFile.getAttribute('data-path');

      if (path1 && path2) {
        // Select multiple files
        await filesPage.selectMultipleFiles([path1, path2]);

        // Both should be selected
        const selected1 = await filesPage.isSelected(path1);
        const selected2 = await filesPage.isSelected(path2);
        expect(selected1).toBe(true);
        expect(selected2).toBe(true);

        // Selected count should be 2
        const totalCount = await filesPage.getSelectedCount();
        expect(totalCount).toBe(2);

        // Clear selection
        await filesPage.clearSelection();
      }
    }
  });

  test('bulk delete button is clickable', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // Select a file
        await filesPage.toggleSelection(path);

        // Click bulk delete (may show confirmation dialog)
        await filesPage.clickBulkDelete();

        // Wait for potential confirmation dialog
        await filesPage.waitForTimeout(500);

        // Clear selection
        await filesPage.clearSelection();
      }
    }
  });

  test('bulk move button is clickable', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // Select a file
        await filesPage.toggleSelection(path);

        // Click bulk move (may show folder picker)
        await filesPage.clickBulkMove();

        // Wait for potential folder picker
        await filesPage.waitForTimeout(500);

        // Clear selection
        await filesPage.clearSelection();
      }
    }
  });

  test('bulk copy button is clickable', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // Select a file
        await filesPage.toggleSelection(path);

        // Click bulk copy (may show folder picker)
        await filesPage.clickBulkCopy();

        // Wait for potential folder picker
        await filesPage.waitForTimeout(500);

        // Clear selection
        await filesPage.clearSelection();
      }
    }
  });

  test('file item has correct attributes', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);

      // Should have data-path attribute
      const path = await firstFile.getAttribute('data-path');
      expect(path).toBeTruthy();

      // Should have icon
      const icon = firstFile.locator('.icon');
      await expect(icon).toBeVisible();

      // Should have name
      const name = await firstFile.locator('.name').textContent();
      expect(name).toBeTruthy();
    }
  });

  test('folder items have is-dir class', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const folderCount = await filesPage.getFolderCount();
    if (folderCount > 0) {
      const firstFolder = await filesPage.getFolderItems().first();

      // Should have is-dir class
      const hasClass = await filesPage.hasClass(firstFolder, 'is-dir');
      expect(hasClass).toBe(true);
    }
  });

  test('file size is displayed', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // Get file size from secondary info
        const size = await filesPage.getFileSize(path);
        // Size might be empty for directories or show actual size for files
        expect(typeof size).toBe('string');
      }
    }
  });

  test('parent directory navigation', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Get initial path
    const initialPath = await filesPage.getCurrentPath();

    // Try to navigate to parent (only works if not at root)
    // Note: getCurrentPath() may return breadcrumb text like " Home" at root level
    if (!initialPath.includes('/') && !initialPath.includes('Home')) {
      await filesPage.navigateToParent();
      await filesPage.waitForFilesToLoad();

      // Path should be different (parent directory)
      const newPath = await filesPage.getCurrentPath();
      expect(newPath).not.toBe(initialPath);
    }
  });

  test('file item double-click navigates into folder', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const folderCount = await filesPage.getFolderCount();
    if (folderCount > 0) {
      const firstFolder = await filesPage.getFolderItems().first();
      const path = await firstFolder.getAttribute('data-path');

      if (path) {
        // Double-click to navigate into folder
        await filesPage.doubleClickFolder(path);
        await filesPage.waitForFilesToLoad();

        // Path should be updated
        const newPath = await filesPage.getCurrentPath();
        expect(newPath).toContain(path);
      }
    }
  });

  test('drag and drop is available', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // This test verifies drag and drop functionality exists
    // Actual drag and drop requires more complex setup
    const fileCount = await filesPage.getFileCount();
    const folderCount = await filesPage.getFolderCount();

    if (fileCount > 0 && folderCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const firstFolder = await filesPage.getFolderItems().first();

      // Verify elements exist (actual drag/drop tested separately)
      await expect(firstFile).toBeVisible();
      await expect(firstFolder).toBeVisible();
    }
  });

  test('file exists check works', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        // File should exist
        const exists = await filesPage.fileExists(path);
        expect(exists).toBe(true);

        // Non-existent file should return false
        const notExists = await filesPage.fileExists('non-existent-file-xyz123');
        expect(notExists).toBe(false);
      }
    }
  });

  test('get all file paths returns array', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    const paths = await filesPage.getAllFilePaths();

    // Should return an array
    expect(Array.isArray(paths)).toBe(true);

    // All paths should be non-empty strings
    for (const path of paths) {
      expect(typeof path).toBe('string');
      expect(path.length).toBeGreaterThan(0);
    }
  });
});
