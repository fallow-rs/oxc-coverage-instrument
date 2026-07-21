# Oxc coverage transform kernel proposal

Status: local prototype plus proposal for maintainer review. The unpublished
workspace crate proves dependency isolation and preserves conformance, but its
name, repository placement, metadata convention, and API are not accepted.

## Goal

Move the reusable AST mutation primitive into Oxc while keeping Istanbul
compatibility, source-map composition, runtime packaging, and reporting in this
workspace. Rolldown and Vitest should be able to schedule coverage inside an
existing Oxc pipeline without parsing or generating code a second time.

Provisional name: `oxc_coverage_transform`. The preferred home is an Oxc
workspace crate beside other reusable transforms, not inside the standalone
transformer facade. Oxc maintainers should choose the final name and placement.

## Responsibility boundary

| Responsibility | Oxc kernel | Satellite workspace |
|:--|:--:|:--:|
| Counter discovery and AST mutation | yes | adapter only |
| Istanbul ignore pragma semantics | yes | conformance authority |
| Statement, function, and branch records | neutral records | Istanbul conversion |
| Scope-safe generated bindings | yes | no duplicate implementation |
| Parse, lower, semantic build, and codegen | host pipeline | standalone convenience API |
| Runtime setup AST | shared contract or host adapter | proven adapter implementation |
| Input and output source-map composition | no | yes |
| V8 range conversion | no | yes |
| Reports and report tree | no | yes |
| CLI, N-API, WASI, and Vitest adapter | no | yes |
| Fallow function identities | no | optional post-transform overlay |

Reporters, remapping, V8 conversion, Istanbul serialization, and Fallow-specific
identities should not move into Oxc core. Runtime setup is the one unresolved
boundary: Oxc must either own a neutral setup builder or expose enough generated
binding metadata for a host adapter to add setup in the same pass.

## Proposed host contract

The host supplies:

- an `Allocator` that owns every inserted node,
- a mutable `Program`, after any syntax lowering required by the host,
- the matching `Scoping` and a parsed pragma map,
- typed transform options.

The transform mutates the program and returns:

- updated `Scoping`, valid for immediate downstream transforms and codegen,
- neutral statement, function, and branch records keyed by counter index,
- generated helper names and optional helper usage,
- handled and unhandled ignore-directive information.

The raw prototype kernel returns counter mutation, metadata, generated names,
and updated scoping. The satellite `instrument_program` adapter then adds the
runtime setup as AST and registers its scopes, symbols, and references directly.
Its final program therefore contains counters and setup without source-text
insertion, generated-code parsing, or semantic rebuilding.

The kernel owns symbols, references, child scopes, helper name reservation, and
all counter nodes. The setup adapter currently owns the equivalent invariants
for setup nodes. The host owns the root program, allocator, comments, source
text, original spans, and transform scheduling. Generated nodes use Oxc's
established generated-span convention. Original coverage locations remain Oxc
spans until the satellite converts them to Istanbul UTF-16 positions.

## Options

Stable transform semantics belong in the kernel:

- statement, function, branch, and logical-expression selection,
- ignore directive association,
- class method exclusion,
- callback argument naming,
- Istanbul-compatible counter ordering where required,
- safe generated-name collision handling.

Satellite policy stays outside the kernel:

- source-map parsing, remapping, and composition,
- Istanbul JSON serialization and embedded source content,
- Fallow function identity overlays,
- parser, lowering, and codegen options,
- report, binding, and package configuration.

The default `transform_program` API has no source-map registration policy. The
prototype retains a hidden compatibility extension because the satellite's
eager composition mode must decide keep/drop and canonical counter identities
before counter ids are embedded in the AST. That extension is not proposed for
the first upstream API. It should remain satellite-only or be redesigned only
after a real host integration proves that the kernel must own it.

## Prototype status and upstream target

| Concern | Current prototype | Recommended first upstream surface | Status |
|:--|:--|:--|:--|
| Coverage metadata | ordered Oxc spans and typed records | same | settled locally |
| Runtime setup | satellite AST adapter with complete scoping | generated names plus a shared builder or host adapter | maintainer decision |
| Registration policy | hidden satellite compatibility extension | omit | revisit only with integration evidence |
| Host placement | lowered `Program` plus matching `Scoping` | exact post-lowering Rolldown insertion point | integration proof required |
| Repository ownership | broader suite remains separate | upstream only the kernel | separate governance decision |

