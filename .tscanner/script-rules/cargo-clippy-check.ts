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
    execSync('cargo clippy --message-format=json -- -D warnings', { stdio: 'pipe' });
  } catch (err: unknown) {
    const error = err as Error & { stdout?: unknown };
    const output = error.stdout?.toString() ?? '';
    const lines = output.split('\n');

    for (const line of lines) {
      if (!line.trim()) continue;

      try {
        const msg = JSON.parse(line);
        if (msg.reason === 'compiler-message' && msg.message) {
          const diagnostic = msg.message;
          if (diagnostic.level === 'error' || diagnostic.level === 'warning') {
            // eslint-disable-next-line @typescript-eslint/prefer-optional-chain
            const span = diagnostic.spans && diagnostic.spans.find((s: Record<string, unknown>) => s.is_primary);
            if (span) {
              const filePath = span.file_name;
              const absolutePath = path.resolve(filePath);

              addIssue(issues, {
                file: absolutePath,
                line: span.line_start,
                message: `[clippy] ${diagnostic.message}`,
              });
            }
          }
        }
      } catch (e) {
        // Ignore JSON parse errors for non-JSON lines
      }
    }
  }

  return issues;
});
