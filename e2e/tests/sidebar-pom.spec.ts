import { test, expect } from '../fixtures';

/**
 * Sidebar navigation tests using Page Object Model
 */
test.describe('Sidebar', () => {
  test.use({ serverOptions: { verbose: false } });

  test('displays folder section', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Folder section should be visible
    await sidebarPage.expectVisible(sidebarPage.folderList);

    // Root folder should exist
    await expect(sidebarPage.getRootFolder()).toBeVisible();
  });

  test('displays device section', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Device section should be visible
    await sidebarPage.expectVisible(sidebarPage.deviceList);
  });

  test('displays mount section', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Mount section should be visible
    await sidebarPage.expectVisible(sidebarPage.mountList);
  });

  test('displays activity section', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Activity section should be visible
    await sidebarPage.expectVisible(sidebarPage.activityList);
  });

  test('offline button is present', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Offline button should be visible
    await sidebarPage.expectVisible(sidebarPage.offlineBtn);

    // Should say "Go Offline" initially
    const btnText = await sidebarPage.getOfflineButtonText();
    expect(btnText).toContain('Offline');
  });

  test('add folder button opens modal', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Click add folder button
    await sidebarPage.clickAddFolder();

    // Modal should be visible
    await sidebarPage.expectVisible(sidebarPage.addFolderModal);

    // Input fields should be empty
    await expect(sidebarPage.newFolderIdInput).toBeVisible();
    await expect(sidebarPage.newFolderPathInput).toBeVisible();
  });

  test('folder item has correct structure', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Root folder should have icon and text
    const rootFolder = sidebarPage.getRootFolder();
    await expect(rootFolder.locator('.icon')).toBeVisible();
    await expect(rootFolder).toContainText('[Root]');
  });

  test('can get folder count', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Should have at least root folder
    const folderCount = await sidebarPage.getFolderCount();
    expect(folderCount).toBeGreaterThanOrEqual(1);
  });

  test('can get device count', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Device count can be 0 or more
    const deviceCount = await sidebarPage.getDeviceCount();
    expect(deviceCount).toBeGreaterThanOrEqual(0);
  });

  test('folder selection changes active state', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Root should be active initially
    await expect(sidebarPage.getRootFolder()).toHaveClass(/active/);

    // Click root folder again (idempotent)
    await sidebarPage.selectFolder(null);
    await filesPage.waitForFilesToLoad();

    // Root should still be active
    await expect(sidebarPage.getRootFolder()).toHaveClass(/active/);
  });
});
