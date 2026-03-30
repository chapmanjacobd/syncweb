// Export test fixtures
export { test, expect } from './fixtures';
export { test as cliTest, expect as cliExpect } from './fixtures-cli';

// Export page objects
export { BasePage } from './pages/base-page';
export { SidebarPage } from './pages/sidebar-page';
export { FilesPage } from './pages/files-page';
export { CompletionPage } from './pages/completion-page';

// Export utilities
export { TestServer } from './utils/test-server';
export type { TestServerOptions } from './utils/test-server';
export { CliRunner } from './utils/cli-runner';
export type { CliCommandOptions, CliResult } from './utils/cli-runner';
