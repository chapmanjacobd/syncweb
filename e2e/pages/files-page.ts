import { Locator } from '@playwright/test';
import { BasePage } from './base-page';

/**
 * Page Object for files view
 * Handles file/folder interactions, selection, and operations
 */
export class FilesPage extends BasePage {
  // Files-specific locators
  readonly fileList: Locator;
  readonly fileItems: Locator;
  readonly parentDirItem: Locator;

  constructor(page: any) {
    super(page);
    this.fileList = page.locator('#file-list');
    this.fileItems = this.fileList.locator('.file-item');
    this.parentDirItem = this.fileList.locator('.file-item:has-text("..")');
  }

  /**
   * Navigate to the base URL and wait for files to load
   */
  async goto(baseUrl: string, timeout: number = 10000): Promise<void> {
    await this.page.goto(baseUrl);
    await this.waitForFilesToLoad(timeout);
  }

  /**
   * Wait for files to load (or for the file list to be ready, even if empty)
   */
  async waitForFilesToLoad(timeout: number = 10000): Promise<void> {
    // Wait for the file list element to be attached to the DOM
    await this.fileList.waitFor({ state: 'attached', timeout });
    // Don't wait for files or visibility - empty state is valid
  }

  /**
   * Get file item by path
   */
  getFileItem(path: string): Locator {
    return this.fileList.locator(`.file-item[data-path="${path}"]`);
  }

  /**
   * Get file item by name
   */
  getFileItemByName(name: string): Locator {
    return this.fileList.locator(`.file-item:has-text("${name}")`);
  }

  /**
   * Get file item by index
   */
  getFileItemByIndex(index: number): Locator {
    return this.fileItems.nth(index);
  }

  /**
   * Get folder items only
   */
  getFolderItems(): Locator {
    return this.fileList.locator('.file-item.is-dir');
  }

  /**
   * Get file items only (not folders)
   */
  getFileOnlyItems(): Locator {
    return this.fileList.locator('.file-item:not(.is-dir)');
  }

  /**
   * Get selected items
   */
  getSelectedItems(): Locator {
    return this.fileList.locator('.file-item.selected');
  }

  /**
   * Get checkbox for file item
   */
  getFileCheckbox(path: string): Locator {
    return this.getFileItem(path).locator('input[type="checkbox"]');
  }

  /**
   * Get file icon
   */
  getFileIcon(path: string): Locator {
    return this.getFileItem(path).locator('.icon');
  }

  /**
   * Get file size
   */
  async getFileSize(path: string): Promise<string> {
    const item = this.getFileItem(path);
    return await item.locator('.secondary-info').textContent() || '';
  }

  /**
   * Check if file is a directory
   */
  async isDirectory(path: string): Promise<boolean> {
    const item = this.getFileItem(path);
    return await this.hasClass(item, 'is-dir');
  }

  /**
   * Check if file is selected
   */
  async isSelected(path: string): Promise<boolean> {
    const item = this.getFileItem(path);
    return await this.hasClass(item, 'selected');
  }

  /**
   * Check if file is local
   */
  async isLocal(path: string): Promise<boolean> {
    const item = this.getFileItem(path);
    const cloudIcon = item.locator('[data-lucide="cloud"]');
    return !(await cloudIcon.count() > 0);
  }

  /**
   * Click on file/folder
   */
  async clickFile(path: string): Promise<void> {
    await this.getFileItem(path).click();
    await this.page.waitForTimeout(500);
  }

  /**
   * Double click on folder to navigate into it
   */
  async doubleClickFolder(path: string): Promise<void> {
    await this.getFileItem(path).dblclick();
    await this.page.waitForTimeout(500);
  }

  /**
   * Right click file to show context menu
   */
  async rightClickFile(path: string): Promise<void> {
    await this.getFileItem(path).click({ button: 'right' });
    await this.page.waitForTimeout(300);
  }

  /**
   * Toggle file selection via checkbox
   */
  async toggleSelection(path: string): Promise<void> {
    await this.getFileCheckbox(path).click();
    await this.page.waitForTimeout(300);
  }

  /**
   * Select multiple files
   */
  async selectMultipleFiles(paths: string[]): Promise<void> {
    for (const path of paths) {
      await this.toggleSelection(path);
    }
  }

  /**
   * Clear selection
   */
  async clearSelection(): Promise<void> {
    if (await this.bulkActions.isVisible()) {
      const cancelBtn = this.bulkActions.locator('button:has-text("Cancel")');
      await cancelBtn.click();
      await this.page.waitForTimeout(300);
    }
  }

