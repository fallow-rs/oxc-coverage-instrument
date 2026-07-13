import { defineConfig } from 'vitest/config';
import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest';

export default defineConfig({
  test: {
    coverage: {
      provider: 'istanbul',
      instrumenter: (options) =>
        createOxcInstrumenter({
          ...options,
          reportLogic: false,
          trackOptionalChainBranches: false,
          stripTypescript: false,
          experimentalDecorators: false,
          emitDecoratorMetadata: false,
          functionIdentityOverlay: false,
          nameCallbackArguments: false,
        }),
      include: ['src/**/*.ts'],
      exclude: ['**/*.test.ts'],
      reporter: ['json', 'text'],
      reportsDirectory: './coverage',
      reportOnFailure: true,
    },
  },
});
