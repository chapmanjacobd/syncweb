import { Locator } from '@playwright/test';
import { BasePage } from './base-page';

/**
 * Page Object for Completion view
 * Handles sync completion monitoring and progress visualization
 */
export class CompletionPage extends BasePage {
  // Completion-specific locators
  readonly completionGrid: Locator;
  readonly completionCards: Locator;
  readonly completionFolderSelect: Locator;
  readonly completionDeviceSelect: Locator;

  constructor(page: any) {
    super(page);
    this.completionGrid = page.locator('#completion-grid');
    this.completionCards = this.completionGrid.locator('.completion-card');
    this.completionFolderSelect = page.locator('#completion-folder-select');
    this.completionDeviceSelect = page.locator('#completion-device-select');
  }

  /**
   * Wait for completion view to load
   */
  async waitForCompletionToLoad(timeout: number = 10000): Promise<void> {
    await this.completionGrid.waitFor({ state: 'visible', timeout });
  }

  /**
   * Get completion card by folder/device
   */
  getCompletionCard(folderId: string, deviceId?: string): Locator {
    if (deviceId) {
      return this.completionCards.filter({ hasText: folderId }).filter({ hasText: deviceId });
    }
    return this.completionCards.filter({ hasText: folderId });
  }

  /**
   * Get completion card by index
   */
  getCompletionCardByIndex(index: number): Locator {
    return this.completionCards.nth(index);
  }

  /**
   * Get progress bar from card
   */
  getProgressBar(card: Locator): Locator {
    return card.locator('.progress-bar');
  }

  /**
   * Get progress fill from card
   */
  getProgressFill(card: Locator): Locator {
    return card.locator('.progress-fill');
  }

  /**
   * Get progress percentage from card
   */
  async getProgressPercentage(card: Locator): Promise<number> {
    const fill = this.getProgressFill(card);
    const style = await fill.getAttribute('style');
    const match = style?.match(/width:\s*(\d+)%/);
    return match ? parseInt(match[1]) : 0;
  }

  /**
   * Get progress stats from card
   */
  async getProgressStats(card: Locator): Promise<{ synced: number; total: number }> {
    const stats = card.locator('.progress-stats');
    const text = await stats.textContent() || '0 / 0';
    const match = text.match(/(\d+)\s*\/\s*(\d+)/);
    return match ? { synced: parseInt(match[1]), total: parseInt(match[2]) } : { synced: 0, total: 0 };
  }

  /**
   * Get completion percentage badge
   */
  async getCompletionPct(card: Locator): Promise<string> {
    const pct = card.locator('.completion-pct');
    return await pct.textContent() || '0%';
  }

  /**
   * Get card title (folder/device name)
   */
  async getCardTitle(card: Locator): Promise<string> {
    const title = card.locator('h4');
    return await title.textContent() || '';
  }

  /**
   * Filter by folder
   */
  async filterByFolder(folderId: string): Promise<void> {
    await this.completionFolderSelect.selectOption(folderId);
    await this.page.waitForTimeout(500);
  }

  /**
   * Filter by device
   */
  async filterByDevice(deviceId: string): Promise<void> {
    await this.completionDeviceSelect.selectOption(deviceId);
    await this.page.waitForTimeout(500);
  }

  /**
   * Clear filters
   */
  async clearFilters(): Promise<void> {
    await this.completionFolderSelect.selectOption('');
    await this.completionDeviceSelect.selectOption('');
    await this.page.waitForTimeout(500);
  }

  /**
   * Click refresh button
   */
  async refresh(): Promise<void> {
    const refreshBtn = this.page.locator('#completion-device-select + button');
    await refreshBtn.click();
    await this.waitForCompletionToLoad();
  }

  /**
   * Get completion card count
   */
  async getCompletionCardCount(): Promise<number> {
    return await this.completionCards.count();
  }

  /**
   * Check if completion card exists
   */
  async completionCardExists(folderId: string, deviceId?: string): Promise<boolean> {
    return await this.getCompletionCard(folderId, deviceId).count() > 0;
  }

  /**
   * Wait for progress to reach percentage
   */
  async waitForProgress(percentage: number, timeout: number = 30000): Promise<void> {
    await this.page.waitForFunction(async ({ pct }) => {
      const fill = (globalThis as any).document.querySelector('.progress-fill');
      if (!fill) return false;
      const style = (fill as any).style.width;
      const current = parseInt(style) || 0;
      return current >= pct;
    }, { pct: percentage }, { timeout });
  }

  /**
   * Get all folder options from select
   */
  async getFolderOptions(): Promise<string[]> {
    const options = this.completionFolderSelect.locator('option');
    const count = await options.count();
    const values: string[] = [];
    for (let i = 0; i < count; i++) {
      const value = await options.nth(i).getAttribute('value');
      if (value) values.push(value);
    }
    return values;
  }

  /**
   * Get all device options from select
   */
  async getDeviceOptions(): Promise<string[]> {
    const options = this.completionDeviceSelect.locator('option');
    const count = await options.count();
    const values: string[] = [];
    for (let i = 0; i < count; i++) {
      const value = await options.nth(i).getAttribute('value');
      if (value) values.push(value);
    }
    return values;
  }

  /**
   * Check if card has error state
   */
  async hasError(card: Locator): Promise<boolean> {
    const statusBadge = card.locator('.status-badge.error');
    return await statusBadge.count() > 0;
  }

  /**
   * Get error message from card
   */
  async getErrorMessage(card: Locator): Promise<string> {
    const errorEl = card.locator('.status-badge.error, .error-text');
    return await errorEl.textContent() || '';
  }
}
