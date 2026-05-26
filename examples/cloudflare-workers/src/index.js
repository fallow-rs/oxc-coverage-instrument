import { fixtureFilename, fixtureSource } from './fixture.js';

globalThis.OXC_COVERAGE_FORCE_WASI_SINGLE_THREADED = true;

export default {
  async fetch() {
    try {
      if (typeof SharedArrayBuffer !== 'undefined') {
        Object.defineProperty(globalThis, 'SharedArrayBuffer', {
          configurable: true,
          value: undefined,
        });
      }

      const { instrument } = await import('oxc-coverage-instrument');
      const result = instrument(fixtureSource, fixtureFilename, { sourceMap: true });
      const fileCoverage = JSON.parse(result.coverageMap);

      return Response.json({
        ok: true,
        hasSharedArrayBuffer: typeof SharedArrayBuffer !== 'undefined',
        fileCoverage,
      });
    } catch (error) {
      return Response.json(
        {
          ok: false,
          error: error instanceof Error ? error.message : String(error),
        },
        { status: 500 },
      );
    }
  },
};
