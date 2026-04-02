import { Locator } from '@playwright/test';
import { BasePage } from './base-page';

/**
 * Page Object for sidebar navigation
 * Handles folder, device, and mount interactions
 */
export class SidebarPage extends BasePage {
  // Sidebar-specific locators
  readonly folderList: Locator;
  readonly pendingFolderList: Locator;
  readonly deviceList: Locator;
  readonly mountList: Locator;
  readonly activityList: Locator;
  readonly logoutBtn: Locator;
  readonly addFolderBtn: Locator;
  readonly addDeviceBtn: Locator;

  constructor(page: any) {
    super(page);
    this.folderList = page.locator('#folder-list');
    this.pendingFolderList = page.locator('#pending-folder-list');
    this.deviceList = page.locator('#device-list');
    this.mountList = page.locator('#mount-list');
    this.activityList = page.locator('#activity-list');
    this.logoutBtn = page.locator('button[onclick="logout()"]');
    this.addFolderBtn = page.locator('button[onclick="addFolder()"]');
    this.addDeviceBtn = page.locator('button[onclick="addDevice()"]');
  }

  /**
   * Get folder item by ID
   */
  getFolderItem(folderId: string): Locator {
    return this.folderList.locator(`.folder-item:has-text("${folderId}")`);
  }

  /**
   * Get root folder item
   */
  getRootFolder(): Locator {
    return this.folderList.locator('.folder-item:has-text("[Root]")');
  }

  /**
   * Get pending folder item by ID
   */
  getPendingFolderItem(folderId: string): Locator {
    return this.pendingFolderList.locator(`.folder-item:has-text("${folderId}")`);
  }

  /**
   * Get device item by name or ID
   */
  getDeviceItem(nameOrId: string): Locator {
    return this.deviceList.locator(`.folder-item:has-text("${nameOrId}")`);
  }

  /**
   * Get pending device item
   */
  getPendingDeviceItem(deviceId: string): Locator {
    return this.deviceList.locator(`.folder-item:has-text("Pending: ${deviceId.substring(0, 7)}")`);
  }

  /**
   * Get mount item by path
   */
  getMountItem(mountpoint: string): Locator {
    return this.mountList.locator(`.folder-item:has-text("${mountpoint}")`);
  }

  /**
   * Get activity item by index
   */
  getActivityItem(index: number): Locator {
    return this.activityList.locator('.folder-item').nth(index);
  }

  /**
   * Select a folder
   */
  async selectFolder(folderId: string | null): Promise<void> {
    if (folderId === null) {
      await this.getRootFolder().click();
    } else {
      await this.getFolderItem(folderId).click();
    }
    await this.page.waitForTimeout(500);
  }

  /**
   * Join a pending folder
   */
  async joinPendingFolder(folderId: string): Promise<void> {
    await this.getPendingFolderItem(folderId).click();
    await this.page.waitForTimeout(500);
  }

  /**
   * Accept a pending device
   */
  async acceptPendingDevice(deviceId: string): Promise<void> {
    await this.getPendingDeviceItem(deviceId).click();
    await this.page.waitForTimeout(500);
  }

  /**
   * Delete a folder (right-click context menu)
   */
  async deleteFolder(folderId: string): Promise<void> {
    const folder = this.getFolderItem(folderId);
    await folder.click({ button: 'right' });
    await this.page.waitForTimeout(300);
    // Confirm dialog will appear - handled by test
  }

  /**
   * Delete a device (right-click context menu)
   */
  async deleteDevice(nameOrId: string): Promise<void> {
    const device = this.getDeviceItem(nameOrId);
    await device.click({ button: 'right' });
    await this.page.waitForTimeout(300);
    // Confirm dialog will appear - handled by test
  }

  /**
   * Click add folder button (opens modal)
   */
  async clickAddFolder(): Promise<void> {
    await this.addFolderBtn.click();
    // Wait for modal to be visible (using display style check since it uses inline style)
    await this.page.waitForFunction(() => {
      const modal = (globalThis as any).document.getElementById('add-folder-ui');
      return modal && modal.style.display !== 'none';
    }, { timeout: 5000 });
  }

  /**
   * Click add device button
   */
  async clickAddDevice(): Promise<void> {
    await this.addDeviceBtn.click();
    await this.page.waitForTimeout(300);
  }

  /**
   * Check if sidebar is visible (for mobile)
   */
  async isSidebarVisible(): Promise<boolean> {
    return await this.sidebar.isVisible();
  }

  /**
   * Get folder count
   */
  async getFolderCount(): Promise<number> {
    return await this.folderList.locator('.folder-item').count();
  }

  /**
   * Get device count
   */
  async getDeviceCount(): Promise<number> {
    return await this.deviceList.locator('.folder-item').count();
  }

  /**
   * Get pending folder count
   */
  async getPendingFolderCount(): Promise<number> {
    const container = this.page.locator('#pending-folder-container');
    if (!(await container.isVisible())) {
      return 0;
    }
    return await this.pendingFolderList.locator('.folder-item').count();
  }

  /**
   * Get pending device count
   */
  async getPendingDeviceCount(): Promise<number> {
    let count = 0;
    const items = this.deviceList.locator('.folder-item');
    const itemCount = await items.count();
    for (let i = 0; i < itemCount; i++) {
      const text = await items.nth(i).textContent();
      if (text?.includes('Pending:')) {
        count++;
      }
    }
    return count;
  }

  /**
   * Get activity count
   */
  async getActivityCount(): Promise<number> {
    return await this.activityList.locator('.folder-item').count();
  }

  /**
   * Wait for folder list to be populated
   */
  async waitForFolders(timeout: number = 5000): Promise<void> {
    await this.folderList.locator('.folder-item').first().waitFor({ state: 'visible', timeout });
  }

  /**
   * Wait for device list to be populated
   */
  async waitForDevices(timeout: number = 5000): Promise<void> {
    await this.deviceList.locator('.folder-item').first().waitFor({ state: 'visible', timeout });
  }

  /**
   * Check if folder is active (selected)
   */
  async isFolderActive(folderId: string | null): Promise<boolean> {
    if (folderId === null) {
      return await this.hasClass(this.getRootFolder(), 'active');
    }
    return await this.hasClass(this.getFolderItem(folderId), 'active');
  }
}
