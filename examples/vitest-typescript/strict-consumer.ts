import { TraceMap, type EncodedSourceMap } from '@jridgewell/trace-mapping';
import { createOxcInstrumenter } from 'oxc-coverage-instrument/vitest';
import type { RawSourceMap } from 'source-map-js';

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

  const inputSourceMap: RawSourceMap = {
    version: '3',
    sources: ['input.ts'],
    names: [],
    mappings: '',
  };
  const code = instrumenter.instrumentSync(
    'export const value: number = 1;',
    'input.ts',
    inputSourceMap,
  );

  const sourceMapAfter = instrumenter.lastSourceMap();
  const coverageAfter = instrumenter.lastFileCoverage();
  const fileCoverageAfter = instrumenter.fileCoverage;

  if (sourceMapAfter !== null) {
    const version: 3 = sourceMapAfter.version;
    const mappings: string = sourceMapAfter.mappings;
    const encodedSourceMap: EncodedSourceMap = sourceMapAfter;
    const traceMap = new TraceMap(sourceMapAfter);
    void [version, mappings, encodedSourceMap, traceMap];
  }

  if (coverageAfter !== null) {
    const path: string = coverageAfter.path;
    const statementLine: number | undefined = coverageAfter.statementMap['0'].start.line;
    const functionName: string = coverageAfter.fnMap['0'].name;
    const branchType: string = coverageAfter.branchMap['0'].type;
    const statementHits: number = coverageAfter.s['0'];
    const functionHits: number = coverageAfter.f['0'];
    const branchHits: number[] = coverageAfter.b['0'];
    const identity = coverageAfter.x_fallow_functionMap?.['0'];

    if (identity !== undefined) {
      const identityId: string = identity.id;
      const identityName: string = identity.name;
      const identityPath: string = identity.path;
      const identityLine: number | undefined = identity.decl.start.line;
      void [identityId, identityName, identityPath, identityLine];
    }

    void [
      path,
      statementLine,
      functionName,
      branchType,
      statementHits,
      functionHits,
      branchHits,
    ];
  }

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
