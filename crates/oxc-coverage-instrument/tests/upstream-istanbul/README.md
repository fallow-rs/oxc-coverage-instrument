# Upstream Istanbul Specs

`scripts/istanbul-upstream-specs.mjs` vendors a focused subset of upstream
`istanbul-lib-instrument` runtime specs that cover statement-child containers
and ignore hints.

The copied cases come from `istanbuljs/istanbuljs` at commit
`28ffdbc314596bdcb3007e85d30a62372602b262`, under the upstream package's
BSD-3-Clause license.

Run it after building the N-API binding:

```sh
npm --prefix napi run build:debug
node scripts/istanbul-upstream-specs.mjs
```
