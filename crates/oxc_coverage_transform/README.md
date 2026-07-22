# Oxc coverage transform prototype

This unpublished crate is the review vehicle for an Oxc-owned coverage
transform kernel. Its default build defines the proposed behavior and public
API. The branch itself is not an upstream merge target.

## Proposed for Oxc

The default-feature build owns only:

- ignore pragma association,
- statement, function, and branch discovery,
- AST counter mutation,
- collision-safe generated bindings,
- `Scoping` updates,
- generated helper names,
- neutral ordered metadata using Oxc spans.

Verify that boundary with:

```bash
cargo check -p oxc_coverage_transform --no-default-features
```

The upstream port should retain this default surface and remove every
`satellite-eager-compose` block plus the resulting compatibility scaffolding.
That is a mechanical extraction, not an open ownership decision.

## Not proposed for Oxc

The `satellite-eager-compose` feature is a private compatibility bridge for the
standalone instrumenter's eager source-map folding. It is deliberately absent
from the default build and must remain in the satellite package.

Runtime setup, Istanbul conversion and serialization, source-map composition,
V8 conversion, reporters, CLI, N-API, WASI, browser packaging, and Vitest
integration also remain outside this crate.

See [the full proposal](../../docs/OXC_KERNEL_PROPOSAL.md) for the host contract,
integration constraints, ownership boundary, and open maintainer decisions.
