import * as path from 'path';
import * as fs from 'fs';
import { execSync, spawn, ChildProcess } from 'child_process';

/**
 * Options for CLI command execution
 */
export interface CliCommandOptions {
  /** Working directory for command execution */
  cwd?: string;
  /** Environment variables */
  env?: Record<string, string>;
  /** Timeout in milliseconds */
  timeout?: number;
  /** Suppress output */
  silent?: boolean;
}

/**
 * Result of CLI command execution
 */
export interface CliResult {
  /** Exit code */
  exitCode: number;
  /** Standard output */
  stdout: string;
  /** Standard error */
  stderr: string;
  /** Command that was executed */
  command: string;
  /** Duration in milliseconds */
  duration: number;
}

/**
 * CLI test runner for syncweb
 * Manages CLI command execution and test environment
 */
export class CliRunner {
  private binaryPath: string;
  private defaultHome: string;
  private defaultToken: string;

  constructor(options: { binaryPath?: string; home?: string; token?: string } = {}) {
    this.binaryPath = options.binaryPath || this.findBinary();
    this.defaultHome = options.home || this.createTempHome();
    this.defaultToken = options.token || 'e2e-test-token';
  }

  /**
   * Find the syncweb binary
   */
  private findBinary(): string {
    const projectRoot = path.join(__dirname, '../..');
    const binaryPath = path.join(projectRoot, 'syncweb');

    if (!fs.existsSync(binaryPath)) {
      throw new Error(
        `syncweb binary not found at ${binaryPath}. Run 'make build' first.`
      );
    }

    return binaryPath;
  }

  /**
   * Create a temporary home directory
   */
  private createTempHome(): string {
    const tmpDir = path.join(__dirname, '../tmp/cli-test');
    const testHome = path.join(tmpDir, `home-${process.pid}-${Date.now()}`);
    fs.mkdirSync(testHome, { recursive: true });
    return testHome;
  }

  /**
   * Run a CLI command and return result
   */
  run(args: string[], options: CliCommandOptions = {}): CliResult {
    const startTime = Date.now();
    const cwd = options.cwd || process.cwd();
    const env = {
      ...process.env,
      SYNCWEB_HOME: this.defaultHome,
      SYNCWEB_API_TOKEN: this.defaultToken,
      SYNCWEB_DEBUG: '1',
      ...options.env,
    };

    // Add --verbose flag for debug output
    const allArgs = [...args, '--verbose'];
    const command = `${this.binaryPath} ${allArgs.join(' ')}`;

    if (!options.silent) {
      console.log(`[CLI] $ ${command}`);
    }

    try {
      const stdout = execSync(command, {
        cwd,
        env,
        encoding: 'utf-8',
        timeout: options.timeout || 30000,
      });

      return {
        exitCode: 0,
        stdout: stdout.trim(),
        stderr: '',
        command,
        duration: Date.now() - startTime,
      };
    } catch (error: any) {
      return {
        exitCode: error.status || 1,
        stdout: error.stdout?.trim() || '',
        stderr: error.stderr?.trim() || '',
        command,
        duration: Date.now() - startTime,
      };
    }
  }

  /**
   * Run a CLI command and verify it succeeded
   */
  runAndVerify(args: string[], options: CliCommandOptions = {}): CliResult {
    const result = this.run(args, options);

    if (result.exitCode !== 0) {
      throw new Error(
        `Command failed with exit code ${result.exitCode}:\n${result.stderr}\n${result.stdout}`
      );
    }

    return result;
  }

  /**
   * Run a CLI command and parse JSON output
   */
  runJson<T = any>(args: string[], options: CliCommandOptions = {}): T {
    const result = this.run([...args, '--json'], options);

    if (result.exitCode !== 0) {
      throw new Error(
        `Command failed with exit code ${result.exitCode}:\n${result.stderr}`
      );
    }

    try {
      return JSON.parse(result.stdout);
    } catch (error) {
      throw new Error(`Failed to parse JSON output: ${result.stdout}`);
    }
  }

  /**
   * Start a long-running CLI command (e.g., serve, listen)
   */
  start(args: string[], options: CliCommandOptions = {}): ChildProcess {
    const cwd = options.cwd || process.cwd();
    const env = {
      ...process.env,
      SYNCWEB_HOME: this.defaultHome,
      SYNCWEB_API_TOKEN: this.defaultToken,
      ...options.env,
    };

    const child = spawn(this.binaryPath, args, {
      cwd,
      env,
      stdio: options.silent ? 'pipe' : 'inherit',
    });

    child.on('error', (err) => {
      console.error(`[CLI] Process error:`, err);
    });

    return child;
  }

  /**
   * Get the default home directory
   */
  getHome(): string {
    return this.defaultHome;
  }

  /**
   * Get the binary path
   */
  getBinaryPath(): string {
    return this.binaryPath;
  }

  /**
   * Set a custom home directory
   */
  setHome(home: string): void {
    this.defaultHome = home;
  }

  /**
   * Clean up temporary files
   */
  cleanup(): void {
    if (this.defaultHome && fs.existsSync(this.defaultHome)) {
      try {
        fs.rmSync(this.defaultHome, { recursive: true, force: true });
      } catch (e) {
        // Ignore cleanup errors
      }
    }
  }

  /**
   * Create a test file
   */
  createTestFile(relativePath: string, content: string = ''): string {
    const fullPath = path.join(this.defaultHome, relativePath);
    const dir = path.dirname(fullPath);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(fullPath, content);
    return fullPath;
  }

  /**
   * Create a test directory
   */
  createTestDir(relativePath: string): string {
    const fullPath = path.join(this.defaultHome, relativePath);
    fs.mkdirSync(fullPath, { recursive: true });
    return fullPath;
  }

  /**
   * Check if file exists
   */
  fileExists(relativePath: string): boolean {
    const fullPath = path.join(this.defaultHome, relativePath);
    return fs.existsSync(fullPath);
  }

  /**
   * Read file content
   */
  readFile(relativePath: string): string {
    const fullPath = path.join(this.defaultHome, relativePath);
    return fs.readFileSync(fullPath, 'utf-8');
  }

  /**
   * Delete file or directory
   */
  deletePath(relativePath: string): void {
    const fullPath = path.join(this.defaultHome, relativePath);
    if (fs.existsSync(fullPath)) {
      fs.rmSync(fullPath, { recursive: true, force: true });
    }
  }

  /**
   * List directory contents
   */
  listDir(relativePath: string): string[] {
    const fullPath = path.join(this.defaultHome, relativePath);
    return fs.readdirSync(fullPath);
  }
}
