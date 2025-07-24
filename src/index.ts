#!/usr/bin/env node
import detectPackageManager from './detect';
import { translateCommand } from './commands';
import { exec } from 'child_process';
import {promisify} from "util";

async function main() {
    const command = process.argv[2];
    const options = process.argv.slice(3);
    const execAsync = promisify(exec);

    const projectPackageManager = await detectPackageManager();

    if (projectPackageManager === 'unknown') {
        console.error("Error: Unable to determine the project's package manager.");
        //console.error("Checked for lockfiles: package-lock.json, yarn.lock, pnpm-lock.yaml, bun.lockb");
        console.error("Please ensure the project has a standard package manager configuration.");
        process.exit(1);
    }

    if (process.argv.includes('--dry-run')) {
        console.log(`Detected package manager: ${projectPackageManager}`);
        console.log(`Would execute: ${projectPackageManager} ${command} ${options.join(' ')}`);
        return;
    }

    if (command) {
        let translatedCommand = command;
        console.log(`Detected package manager: ${projectPackageManager}. Running ${command}`);

        if (projectPackageManager !== 'pnpm') {
            translatedCommand = translateCommand(command, projectPackageManager);
        }

       try {
            const { stdout, stderr } = await execAsync(`${projectPackageManager} ${translatedCommand} ${options.join(' ')}`);
            console.log(stdout);
            if (stderr) {
              console.error(stderr);
            }
          } catch (error : any) {
             console.error(`Execution failed: ${error.message}`);
             process.exit(1);
        }

    } else {
        console.log("No command provided.");
    }
}

main();
