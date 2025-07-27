export function translateCommand(command: string, packageManager: string): string {
  switch (packageManager) {
    case 'npm':
      switch (command) {
        case 'add':
          return 'install';
        case 'remove':
          return 'uninstall';
        default:
          return command;
      }
    case 'yarn':
      // Add more complex translations as needed
      return command; // Assume most commands are the same
    case 'bun':
          return command;
     default:
      return command;
  }
}
