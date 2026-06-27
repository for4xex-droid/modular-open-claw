/**
 * Aiome Code Mode Host API
 * 
 * This file defines the TypeScript interface available to Javascript scripts
 * running within the WasmSkillManager isolate.
 */

declare namespace aiome {
  /**
   * Performs an OS-hardened shell execution (only if allow_shell_execution is granted).
   * @param command The shell command to run.
   */
  function exec(command: string): Promise<string>;

  /**
   * Performs a secure HTTP request.
   * @param method HTTP method (GET, POST, etc.)
   * @param url Target URL (must match allowed_domains)
   * @param headers Optional request headers
   * @param body Optional request body
   */
  function fetch(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE',
    url: string,
    headers?: Record<string, string>,
    body?: string
  ): Promise<{ status: number; body: string }>;

  /**
   * Writes data securely to a file path within the isolated cell directory.
   * @param path Target filepath (must be within sandboxed directory)
   * @param content File content to write
   */
  function writeFile(path: string, content: string): Promise<void>;

  /**
   * Reads a file securely from a file path within the isolated cell directory.
   * @param path Target filepath
   */
  function readFile(path: string): Promise<string>;

  /**
   * Appends logs to the agent trajectory.
   * @param message Log entry content
   */
  function log(message: string): void;
}
