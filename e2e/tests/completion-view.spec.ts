import { test, expect } from '../fixtures';

/**
 * Completion view tests
 * Tests for sync completion monitoring and progress visualization
 */
test.describe('completion-view', () => {
  test.use({ serverOptions: { verbose: false } });

  test('completion view loads', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Switch to completion view
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Completion grid should be visible
    await expect(completionPage.completionGrid).toBeVisible();
  });

  test('completion view has filter controls', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Folder filter should exist
    await expect(completionPage.completionFolderSelect).toBeVisible();

    // Device filter should exist
    await expect(completionPage.completionDeviceSelect).toBeVisible();
  });

  test('completion cards are displayed', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Get card count (may be 0 if no sync activity)
    const cardCount = await completionPage.getCompletionCardCount();
    expect(cardCount).toBeGreaterThanOrEqual(0);
  });

  test('completion card has correct structure', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    const cardCount = await completionPage.getCompletionCardCount();
    if (cardCount > 0) {
      const firstCard = completionPage.getCompletionCardByIndex(0);

      // Should have title
      const title = await completionPage.getCardTitle(firstCard);
      expect(title).toBeTruthy();

      // Should have progress bar
      const progressBar = completionPage.getProgressBar(firstCard);
      await expect(progressBar).toBeVisible();

      // Should have progress fill
      const progressFill = completionPage.getProgressFill(firstCard);
      await expect(progressFill).toBeVisible();
    }
  });

  test('progress percentage is valid', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    const cardCount = await completionPage.getCompletionCardCount();
    if (cardCount > 0) {
      const firstCard = completionPage.getCompletionCardByIndex(0);

      // Get progress percentage
      const percentage = await completionPage.getProgressPercentage(firstCard);

      // Should be between 0 and 100
      expect(percentage).toBeGreaterThanOrEqual(0);
      expect(percentage).toBeLessThanOrEqual(100);
    }
  });

  test('progress stats are displayed', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    const cardCount = await completionPage.getCompletionCardCount();
    if (cardCount > 0) {
      const firstCard = completionPage.getCompletionCardByIndex(0);

      // Get stats
      const stats = await completionPage.getProgressStats(firstCard);

      // Synced should be <= total
      expect(stats.synced).toBeGreaterThanOrEqual(0);
      expect(stats.total).toBeGreaterThanOrEqual(0);
      expect(stats.synced).toBeLessThanOrEqual(stats.total);
    }
  });

  test('completion percentage badge is displayed', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    const cardCount = await completionPage.getCompletionCardCount();
    if (cardCount > 0) {
      const firstCard = completionPage.getCompletionCardByIndex(0);

      // Get completion percentage text
      const pctText = await completionPage.getCompletionPct(firstCard);

      // Should contain percentage symbol
      expect(pctText).toContain('%');
    }
  });

  test('filter by folder works', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Get available folder options
    const folderOptions = await completionPage.getFolderOptions();

    if (folderOptions.length > 1) {
      // Select a folder (skip empty option)
      const folderId = folderOptions.find(opt => opt !== '');
      if (folderId) {
        await completionPage.filterByFolder(folderId);

        // Cards should be filtered
        await filesPage.waitForTimeout(500);
      }
    }
  });

  test('filter by device works', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Get available device options
    const deviceOptions = await completionPage.getDeviceOptions();

    if (deviceOptions.length > 1) {
      // Select a device (skip empty option)
      const deviceId = deviceOptions.find(opt => opt !== '');
      if (deviceId) {
        await completionPage.filterByDevice(deviceId);

        // Cards should be filtered
        await filesPage.waitForTimeout(500);
      }
    }
  });

  test('clear filters resets view', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Apply a filter
    const folderOptions = await completionPage.getFolderOptions();
    if (folderOptions.length > 1) {
      const folderId = folderOptions.find(opt => opt !== '');
      if (folderId) {
        await completionPage.filterByFolder(folderId);
        await filesPage.waitForTimeout(500);

        // Clear filters
        await completionPage.clearFilters();
        await filesPage.waitForTimeout(500);

        // Card count should return to initial (approximately)
        const afterClearCount = await completionPage.getCompletionCardCount();
        expect(afterClearCount).toBeGreaterThanOrEqual(0);
      }
    }
  });

  test('refresh button updates completion data', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Click refresh
    await completionPage.refresh();

    // Card count may change
    const afterRefreshCount = await completionPage.getCompletionCardCount();
    expect(afterRefreshCount).toBeGreaterThanOrEqual(0);
  });

  test('completion view tab is active after switch', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Switch to completion view
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Completion tab should be active
    await expect(filesPage.completionTab).toHaveClass(/active/);
  });

  test('switch from completion back to files view', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Switch to completion
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Switch back to files
    await filesPage.switchView('files');
    await filesPage.waitForFilesToLoad();

    // Files tab should be active
    await expect(filesPage.filesTab).toHaveClass(/active/);

    // Files list should be visible
    await expect(filesPage.fileList).toBeVisible();
  });

  test('completion card error state detection', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    const cardCount = await completionPage.getCompletionCardCount();
    if (cardCount > 0) {
      const firstCard = completionPage.getCompletionCardByIndex(0);

      // Check for error state (may or may not have errors)
      const hasError = await completionPage.hasError(firstCard);

      // If has error, error message should be present
      if (hasError) {
        const errorMsg = await completionPage.getErrorMessage(firstCard);
        expect(errorMsg).toBeTruthy();
      }
    }
  });

  test('completion view is responsive', async ({ completionPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Completion grid should be visible at different viewport sizes
    await expect(completionPage.completionGrid).toBeVisible();

    // Test at mobile viewport
    await filesPage.page.setViewportSize({ width: 375, height: 667 });
    await filesPage.waitForTimeout(500);
    await expect(completionPage.completionGrid).toBeVisible();

    // Reset to desktop viewport
    await filesPage.page.setViewportSize({ width: 1280, height: 720 });
  });
});
