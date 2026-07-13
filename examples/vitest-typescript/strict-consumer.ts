import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest';

type IsAny<Value> = 0 extends 1 & Value ? true : false;
type AssertNotAny<Value> = IsAny<Value> extends true ? never : true;

export const checkVitestDeclaration = (): void => {
  const instrumenter = createOxcInstrumenter({
    coverageVariable: '__STRICT_CONSUMER_COVERAGE__',
    ignoreClassMethods: ['ignored'],
    reportLogic: false,
    trackOptionalChainBranches: false,
    stripTypescript: false,
    experimentalDecorators: false,
    emitDecoratorMetadata: false,
    functionIdentityOverlay: false,
    nameCallbackArguments: false,
  });

  const sourceMapIsTyped: AssertNotAny<ReturnType<typeof instrumenter.lastSourceMap>> = true;
  const lastCoverageIsTyped: AssertNotAny<
    ReturnType<typeof instrumenter.lastFileCoverage>
  > = true;
  const fileCoverageIsTyped: AssertNotAny<typeof instrumenter.fileCoverage> = true;

  const sourceMapBefore = instrumenter.lastSourceMap();
  const coverageBefore = instrumenter.lastFileCoverage();
  const fileCoverageBefore = instrumenter.fileCoverage;

  const code = instrumenter.instrumentSync('export const value: number = 1;', 'input.ts', {
    version: 3,
    sources: ['input.ts'],
    names: [],
    mappings: '',
  });

  const sourceMapAfter = instrumenter.lastSourceMap();
  const coverageAfter = instrumenter.lastFileCoverage();
  const fileCoverageAfter = instrumenter.fileCoverage;

  void [
    sourceMapIsTyped,
    lastCoverageIsTyped,
    fileCoverageIsTyped,
    sourceMapBefore,
    coverageBefore,
    fileCoverageBefore,
    code,
    sourceMapAfter,
    coverageAfter,
    fileCoverageAfter,
  ];
};
