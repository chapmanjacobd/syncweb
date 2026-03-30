import * as path from 'path';
import * as fs from 'fs';
import * as http from 'http';

/**
 * Options for configuring the test server
 */
export interface TestServerOptions {
  /** Port to run the server on (default: find available port) */
  port?: number;
  /** Home directory for syncweb (default: temp directory) */
  homeDir?: string;
  /** API token for authentication (default: 'e2e-test-token') */
  apiToken?: string;
  /** Additional environment variables */
  env?: Record<string, string>;
  /** Verbose logging */
  verbose?: boolean;
  /** Public directory for web assets (default: embedded assets) */
  publicDir?: string;
}

/**
 * Test server wrapper for syncweb
 * Manages server lifecycle during E2E tests
 */
export class TestServer {
  private serverProcess: any = null;
  private port: number;
  private homeDir: string;
  private apiToken: string;
  private env: Record<string, string>;
  private verbose: boolean;
  private baseUrl: string;
  private publicDir?: string;

  constructor(options: TestServerOptions = {}) {
    this.port = options.port || this.findAvailablePort();
    this.homeDir = options.homeDir || this.createTempHome();
    this.apiToken = options.apiToken || 'e2e-test-token';
    this.env = options.env || {};
    this.verbose = options.verbose || false;
    this.publicDir = options.publicDir;
    this.baseUrl = `http://localhost:${this.port}`;
  }

  /**
   * Find an available port starting from 8889
   */
  private findAvailablePort(startPort: number = 8889): number {
    return new Promise<number>((resolve) => {
      const checkPort = (port: number) => {
        const server = http.createServer();
        server.listen(port, () => {
          server.close(() => resolve(port));
        });
        server.on('error', () => {
          checkPort(port + 1);
        });
      };
      checkPort(startPort);
    }) as unknown as number;
  }

  /**
   * Create a temporary home directory for syncweb
   */
  private createTempHome(): string {
    const tmpDir = path.join(__dirname, '../tmp');
    const testHome = path.join(tmpDir, `test-home-${process.pid}-${Date.now()}`);
    fs.mkdirSync(testHome, { recursive: true });
    return testHome;
  }

  /**
   * Start the syncweb server
   */
  async start(): Promise<void> {
    if (this.serverProcess) {
      throw new Error('Server already started');
    }

    // Build the binary if it doesn't exist
    const projectRoot = path.join(__dirname, '../..');
    const binaryPath = path.join(projectRoot, 'syncweb');

    if (!fs.existsSync(binaryPath)) {
      console.log('Building syncweb binary...');
      const { execSync } = require('child_process');
      try {
        execSync('make build', { cwd: projectRoot, stdio: 'inherit' });
      } catch (e) {
        throw new Error('Failed to build syncweb binary. Run `make build` first.');
      }
    }

    // Start the server
    const { spawn } = require('child_process');

    const serverEnv = {
      ...process.env,
      SYNCWEB_HOME: this.homeDir,
      SYNCWEB_API_TOKEN: this.apiToken,
      ...this.env,
    };

    this.serverProcess = spawn(binaryPath, ['serve', '--port', this.port.toString(), '--public-dir', path.resolve(__dirname, '../../web/dist')], {
      env: serverEnv,
      cwd: projectRoot,
      stdio: this.verbose ? 'inherit' : 'pipe',
    });

    this.serverProcess.on('error', (err: any) => {
      console.error('Server process error:', err);
    });

    if (this.verbose) {
      this.serverProcess.stdout?.on('data', (data: Buffer) => {
        console.log(`[syncweb] ${data.toString()}`);
      });
      this.serverProcess.stderr?.on('data', (data: Buffer) => {
        console.error(`[syncweb] ${data.toString()}`);
      });
    }

    // Wait for server to be ready
    await this.waitForServer();
  }

  /**
   * Wait for the server to be ready
   */
  private async waitForServer(timeout: number = 30000): Promise<void> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      try {
        await this.makeRequest('/api/syncweb/status');
        return;
      } catch (e) {
        await this.sleep(500);
      }
    }

    throw new Error(`Server failed to start within ${timeout}ms`);
  }

  /**
   * Make an HTTP request to check server health
   */
  private async makeRequest(endpoint: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const url = new URL(endpoint, this.baseUrl);
      const req = http.get(url.toString(), {
        headers: {
          'X-Syncweb-Token': this.apiToken,
        },
      }, (res) => {
        if (res.statusCode === 200) {
          resolve();
        } else {
          reject(new Error(`Status: ${res.statusCode}`));
        }
      });

      req.on('error', reject);
      req.setTimeout(5000, () => {
        req.destroy();
        reject(new Error('Timeout'));
      });
    });
  }

  /**
   * Sleep for a given duration
   */
  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Stop the syncweb server
   */
  async stop(): Promise<void> {
    if (!this.serverProcess) {
      return;
    }

    // Send SIGTERM for graceful shutdown
    this.serverProcess.kill('SIGTERM');

    // Wait for process to exit
    await new Promise<void>((resolve) => {
      const timeout = setTimeout(() => {
        if (this.serverProcess) {
          this.serverProcess.kill('SIGKILL');
        }
        resolve();
      }, 5000);

      this.serverProcess.on('exit', () => {
        clearTimeout(timeout);
        resolve();
      });
    });

    this.serverProcess = null;

    // Clean up temp home directory
    if (this.homeDir && fs.existsSync(this.homeDir)) {
      try {
        fs.rmSync(this.homeDir, { recursive: true, force: true });
      } catch (e) {
        // Ignore cleanup errors
      }
    }
  }

  /**
   * Get the base URL of the server
   */
  getBaseUrl(): string {
    return this.baseUrl;
  }

  /**
   * Get the API token
   */
  getApiToken(): string {
    return this.apiToken;
  }

  /**
   * Get the home directory
   */
  getHomeDir(): string {
    return this.homeDir;
  }

  /**
   * Get the port
   */
  getPort(): number {
    return this.port;
  }
}
