#!/usr/bin/env npx tsx

// tscanner-ignore-next-line no-absolute-imports
import { type ScriptIssue, addIssue, runScript } from 'tscanner';
// tscanner-ignore-next-line no-absolute-imports
import { execSync } from 'child_process';
// tscanner-ignore-next-line no-absolute-imports
import * as path from 'path';

runScript((input) => {
  const issues: ScriptIssue[] = [];

  try {
    // Run cargo fmt --all -- --check to find formatting issues without modified files on disk
    execSync('cargo fmt --all -- --check', { stdio: 'pipe' });
  } catch (err: unknown) {
    const error = err as Error & { stderr?: unknown; stdout?: unknown };
    const stdout = (error.stdout as Buffer)?.toString() ?? '';
    const stderr = (error.stderr as Buffer)?.toString() ?? '';
    const output = `${stdout}\n${stderr}`;
    const lines = output.split('\n');

    for (const line of lines) {
      if (line.startsWith('Diff in ')) {
        let filePath = line.substring('Diff in '.length).trim();

        // Remove Windows long path prefix if present
        if (filePath.startsWith('\\\\?\\')) {
          filePath = filePath.substring(4);
        }

        // Normalize path to relative if possible, or keep absolute
        const absolutePath = path.resolve(filePath);

        // tscanner expects file paths to match what it provided in input.files
        // We'll add an issue for the first line of the file since cargo fmt doesn't provide line numbers
        addIssue(issues, {
          file: absolutePath,
          line: 1,
          message: 'File is not formatted correctly. Run `cargo fmt` to fix.',
        });
      }
    }
  }

  return issues;
});
