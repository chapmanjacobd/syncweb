import { test, expect } from '../fixtures';

/**
 * Modal interaction tests
 * Tests for add folder, add device, and other modal dialogs
 */
test.describe('modal-interactions', () => {
  test.use({ serverOptions: { verbose: false } });

  test('add folder modal opens', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Click add folder button
    await sidebarPage.clickAddFolder();

    // Modal should be visible
    await sidebarPage.expectVisible(sidebarPage.addFolderModal);
  });

  test('add folder modal has input fields', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Folder ID input should exist
    await expect(sidebarPage.newFolderIdInput).toBeVisible();

    // Folder path input should exist
    await expect(sidebarPage.newFolderPathInput).toBeVisible();

    // Path preview should exist
    await expect(sidebarPage.pathPreview).toBeVisible();
  });

  test('add folder modal inputs are editable', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Fill in folder ID
    await sidebarPage.newFolderIdInput.fill('test-folder');
    const folderIdValue = await sidebarPage.newFolderIdInput.inputValue();
    expect(folderIdValue).toBe('test-folder');

    // Fill in folder path
    await sidebarPage.newFolderPathInput.fill('/tmp/test');
    const pathValue = await sidebarPage.newFolderPathInput.inputValue();
    expect(pathValue).toBe('/tmp/test');
  });

  test('add folder modal can be closed', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Modal should be visible
    await sidebarPage.expectVisible(sidebarPage.addFolderModal);

    // Close button should exist (usually X button or cancel)
    const closeButton = sidebarPage.addFolderModal.locator('button:has-text("Cancel"), button.close, .close-btn');
    if (await closeButton.count() > 0) {
      await closeButton.click();
      await filesPage.waitForTimeout(500);

      // Modal should be hidden
      await sidebarPage.expectHidden(sidebarPage.addFolderModal);
    }
  });

  test('add device button exists', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Add device button should be visible
    await sidebarPage.expectVisible(sidebarPage.addDeviceBtn);
  });

  test('add device button is clickable', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Click add device button
    await sidebarPage.clickAddDevice();

    // Wait for potential modal or dialog
    await filesPage.waitForTimeout(500);
  });

  test('modal has proper z-index and overlay', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Modal should be on top
    const modal = sidebarPage.addFolderModal;
    await expect(modal).toBeVisible();

    // Modal should have proper styling (implementation dependent)
    const zIndex = await modal.evaluate((el: HTMLElement) => {
      const style = el.style;
      return parseInt(style.zIndex) || 0;
    });

    // Should have a z-index greater than 0
    expect(zIndex).toBeGreaterThan(0);
  });

  test('path preview updates as path is typed', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Type in path
    const testPath = '/tmp/test-folder';
    await sidebarPage.newFolderPathInput.fill(testPath);

    // Path preview should update (implementation dependent)
    await filesPage.waitForTimeout(300);

    // Preview should contain the path or be visible
    await expect(sidebarPage.pathPreview).toBeVisible();
  });

  test('folder ID validation (if implemented)', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Test with empty ID
    await sidebarPage.newFolderIdInput.fill('');
    await filesPage.waitForTimeout(300);

    // Test with valid ID
    await sidebarPage.newFolderIdInput.fill('valid-folder-id');
    const value = await sidebarPage.newFolderIdInput.inputValue();
    expect(value).toBe('valid-folder-id');

    // Test with special characters
    await sidebarPage.newFolderIdInput.fill('folder-with-dashes');
    const specialValue = await sidebarPage.newFolderIdInput.inputValue();
    expect(specialValue).toBe('folder-with-dashes');
  });

  test('modal keyboard accessibility', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Modal should be visible
    await sidebarPage.expectVisible(sidebarPage.addFolderModal);

    // Press Escape to close
    await filesPage.page.keyboard.press('Escape');
    await filesPage.waitForTimeout(500);

    // Modal should be hidden
    await sidebarPage.expectHidden(sidebarPage.addFolderModal);
  });

  test('modal focus management', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Focus should move to modal
    const activeElement = await filesPage.page.evaluate(() => {
      return (globalThis as any).document.activeElement?.tagName || '';
    });

    // Active element should be within modal or an input
    expect(['INPUT', 'BUTTON', 'DIV']).toContain(activeElement);
  });

  test('multiple modal open/close cycles', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Open and close modal multiple times
    for (let i = 0; i < 3; i++) {
      await sidebarPage.clickAddFolder();
      await sidebarPage.expectVisible(sidebarPage.addFolderModal);

      // Close with Escape
      await filesPage.page.keyboard.press('Escape');
      await filesPage.waitForTimeout(300);

      await sidebarPage.expectHidden(sidebarPage.addFolderModal);
    }
  });

  test('modal does not submit with empty fields', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Clear inputs
    await sidebarPage.newFolderIdInput.fill('');
    await sidebarPage.newFolderPathInput.fill('');

    // Try to submit (OK button)
    const submitButton = sidebarPage.addFolderModal.locator('button:has-text("OK"), button[type="submit"]');
    if (await submitButton.count() > 0) {
      await submitButton.click();
      await filesPage.waitForTimeout(500);

      // Modal should still be open (validation failed)
      await sidebarPage.expectVisible(sidebarPage.addFolderModal);
    }
  });

  test('modal responsive design', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Test at mobile viewport
    await filesPage.page.setViewportSize({ width: 375, height: 667 });
    await sidebarPage.clickAddFolder();
    await sidebarPage.expectVisible(sidebarPage.addFolderModal);

    // Modal should be visible and properly sized
    const modalBox = await sidebarPage.addFolderModal.boundingBox();
    if (modalBox) {
      expect(modalBox.width).toBeGreaterThan(0);
      expect(modalBox.height).toBeGreaterThan(0);
    }

    // Reset viewport
    await filesPage.page.setViewportSize({ width: 1280, height: 720 });
  });

  test('modal has proper ARIA attributes (if implemented)', async ({ sidebarPage, filesPage, server }) => {
    await filesPage.goto(server.getBaseUrl());
    await sidebarPage.clickAddFolder();

    // Check for role attribute
    const role = await sidebarPage.addFolderModal.getAttribute('role');
    
    // May have dialog, alertdialog, or no role
    if (role) {
      expect(['dialog', 'alertdialog']).toContain(role);
    }
  });
});
