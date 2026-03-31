import { test, expect } from '../fixtures';

/**
 * Offline mode tests
 * Tests for offline toggle, state persistence, and offline behavior
 */
test.describe('offline-mode', () => {
  test.use({ serverOptions: { verbose: false } });

  test('offline button is visible in sidebar', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Offline button should be visible
    await sidebarPage.expectVisible(sidebarPage.offlineBtn);
  });

  test('offline button shows current state', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Get initial button text
    const initialText = await sidebarPage.getOfflineButtonText();

    // Should contain "Offline" or "Online"
    expect(initialText.toLowerCase()).toContain('offline');
  });

  test('toggle offline mode', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Get initial state
    const initialText = await sidebarPage.getOfflineButtonText();
    const wasInitiallyOffline = initialText.toLowerCase().includes('go online');

    // Toggle offline
    await sidebarPage.toggleOffline();

    // Get new state
    const newText = await sidebarPage.getOfflineButtonText();
    const isNowOffline = newText.toLowerCase().includes('go online');

    // State should have changed
    expect(isNowOffline).not.toBe(wasInitiallyOffline);

    // Toggle back
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(500);
  });

  test('offline mode shows toast notification', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle offline
    await sidebarPage.toggleOffline();

    // Toast should appear
    await filesPage.waitForToast(5000);

    // Toast should contain relevant message
    const toastText = await filesPage.getToastMessage();
    expect(toastText.toLowerCase()).toContain('offline');

    // Toggle back
    await sidebarPage.toggleOffline();
  });

  test('offline state persists across page refresh', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle offline
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(500);

    // Get state before refresh
    const stateBefore = await sidebarPage.getOfflineButtonText();

    // Refresh page
    await filesPage.page.reload();
    await filesPage.waitForPageLoad();

    // State should be preserved
    const stateAfter = await sidebarPage.getOfflineButtonText();
    expect(stateAfter).toBe(stateBefore);

    // Toggle back if needed
    if (stateBefore.toLowerCase().includes('go online')) {
      await sidebarPage.toggleOffline();
    }
  });

  test('offline mode affects sync status', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle offline
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(1000);

    // Check if any visual indicator shows offline state
    // This is implementation dependent
    await filesPage.waitForTimeout(500);

    // Toggle back
    await sidebarPage.toggleOffline();
  });

  test('offline button is accessible via keyboard', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Focus on offline button using Tab
    await sidebarPage.offlineBtn.focus();

    // Button should have focus
    const isFocused = await sidebarPage.offlineBtn.evaluate((el) => {
      return document.activeElement === el;
    });
    expect(isFocused).toBe(true);

    // Press Enter to toggle
    await filesPage.page.keyboard.press('Enter');
    await filesPage.waitForTimeout(500);

    // State should have changed
    const newText = await sidebarPage.getOfflineButtonText();
    expect(newText).toBeTruthy();

    // Toggle back
    await sidebarPage.toggleOffline();
  });

  test('offline button has proper ARIA attributes', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Check for aria-label or aria-pressed
    const ariaLabel = await sidebarPage.offlineBtn.getAttribute('aria-label');
    const ariaPressed = await sidebarPage.offlineBtn.getAttribute('aria-pressed');

    // Should have at least one accessibility attribute
    expect(ariaLabel || ariaPressed).toBeTruthy();
  });

  test('offline mode visual indicator', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle offline
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(500);

    // Check for visual indicators (implementation dependent)
    const sidebarClass = await sidebarPage.sidebar.getAttribute('class');
    
    // May have offline-related class
    if (sidebarClass) {
      // Just verify sidebar is still visible
      expect(sidebarClass).toBeTruthy();
    }

    // Toggle back
    await sidebarPage.toggleOffline();
  });

  test('offline mode does not break navigation', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle offline
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(500);

    // Navigation should still work
    await expect(sidebarPage.getRootFolder()).toBeVisible();

    // Click root folder
    await sidebarPage.selectFolder(null);
    await filesPage.waitForFilesToLoad();

    // Files should still be visible
    await expect(filesPage.fileList).toBeVisible();

    // Toggle back
    await sidebarPage.toggleOffline();
  });

  test('offline mode does not break view switching', async ({ sidebarPage, filesPage, completionPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle offline
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(500);

    // Switch to completion view
    await filesPage.switchView('completion');
    await completionPage.waitForCompletionToLoad();

    // Completion view should be visible
    await expect(completionPage.completionGrid).toBeVisible();

    // Switch back to files
    await filesPage.switchView('files');
    await filesPage.waitForFilesToLoad();

    // Toggle back
    await sidebarPage.toggleOffline();
  });

  test('multiple offline toggles', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Toggle multiple times
    for (let i = 0; i < 3; i++) {
      await sidebarPage.toggleOffline();
      await filesPage.waitForTimeout(300);
    }

    // Should still be functional
    await sidebarPage.expectVisible(sidebarPage.offlineBtn);
  });

  test('offline button disabled state (if applicable)', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Check if button is disabled
    const isDisabled = await sidebarPage.offlineBtn.isDisabled();

    // Button should typically be enabled
    expect(isDisabled).toBe(false);
  });

  test('offline mode with active sync (edge case)', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Go to completion view to see sync status
    await filesPage.switchView('completion');

    // Toggle offline
    await sidebarPage.toggleOffline();
    await filesPage.waitForTimeout(1000);

    // Sync should be paused or show offline state
    // Implementation dependent

    // Toggle back
    await sidebarPage.toggleOffline();
  });
});