  /**
   * Get selected count
   */
  async getSelectedCount(): Promise<number> {
    if (!(await this.bulkActions.isVisible())) {
      return 0;
    }
    const text = await this.selectedCount.textContent() || '0 selected';
    const match = text.match(/(\d+)/);
    return match ? parseInt(match[1]) : 0;
  }

  /**
   * Check if bulk actions bar is visible
   */
  async isBulkActionsVisible(): Promise<boolean> {
    return await this.bulkActions.isVisible();
  }

  /**
   * Click bulk move button
   */
  async clickBulkMove(): Promise<void> {
    const moveBtn = this.bulkActions.locator('button:has-text("Move")');
    await moveBtn.click();
    await this.page.waitForTimeout(300);
  }

  /**
   * Click bulk copy button
   */
  async clickBulkCopy(): Promise<void> {
    const copyBtn = this.bulkActions.locator('button:has-text("Copy")');
    await copyBtn.click();
    await this.page.waitForTimeout(300);
  }

  /**
   * Click bulk delete button
   */
  async clickBulkDelete(): Promise<void> {
    const deleteBtn = this.bulkActions.locator('button:has-text("Delete")');
    await deleteBtn.click();
    await this.page.waitForTimeout(300);
  }

  /**
   * Search for files
   */
  async search(query: string): Promise<void> {
    await this.searchInput.fill(query);
    await this.searchButton.click();
    await this.waitForFilesToLoad();
  }

  /**
   * Clear search
   */
  async clearSearch(): Promise<void> {
    await this.searchInput.clear();
    await this.searchButton.click();
    await this.waitForFilesToLoad();
  }

  /**
   * Change sort order
   */
  async setSortBy(sortType: 'name' | 'size' | 'date'): Promise<void> {
    await this.sortSelect.selectOption(sortType);
    await this.page.waitForTimeout(500);
  }

  /**
   * Get current sort value
   */
  async getCurrentSort(): Promise<string> {
    return await this.sortSelect.getAttribute('value') || 'name';
  }

  /**
   * Click refresh button
   */
  async refresh(): Promise<void> {
    await this.refreshButton.click();
    await this.waitForFilesToLoad();
  }

  /**
   * Get current path
   */
  async getCurrentPath(): Promise<string> {
    return await this.currentPath.textContent() || '';
  }

  /**
   * Get file count
   */
  async getFileCount(): Promise<number> {
    return await this.fileItems.count();
  }

  /**
   * Get folder count (excluding parent dir)
   */
  async getFolderCount(): Promise<number> {
    const folders = this.getFolderItems();
    const count = await folders.count();
    // Exclude parent dir item if present
    const hasParent = await this.parentDirItem.count() > 0;
    return hasParent ? count - 1 : count;
  }

  /**
   * Get file count (files only)
   */
  async getFilesOnlyCount(): Promise<number> {
    return await this.getFileOnlyItems().count();
  }

  /**
   * Navigate to parent directory
   */
  async navigateToParent(): Promise<void> {
    if (await this.parentDirItem.isVisible()) {
      await this.parentDirItem.click();
      await this.page.waitForTimeout(500);
    }
  }

  /**
   * Check if file exists
   */
  async fileExists(path: string): Promise<boolean> {
    return await this.getFileItem(path).count() > 0;
  }

  /**
   * Get all file paths
   */
  async getAllFilePaths(): Promise<string[]> {
    const paths: string[] = [];
    const count = await this.fileItems.count();
    for (let i = 0; i < count; i++) {
      const path = await this.fileItems.nth(i).getAttribute('data-path');
      if (path) paths.push(path);
    }
    return paths;
  }

  /**
   * Wait for file to appear
   */
  async waitForFile(path: string, timeout: number = 5000): Promise<void> {
    await this.getFileItem(path).waitFor({ state: 'visible', timeout });
  }

  /**
   * Wait for file to disappear
   */
  async waitForFileToDisappear(path: string, timeout: number = 5000): Promise<void> {
    await this.getFileItem(path).waitFor({ state: 'hidden', timeout });
  }

  /**
   * Drag file to folder
   */
  async dragFileToFolder(srcPath: string, dstFolderPath: string): Promise<void> {
    const src = this.getFileItem(srcPath);
    const dst = this.getFileItem(dstFolderPath);
    await src.dragTo(dst);
    await this.page.waitForTimeout(1000);
  }

  /**
   * Get current view (for compatibility with BasePage)
   */
  async getCurrentView(): Promise<string> {
    return 'files';
  }
}
