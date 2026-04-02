import { Page, Locator, expect } from '@playwright/test';

/**
 * Base page object with common functionality
 * All page objects should extend this class
 */
export class BasePage {
  readonly page: Page;

  // Common locators shared across multiple pages
  readonly menuBtn: Locator;
  readonly sidebar: Locator;
  readonly toast: Locator;
  readonly currentPath: Locator;
  readonly currentFolderTitle: Locator;
  readonly breadcrumbs: Locator;
  readonly sortSelect: Locator;
  readonly searchInput: Locator;
  readonly searchButton: Locator;
  readonly refreshButton: Locator;
  readonly bulkActions: Locator;
  readonly selectedCount: Locator;

  // View tabs
  readonly viewTabs: Locator;
  readonly filesTab: Locator;
  readonly completionTab: Locator;
  readonly treeTab: Locator;
  readonly localChangedTab: Locator;
  readonly needTab: Locator;
  readonly remoteNeedTab: Locator;

  // Modals
  readonly addFolderModal: Locator;
  readonly newFolderIdInput: Locator;
  readonly newFolderPathInput: Locator;
  readonly pathPreview: Locator;

  constructor(page: Page) {
    this.page = page;

    // Common locators
    this.menuBtn = page.locator('#menu-btn');
    this.sidebar = page.locator('aside.sidebar-left');
    this.toast = page.locator('#toast');
    this.currentPath = page.locator('#current-path');
    this.currentFolderTitle = page.locator('#current-folder-title');
    this.breadcrumbs = page.locator('#breadcrumbs');
    this.sortSelect = page.locator('#sort-select');
    this.searchInput = page.locator('#search-input');
    this.searchButton = page.locator('button[onclick="searchFiles()"]');
    this.refreshButton = page.locator('button[onclick="refresh()"]');
    this.bulkActions = page.locator('#bulk-actions');
    this.selectedCount = page.locator('#selected-count');

    // View tabs
    this.viewTabs = page.locator('.view-tabs');
    this.filesTab = page.locator('.view-tab:has-text("Files")');
    this.completionTab = page.locator('.view-tab:has-text("Completion")');
    this.treeTab = page.locator('.view-tab:has-text("Tree")');
    this.localChangedTab = page.locator('.view-tab:has-text("Local Changed")');
    this.needTab = page.locator('.view-tab:has-text("Need")');
    this.remoteNeedTab = page.locator('.view-tab:has-text("Remote Need")');

    // Modals
    this.addFolderModal = page.locator('#add-folder-ui');
    this.newFolderIdInput = page.locator('#new-folder-id');
    this.newFolderPathInput = page.locator('#new-folder-path');
    this.pathPreview = page.locator('#path-preview');
  }

  /**
   * Wait for toast notification
   */
  async waitForToast(timeout: number = 5000): Promise<void> {
    await this.toast.waitFor({ state: 'visible', timeout });
  }

  /**
   * Get toast message
   */
  async getToastMessage(): Promise<string> {
    return await this.toast.textContent() || '';
  }

  /**
   * Check if toast contains text
   */
  async toastContainsText(text: string): Promise<boolean> {
    const toastText = await this.getToastMessage();
    return toastText.includes(text);
  }

  /**
   * Open sidebar on mobile
   */
  async openSidebar(): Promise<void> {
    if (await this.menuBtn.isVisible()) {
      await this.menuBtn.click();
      await this.sidebar.waitFor({ state: 'visible' });
    }
  }

  /**
   * Close sidebar on mobile
   */
  async closeSidebar(): Promise<void> {
    if (await this.menuBtn.isVisible()) {
      await this.menuBtn.click();
      await this.sidebar.waitFor({ state: 'hidden' });
    }
  }

  /**
   * Check if element is visible
   */
  async isVisible(locator: Locator): Promise<boolean> {
    return await locator.isVisible();
  }

  /**
   * Wait for element to be visible
   */
  async waitForVisible(locator: Locator, timeout: number = 5000): Promise<void> {
    await locator.waitFor({ state: 'visible', timeout });
  }

  /**
   * Click element if visible
   */
  async clickIfVisible(locator: Locator): Promise<void> {
    if (await locator.isVisible()) {
      await locator.click();
    }
  }

  /**
   * Get element text content
   */
  async getText(locator: Locator): Promise<string> {
    return await locator.textContent() || '';
  }

  /**
   * Get element attribute
   */
  async getAttribute(locator: Locator, attr: string): Promise<string | null> {
    return await locator.getAttribute(attr);
  }

  /**
   * Get element count
   */
  async getCount(locator: Locator): Promise<number> {
    return await locator.count();
  }

  /**
   * Assert element is visible
   */
  async expectVisible(locator: Locator): Promise<void> {
    await expect(locator).toBeVisible();
  }

  /**
   * Assert element is hidden
   */
  async expectHidden(locator: Locator): Promise<void> {
    await expect(locator).toBeHidden();
  }

  /**
   * Assert element has text
   */
  async expectHasText(locator: Locator, text: string): Promise<void> {
    await expect(locator).toContainText(text);
  }

  /**
   * Assert element has attribute
   */
  async expectAttribute(locator: Locator, attr: string, value: string): Promise<void> {
    await expect(locator).toHaveAttribute(attr, value);
  }

  /**
   * Wait for page load
   */
  async waitForPageLoad(timeout: number = 10000): Promise<void> {
    await this.page.waitForLoadState('networkidle', { timeout });
  }

  /**
   * Wait for timeout (use sparingly)
   */
  async waitForTimeout(ms: number): Promise<void> {
    await this.page.waitForTimeout(ms);
  }

  /**
   * Wait for function to return true
   */
  async waitForFunction<T>(fn: () => T, options?: { timeout?: number }): Promise<void> {
    await this.page.waitForFunction(fn, undefined, options);
  }

  /**
   * Switch to a specific view tab
   */
  async switchView(viewName: 'files' | 'completion' | 'tree' | 'local-changed' | 'need' | 'remote-need'): Promise<void> {
    const tab = this.page.locator(`.view-tab:has-text("${viewName.charAt(0).toUpperCase() + viewName.slice(1).replace('-', ' ')}")`);
    await tab.click();
    await this.page.waitForTimeout(500);
  }

  /**
   * Get current view from URL or state
   */
  async getCurrentView(): Promise<string> {
    return await this.page.evaluate(() => {
      const activeTab = (globalThis as any).document.querySelector('.view-tab.active');
      return activeTab?.textContent?.toLowerCase().replace(' ', '-') || 'files';
    });
  }

  /**
   * Wait for specific view to be active
   */
  async waitForView(viewName: string, timeout: number = 5000): Promise<void> {
    await this.page.waitForFunction((view) => {
      const activeTab = (globalThis as any).document.querySelector('.view-tab.active');
      return activeTab?.textContent?.toLowerCase().replace(' ', '-') === view;
    }, viewName, { timeout });
  }

  /**
   * Check if element has a specific CSS class
   */
  async hasClass(locator: Locator, className: string): Promise<boolean> {
    const classAttribute = await locator.getAttribute('class');
    return classAttribute?.includes(className) || false;
  }
}
