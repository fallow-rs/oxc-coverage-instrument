import { strict as assert } from 'node:assert'
import { instrument } from 'oxc-coverage-instrument'

const source = `
export function classify(n) {
  if (n < 0) {
    return 'negative'
  } else if (n === 0) {
    return 'zero'
  } else {
    return 'positive'
  }
}
`

const result = instrument(source, 'classify.js', { sourceMap: true })
const coverage = JSON.parse(result.coverageMap)

assert.ok(result.code.includes('__coverage__'), 'expected __coverage__ preamble in instrumented code')
assert.ok(Object.keys(coverage.statementMap).length > 0, 'expected at least one statement entry')
assert.ok(Object.keys(coverage.fnMap).length > 0, 'expected at least one function entry')
assert.ok(Object.keys(coverage.branchMap).length > 0, 'expected at least one branch entry')
assert.ok(typeof result.sourceMap === 'string', 'expected a source map when sourceMap: true')

const stmts = Object.keys(coverage.statementMap).length
const fns = Object.keys(coverage.fnMap).length
const branches = Object.keys(coverage.branchMap).length

const wasi = process.env.NAPI_RS_FORCE_WASI ? ' (wasm binding)' : ''
console.log(`smoke OK${wasi}: ${stmts} statements, ${fns} functions, ${branches} branch groups`)
