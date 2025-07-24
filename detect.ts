import { promises as fs } from 'fs';
import path from 'path';

type PackageManager = 'npm' | 'yarn' | 'pnpm' | 'bun' | 'none' | 'unknown';

async function detectPackageManager(): Promise<PackageManager> {
  const lockfileMap: { [key: string]: PackageManager } = {
    'package-lock.json': 'npm',
    'yarn.lock': 'yarn',
    'pnpm-lock.yaml': 'pnpm',
    'bun.lockb': 'bun',
  };

  try {
    const files = await fs.readdir(process.cwd());

    for (const file of files) {
      if (lockfileMap[file]) {
        return lockfileMap[file];
      }
    }

    const hasNodeModules = files.includes('node_modules');
    const hasPackageJson = files.includes('package.json');

    if (!hasNodeModules && !hasPackageJson) {
      return 'none';
    }

    return 'unknown'; // Unable to determine reliably
  } catch (error) {
    console.error("Error detecting package manager:", error);
    return 'unknown';
  }
}

export default detectPackageManager;
