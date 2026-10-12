export interface FileStore {
  chooseDirectory(): Promise<string | null>;
  chooseFile(extensions: string[]): Promise<string | null>;
  readText(path: string): Promise<string>;
  writeTextAtomically(input: { path: string; contents: string; backupLabel: string }): Promise<void>;
}

export interface SecureSecretStore {
  save(reference: string, secret: string): Promise<void>;
  has(reference: string): Promise<boolean>;
  delete(reference: string): Promise<void>;
}

export interface ProcessRunner {
  run(input: { command: string; args: string[]; cwd?: string }): Promise<{ exitCode: number; stdout: string; stderr: string }>;
}
