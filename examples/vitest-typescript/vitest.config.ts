import { defineConfig } from 'vitest/config';
import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest';

export default defineConfig({
  test: {
    coverage: {
      provider: 'istanbul',
      instrumenter: (options) => createOxcInstrumenter(options),
      include: ['src/**/*.ts'],
      exclude: ['**/*.test.ts'],
      reporter: ['json', 'text'],
      reportsDirectory: './coverage',
      reportOnFailure: true,
    },
  },
});
