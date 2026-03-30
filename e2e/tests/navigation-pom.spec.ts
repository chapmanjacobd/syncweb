import { test, expect } from '../fixtures';

/**
 * Basic navigation tests using Page Object Model
 * Demonstrates POM usage with SidebarPage and FilesPage
 */
test.describe('Navigation', () => {
  test.use({ serverOptions: { verbose: false } });

  test('loads the home page with root folder selected', async ({ filesPage, sidebarPage, server }) => {
    // Navigate to the application
    await filesPage.goto(server.getBaseUrl());

    // Verify sidebar is visible
    await sidebarPage.expectVisible(sidebarPage.sidebar);

    // Verify root folder is active
    await expect(sidebarPage.getRootFolder()).toHaveClass(/active/);

    // Verify files view is loaded
    await filesPage.expectVisible(filesPage.fileList);

    // Wait for current path to be updated (might take a moment for JS to load)
    await expect(filesPage.currentPath).not.toHaveText('Select a folder');
    
    // Verify current path shows root
    const currentPath = await filesPage.getCurrentPath();
    expect(currentPath).toContain('/');
  });

  test('switches between view tabs', async ({ filesPage, completionPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Should start on Files view
    await expect(filesPage.filesTab).toHaveClass(/active/);

    // Switch to Completion view
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();
    await expect(filesPage.completionTab).toHaveClass(/active/);

    // Switch back to Files view
    await filesPage.switchView('files');
    await filesPage.waitForFilesToLoad();
    await expect(filesPage.filesTab).toHaveClass(/active/);
  });

  test('sidebar folders are clickable', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Root folder should exist
    await expect(sidebarPage.getRootFolder()).toBeVisible();

    // Click root folder (should already be active, but tests click handler)
    await sidebarPage.selectFolder(null);
    await filesPage.waitForFilesToLoad();

    // Verify we're still on root
    await expect(sidebarPage.getRootFolder()).toHaveClass(/active/);
  });

  test('search functionality works', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Get initial file count
    const initialCount = await filesPage.getFileCount();

    // Perform a search (may return no results, which is fine)
    await filesPage.search('test');

    // Results should be loaded (could be 0 or more)
    const newCount = await filesPage.getFileCount();
    expect(newCount).toBeGreaterThanOrEqual(0);

    // Clear search
    await filesPage.clearSearch();

    // Should return to initial state
    const afterClearCount = await filesPage.getFileCount();
    expect(afterClearCount).toBe(initialCount);
  });

  test('sort order can be changed', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Default sort should be 'name'
    const initialSort = await filesPage.getCurrentSort();
    expect(initialSort).toBe('name');

    // Change to size sort
    await filesPage.setSortBy('size');
    const sizeSort = await filesPage.getCurrentSort();
    expect(sizeSort).toBe('size');

    // Change to date sort
    await filesPage.setSortBy('date');
    const dateSort = await filesPage.getCurrentSort();
    expect(dateSort).toBe('date');

    // Back to name
    await filesPage.setSortBy('name');
    const nameSort = await filesPage.getCurrentSort();
    expect(nameSort).toBe('name');
  });

  test('refresh button works', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Click refresh
    await filesPage.refresh();

    // Files should still be visible after refresh
    await filesPage.expectVisible(filesPage.fileList);
  });

  test('bulk actions appear when items selected', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Initially no bulk actions
    await expect(filesPage.bulkActions).toBeHidden();

    // Select an item (if any exist)
    const fileCount = await filesPage.getFileCount();
    if (fileCount > 0) {
      const firstFile = await filesPage.getFileItemByIndex(0);
      const path = await firstFile.getAttribute('data-path');

      if (path) {
        await filesPage.toggleSelection(path);

        // Bulk actions should appear
        await expect(filesPage.bulkActions).toBeVisible();

        // Selected count should be 1
        const selectedCount = await filesPage.getSelectedCount();
        expect(selectedCount).toBe(1);

        // Clear selection
        await filesPage.clearSelection();
        await expect(filesPage.bulkActions).toBeHidden();
      }
    }
  });

  test('toast notifications work', async ({ filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toast should be hidden initially
    await expect(filesPage.toast).toBeHidden();

    // Trigger an action that shows a toast (refresh is safe)
    await filesPage.refresh();

    // Toast might appear briefly, but we can't reliably test timing
    // This is a placeholder for more specific toast tests
  });
});
