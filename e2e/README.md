# Syncweb E2E Tests

End-to-end tests using [Playwright](https://playwright.dev/) with Page Object Model (POM) architecture.

## Quick Start

```bash
# First time setup
make e2e-install

# Initialize test database/fixtures
make e2e-init

# Run all tests (headless)
make e2e

# Run with UI
cd e2e && npm run test:ui

# Debug tests
cd e2e && npm run test:debug

# View test report
cd e2e && npm run test:report
```

> **Important:** All `npx playwright` commands must be run from within the `e2e/` directory.
> The Playwright configuration and dependencies are located in `e2e/`, not the project root.

## Architecture

### Page Objects

Reusable page-specific logic in `e2e/pages/`:

| Page Object | Description |
|-------------|-------------|
| `BasePage` | Common functionality shared across all pages |
| `SidebarPage` | Sidebar navigation, folders, devices, mounts |
| `FilesPage` | File grid/list interactions and operations |
| `CompletionPage` | Sync completion monitoring view |

### CLI Testing

CLI E2E tests in `e2e/tests/cli-*.spec.ts` use `fixtures-cli.ts`:

| Fixture | Description |
|---------|-------------|
| `cli` | CliRunner instance for command execution |
| `tempDir` | Temporary directory for test files |
| `testHome` | Isolated home directory per test |
| `apiToken` | API token for authentication |
| `createDummyFile` | Helper to create test files |
| `createDummyDir` | Helper to create test directories |
| `runJson` | Run command and parse JSON output |
| `runAndVerify` | Run command and verify success |

### Web UI Test Fixtures

| Fixture | Description |
|---------|-------------|
| `server` | Running Syncweb server instance |
| `serverOptions` | Server configuration options |
| `sidebarPage` | SidebarPage instance |
| `filesPage` | FilesPage instance |
| `completionPage` | CompletionPage instance |

## Writing Tests

### Web UI Tests

```typescript
import { test, expect } from '../fixtures';

test.describe('My Feature', () => {
  test.use({ serverOptions: { verbose: false } });

  test('opens a folder', async ({ filesPage, sidebarPage, server }) => {
    await filesPage.goto(server.getBaseUrl());

    // Use sidebar POM to select a folder
    await sidebarPage.selectFolder('my-folder');

    // Use files POM to verify content
    await expect(filesPage.fileList).toBeVisible();
    await expect(filesPage.getCurrentPath()).toContain('my-folder');
  });
});
```

### CLI Tests

```typescript
import { test, expect } from '../fixtures-cli';

test.describe('cli-my-command', () => {
  test('my command works', async ({ cli, createDummyFile, runJson }) => {
    // Create test files
    createDummyFile('test.txt', 'content');

    // Run command and verify success
    const result = cli.runAndVerify(['ls']);
    expect(result.stdout).toContain('test.txt');

    // Or use runJson for structured output
    const files = await runJson(['ls']);
    expect(Array.isArray(files)).toBe(true);
  });
});
```

### Waiting Strategies

```typescript
// ✅ Wait for element
await element.waitFor({ state: 'visible' });

// ✅ Wait for API response
const [response] = await Promise.all([
  page.waitForResponse(resp => resp.url().includes('/api/syncweb/ls')),
  page.click('#refresh-button'),
]);

// ✅ Wait for condition
await page.waitForFunction(() => {
  const fileList = document.getElementById('file-list');
  return fileList && fileList.children.length > 0;
});
```

## Running Tests

```bash
# All tests
npx playwright test --project=desktop

# Specific file
npx playwright test tests/navigation-pom.spec.ts

# Pattern match
npx playwright test -g "sidebar"

# Headed mode (visible browser)
npx playwright test --headed

# Check for flakes (5 runs)
for i in 1 2 3 4 5; do npx playwright test --project=desktop; done
```

> **Note:** Always run `npx playwright` commands from the `e2e/` directory.

## Test Server

The E2E tests automatically start a Syncweb server instance:

- **Default port:** 8889 (or first available)
- **Home directory:** Temporary directory in `e2e/tmp/`
- **API token:** `e2e-test-token`
- **Lifecycle:** Server starts on first test, stops after all tests complete

### Custom Server Options

```typescript
test.use({
  serverOptions: {
    port: 9999,
    verbose: true,
    apiToken: 'custom-token',
  }
});
```

## Debugging

### Playwright Inspector
```bash
npm run test:debug
```

### Trace Viewer
```bash
npx playwright show-trace test-results/<test-name>/trace.zip
```

### VS Code
Install [Playwright Test extension](https://marketplace.visualstudio.com/items?itemName=ms-playwright.playwright)

## File Structure

```
e2e/
├── pages/
│   ├── base-page.ts         # Base page with common functionality
│   ├── sidebar-page.ts      # Sidebar navigation POM
│   ├── files-page.ts        # Files view POM
│   └── completion-page.ts   # Completion view POM
├── utils/
│   ├── test-server.ts       # Server management
│   └── cli-runner.ts        # CLI command runner
├── tests/
│   ├── navigation-pom.spec.ts   # Navigation tests (POM example)
│   ├── sidebar-pom.spec.ts      # Sidebar tests (POM example)
│   ├── cli-basic.spec.ts        # CLI basic commands
│   ├── cli-folders.spec.ts      # CLI folders commands
│   ├── cli-devices.spec.ts      # CLI devices commands
│   ├── cli-ls.spec.ts           # CLI ls commands
│   └── cli-find.spec.ts         # CLI find commands
├── fixtures.ts              # Web UI test fixtures
├── fixtures-cli.ts          # CLI test fixtures
├── global-setup.ts          # Global test setup
├── playwright.config.ts     # Playwright configuration
├── tsconfig.json            # TypeScript configuration
├── package.json             # Dependencies
└── README.md                # This file
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Server fails to start | `make build` - ensure binary exists |
| Tests timeout | Increase `timeout` in `playwright.config.ts` |
| Browser not found | `npx playwright install` |
| Port in use | `SYNCWEB_BASE_URL=http://localhost:8890 npx playwright test` |
| Permission errors | Check `SYNCWEB_HOME` directory permissions |

## CI/CD

Tests run on:
- Push to `main`
- Pull requests to `main`

Artifacts (screenshots, videos, traces) uploaded for failed tests.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SYNCWEB_BASE_URL` | Base URL for tests | `http://localhost:8889` |
| `SYNCWEB_HOME` | Syncweb home directory | Auto-generated temp dir |
| `SYNCWEB_API_TOKEN` | API token for auth | `e2e-test-token` |
| `DEBUG` | Enable debug logging | `false` |
| `CI` | CI mode (more retries, less workers) | `false` |