## Neutral metadata

The prototype returns ordered Oxc byte spans with typed functions and branches.
It contains no Istanbul maps, `serde_json::Value`, source-map JSON, hashes, or
reporter types. UTF-16 conversion now happens only in the satellite adapter,
which builds `FileCoverage`, performs optional remapping, and adds the Fallow
overlay.

Counter order and record order are observable compatibility constraints. The
first adapter must prove strict Istanbul output and default-profile documented
deltas without changing traversal order.

## Dependency policy

The kernel may depend on the Oxc allocator, AST, span, syntax, semantic, and
traverse layers accepted by maintainers. It must not depend on parser, codegen,
transformer lowering, source-map crates, V8 conversion, reports, N-API, WASI,
SHA-256, or this workspace's umbrella crate.

The satellite remains pinned to a compatible Oxc release because public AST
types are version-coupled. Once upstream, the kernel follows Oxc's internal
versioning and release policy. The satellite updates the grouped Oxc dependency
set and keeps end-to-end conformance as its compatibility gate.

## Repository ownership

Moving the AST kernel into Oxc and transferring this repository to the Oxc
organization are separate decisions. The technical recommendation is to
upstream only the minimal transform first. This broader workspace remains the
satellite, conformance authority, distribution surface, and reporter suite
unless Oxc maintainers separately choose to own that maintenance and release
scope.

The kernel proposal therefore does not imply ownership transfer of the CLI,
N-API and WASI packages, source-map and V8 conversion, coverage types, or
reporters.

## Performance evidence and non-claim

The transform-only benchmark excludes parsing and semantic construction and is
now a working CodSpeed shard. It can catch kernel regressions, but it does not
prove the end-to-end benefit Boshen described for Rolldown and Vitest.

The AST-native setup on this prototype branch is also not a standalone package
speedup over the optimized setup path on `main`. It exists here to prove a
semantically complete host-owned AST contract. No package performance claim
should be made from this branch.

Before upstreaming, benchmark the same real-world modules through both complete
pipelines: the released standalone adapter, and an exact Rolldown integration
that reuses its existing parse, lowering, semantic, and codegen phases. Report
the coverage traversal separately from the phases the host avoids duplicating.

## Adoption sequence

1. Oxc maintainers accept placement, naming, host ownership, and the minimal API.
2. Extract the traversal mechanically behind neutral Oxc-span records. Proven locally.
3. Make setup insertion AST-native with valid scoping and remove text replacement. Proven locally.
4. Prove transform-only performance, dependency isolation, and conformance. Gated locally and in CI.
5. Port the kernel to Oxc and test this workspace against that exact revision.
6. Prove the end-to-end Rolldown and Vitest integration benefit on real modules.
7. Replace the temporary local implementation and remove duplicate traversal code.

The first upstream API should be the smallest surface Rolldown needs. Standalone
parsing, code generation, compatibility helpers, and package ownership are not
prerequisites for direct Vitest integration.

## Rolldown placement constraint

The currently inspected Rolldown `transform_ast` hook receives an `EcmaAst`
before semantic construction and TypeScript lowering. This prototype requires
a lowered `Program` plus its matching `Scoping`. It is therefore not a drop-in
hook implementation. Upstream work must first prove one of these placements:

1. schedule coverage after lowering at a point that can hand over `Scoping`, or
2. let coverage participate in the host's semantic-building transform phase.

Adding a second parser or semantic build inside the hook would defeat the
integration goal and is not an acceptable fallback.

## Prototype packaging constraint

The published instrument crate currently depends on the unpublished local
`oxc_coverage_transform` crate. This branch is intentionally not releaseable:
`cargo package` must fail until the kernel is replaced by an accepted upstream
crate, vendored back into the published crate, or otherwise given a valid
publication topology. The Oxc extraction branch must not merge into the release
line while that constraint remains.

## Decisions requested from Oxc maintainers

- crate or module placement and final name,
- whether repository ownership is considered separately from kernel ownership,
- whether setup insertion belongs in the kernel,
- ownership transfer rules for `Scoping`, symbols, and references,
- generated-span and comment conventions,
- whether the registration policy belongs in the first API,
- which Istanbul ordering guarantees Oxc is willing to treat as stable.
