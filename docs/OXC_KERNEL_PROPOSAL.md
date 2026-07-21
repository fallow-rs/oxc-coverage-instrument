# Oxc coverage transform kernel proposal

Status: proposal for maintainer review. This document does not assume that the
crate name, repository placement, or API has been accepted.

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
| Input and output source-map composition | no | yes |
| V8 range conversion | no | yes |
| Reports and report tree | no | yes |
| CLI, N-API, WASI, and Vitest adapter | no | yes |
| Fallow function identities | no | optional post-transform overlay |

Reporters, remapping, V8 conversion, runtime bindings, and Fallow-specific
identities should not move into Oxc core.

## Proposed host contract

The host supplies:

- an `Allocator` that owns every inserted node,
- a mutable `Program`, after any syntax lowering required by the host,
- the matching `Scoping`, source text, and source type,
- typed transform options,
- an optional location predicate for generated-to-original mapping policy.

The transform mutates the program and returns:

- updated `Scoping`, valid for immediate downstream transforms and codegen,
- neutral statement, function, and branch records keyed by counter index,
- generated helper names and optional helper usage,
- handled and unhandled ignore-directive information.

The program must contain both counter expressions and runtime setup when the
call returns. Setup follows the directive prologue. A host must not insert
source text, reparse generated code, or rebuild semantic state.

The kernel owns symbols, references, child scopes, helper name reservation, and
all nodes that it inserts. The host owns the root program, allocator, comments,
source text, original spans, and transform scheduling. Generated setup nodes use
Oxc's established generated-span convention. Original coverage locations remain
Oxc spans until the satellite converts them to Istanbul UTF-16 positions.

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

The location predicate is intentionally an interface, not a dependency on this
workspace's source-map crate. It lets a host suppress unmapped counters without
pulling remapping code into the transform.

## Neutral metadata

Records should use typed counter indices, Oxc spans, optional function names,
branch kind, and ordered arm spans. They should not contain `serde_json::Value`,
Istanbul maps, source-map JSON, hashes, or reporter types. The satellite adapter
converts spans to Istanbul locations, builds `FileCoverage`, performs optional
remapping, and adds the Fallow overlay.

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

## Adoption sequence

1. Oxc maintainers accept placement, naming, host ownership, and the minimal API.
2. Extract the traversal mechanically behind neutral records, without redesigning setup.
3. Make setup insertion AST-native with valid scoping and remove text replacement.
4. Prove transform-only performance, dependency isolation, and conformance.
5. Port the kernel to Oxc and test this workspace against that exact revision.
6. Replace the temporary local implementation and remove duplicate traversal code.

The first upstream API should be the smallest surface Rolldown needs. Standalone
parsing, code generation, compatibility helpers, and package ownership are not
prerequisites for direct Vitest integration.

## Decisions requested from Oxc maintainers

- crate or module placement and final name,
- whether setup insertion belongs in the kernel,
- ownership transfer rules for `Scoping`, symbols, and references,
- generated-span and comment conventions,
- whether the location predicate belongs in the first API,
- which Istanbul ordering guarantees Oxc is willing to treat as stable.
