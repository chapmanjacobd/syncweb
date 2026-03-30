import { test as base, expect } from '@playwright/test';
import { CliRunner, CliResult } from './utils/cli-runner';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Test fixtures for Syncweb CLI E2E tests
 * Extends Playwright's test with custom fixtures for CLI testing
 */
export const test = base.extend<{
  cli: CliRunner;
  tempDir: string;
  testHome: string;
  apiToken: string;
  createDummyFile: (name: string, content?: string) => string;
  createDummyDir: (name: string) => string;
  runJson: (args: string[]) => Promise<any>;
  runAndVerify: (args: string[]) => Promise<CliResult>;
}>({
  apiToken: 'e2e-test-token',

  testHome: async ({}, use) => {
    const tmpDir = path.join(__dirname, '../tmp/cli-test');
    fs.mkdirSync(tmpDir, { recursive: true });
    const testHome = path.join(tmpDir, `home-${process.pid}-${Date.now()}`);
    fs.mkdirSync(testHome, { recursive: true });

    await use(testHome);

    // Cleanup after test
    if (fs.existsSync(testHome)) {
      try {
        fs.rmSync(testHome, { recursive: true, force: true });
      } catch (e) {
        // Ignore cleanup errors
      }
    }
  },

  tempDir: async ({ testHome }, use) => {
    // tempDir is a subdirectory for test-specific files
    const tempDir = path.join(testHome, 'temp');
    fs.mkdirSync(tempDir, { recursive: true });
    await use(tempDir);
  },

  cli: async ({ testHome, apiToken }, use) => {
    const cli = new CliRunner({
      home: testHome,
      token: apiToken,
    });

    await use(cli);

    // Cleanup
    cli.cleanup();
  },

  createDummyFile: async ({ testHome, cli }, use) => {
    const createFile = (name: string, content: string = ''): string => {
      const fullPath = path.join(testHome, name);
      const dir = path.dirname(fullPath);
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(fullPath, content);
      
      // Trigger scan after file creation to ensure Syncthing indexes it
      cli.run(['scan'], { silent: true });
      
      return name;
    };

    await use(createFile);
  },

  createDummyDir: async ({ testHome, cli }, use) => {
    const createDir = (name: string): string => {
      const fullPath = path.join(testHome, name);
      fs.mkdirSync(fullPath, { recursive: true });
      
      // Trigger scan after directory creation
      cli.run(['scan'], { silent: true });
      
      return name;
    };

    await use(createDir);
  },

  runJson: async ({ cli }, use) => {
    const runJson = async (args: string[]): Promise<any> => {
      const result = cli.run([...args, '--json'], { silent: true });
      if (result.exitCode !== 0) {
        throw new Error(`Command failed: ${result.stderr}`);
      }
      return JSON.parse(result.stdout);
    };

    await use(runJson);
  },

  runAndVerify: async ({ cli }, use) => {
    const runAndVerify = async (args: string[]): Promise<CliResult> => {
      return cli.runAndVerify(args, { silent: true });
    };

    await use(runAndVerify);
  },
});

export { expect };
