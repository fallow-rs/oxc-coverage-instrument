//! Rewriting a `FileCoverage`'s positions with a parsed source map, in both the
//! single-source and multi-source fan-out forms.

use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
};

use oxc_coverage_types::{BranchEntry, FileCoverage, FnEntry, Location};
use srcmap_sourcemap::SourceMap;

use crate::{
    context::{MappedLocation, RemapCaches, RemapContext},
    get_mapping::{direct_remap_location, get_mapped_location_cached, get_mapping_location_cached},
    merge::{empty_file_coverage, merge_file_coverage, numeric_id_order},
    options::RemapOptions,
    sources::resolve_source_path,
};

/// Remap `coverage` against a map whose sources all resolve to `path`, keeping
/// the input's metadata ids.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn apply_source_map_single(
    coverage: &FileCoverage,
    sm: &SourceMap,
    options: RemapOptions,
    path: String,
) -> FileCoverage {
    let mut output = coverage.clone();
    output.path = path;
    output.input_source_map = None;
    let mut caches = RemapCaches::default();
    let mut ctx = RemapContext::new(sm, &mut caches);

    if options.drop_unmapped {
        prune_single_source_unmapped(&mut output, &mut ctx);
    } else {
        for location in output.statement_map.values_mut() {
            remap_single_source_location(location, &mut ctx);
        }
        for function in output.fn_map.values_mut() {
            remap_single_source_location(&mut function.decl, &mut ctx);
            remap_single_source_location(&mut function.loc, &mut ctx);
            function.line = function.loc.start.line;
        }
        for branch in output.branch_map.values_mut() {
            remap_single_source_location(&mut branch.loc, &mut ctx);
            for arm in &mut branch.locations {
                remap_single_source_location(arm, &mut ctx);
            }
            branch.line = branch.loc.start.line;
        }
    }
    output.prune_orphan_counters();
    output
}

fn remap_single_source_location(location: &mut Location, ctx: &mut RemapContext<'_>) {
    if location.start.line != 0
        && location.end.line != 0
        && let Some(mapped) = get_mapping_location_cached(ctx, location)
    {
        *location = mapped;
        return;
    }
    direct_remap_location(location, ctx.sm);
}

fn try_remap_single_source_location(location: &mut Location, ctx: &mut RemapContext<'_>) -> bool {
    if location.start.line == 0 || location.end.line == 0 {
        direct_remap_location(location, ctx.sm);
        return true;
    }
    let Some(mapped) = get_mapping_location_cached(ctx, location) else {
        return false;
    };
    *location = mapped;
    true
}

fn prune_single_source_unmapped(coverage: &mut FileCoverage, ctx: &mut RemapContext<'_>) {
    let mut dropped = Vec::new();
    coverage.statement_map.retain(|id, location| {
        if try_remap_single_source_location(location, ctx) {
            true
        } else {
            dropped.push(id.clone());
            false
        }
    });
    for id in mem::take(&mut dropped) {
        coverage.s.remove(&id);
    }

    coverage.fn_map.retain(|id, function| {
        let decl_maps = try_remap_single_source_location(&mut function.decl, ctx);
        let loc_maps = try_remap_single_source_location(&mut function.loc, ctx);
        if decl_maps && loc_maps {
            function.line = function.loc.start.line;
            true
        } else {
            dropped.push(id.clone());
            false
        }
    });
    for id in mem::take(&mut dropped) {
        coverage.f.remove(&id);
        if let Some(overlay) = coverage.x_fallow_function_map.as_mut() {
            overlay.remove(&id);
        }
    }

    let mut surviving_arms = BTreeMap::new();
    coverage.branch_map.retain(|id, branch| {
        let umbrella_maps = try_remap_single_source_location(&mut branch.loc, ctx);
        let mut indices = Vec::new();
        let mut locations = Vec::new();
        for (index, arm) in branch.locations.iter_mut().enumerate() {
            if try_remap_single_source_location(arm, ctx) {
                indices.push(index);
                locations.push(arm.clone());
            }
        }
        if locations.is_empty() {
            dropped.push(id.clone());
            return false;
        }
        if !umbrella_maps {
            branch.loc.clone_from(&locations[0]);
        }
        branch.locations = locations;
        branch.line = branch.loc.start.line;
        surviving_arms.insert(id.clone(), indices);
        true
    });
    for id in dropped {
        coverage.b.remove(&id);
        if let Some(b_t) = coverage.b_t.as_mut() {
            b_t.remove(&id);
        }
    }
    project_branch_counters(&mut coverage.b, &surviving_arms);
    if let Some(b_t) = coverage.b_t.as_mut() {
        project_branch_counters(b_t, &surviving_arms);
    }
}

fn project_branch_counters(
    counters: &mut BTreeMap<String, Vec<u32>>,
    surviving_arms: &BTreeMap<String, Vec<usize>>,
) {
    for (id, indices) in surviving_arms {
        if let Some(hits) = counters.get_mut(id) {
            *hits = project_arm_counts(hits, indices);
        }
    }
}

/// Fan `coverage` out into one entry per original source, with metadata ids
/// renumbered contiguously per output file.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn apply_source_map_to_map(
    coverage: &FileCoverage,
    sm: &SourceMap,
    options: RemapOptions,
) -> Option<BTreeMap<String, FileCoverage>> {
    apply_source_map_to_map_internal(coverage, sm, options, true)
}

/// Fan-out with `canonicalize_ids` controlling whether output ids are
/// renumbered. The single-result callers keep the input ids so a caller reading
/// back one file still recognises its own keys.
#[expect(
    clippy::redundant_pub_crate,
    reason = "`pub(crate)` marks the API boundary; the module is private by construction"
)]
pub(crate) fn apply_source_map_to_map_internal(
    coverage: &FileCoverage,
    sm: &SourceMap,
    options: RemapOptions,
    canonicalize_ids: bool,
) -> Option<BTreeMap<String, FileCoverage>> {
    let resolved_sources: Vec<(u32, String)> = sm
        .sources
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            let index = u32::try_from(index).ok()?;
            resolve_source_path(sm, index).map(|path| (index, path))
        })
        .collect();
    if resolved_sources.is_empty() {
        return None;
    }

    let unique_paths: BTreeSet<&str> =
        resolved_sources.iter().map(|(_, path)| path.as_str()).collect();
    let fallback_source =
        (!options.drop_unmapped && unique_paths.len() == 1).then(|| resolved_sources[0].0);
    let mut outputs = BTreeMap::new();
    let mut caches = RemapCaches::default();
    let mut ctx = RemapContext::new(sm, &mut caches);

    fan_out_statements(&mut outputs, coverage, &mut ctx, fallback_source, canonicalize_ids);
    fan_out_functions(&mut outputs, coverage, &mut ctx, fallback_source, canonicalize_ids);
    fan_out_branches(&mut outputs, coverage, &mut ctx, fallback_source, canonicalize_ids);

    for output in outputs.values_mut() {
        if canonicalize_ids {
            let mut canonical = empty_file_coverage(output, output.path.clone());
            merge_file_coverage(&mut canonical, output);
            *output = canonical;
        } else {
            output.prune_orphan_counters();
        }
    }
    Some(outputs)
}

fn fan_out_statements(
    outputs: &mut BTreeMap<String, FileCoverage>,
    coverage: &FileCoverage,
    ctx: &mut RemapContext<'_>,
    fallback_source: Option<u32>,
    canonicalize_ids: bool,
) {
    for (key, location) in numeric_id_order(&coverage.statement_map) {
        let Some(mapped) = mapped_or_direct_location(location, ctx, fallback_source) else {
            continue;
        };
        let Some(output) = output_for_source(outputs, coverage, ctx.sm, mapped.source) else {
            continue;
        };
        let id =
            if canonicalize_ids { output.statement_map.len().to_string() } else { key.clone() };
        output.statement_map.insert(id.clone(), mapped.location);
        if let Some(hits) = coverage.s.get(key) {
            output.s.insert(id, *hits);
        }
    }
}

fn fan_out_functions(
    outputs: &mut BTreeMap<String, FileCoverage>,
    coverage: &FileCoverage,
    ctx: &mut RemapContext<'_>,
    fallback_source: Option<u32>,
    canonicalize_ids: bool,
) {
    for (key, function) in numeric_id_order(&coverage.fn_map) {
        let mapped_decl = mapped_or_direct_location(&function.decl, ctx, fallback_source);
        let mapped_loc = mapped_or_direct_location(&function.loc, ctx, fallback_source);
        let Some((source, decl, loc)) =
            matching_function_locations(function, mapped_decl, mapped_loc, ctx, fallback_source)
        else {
            continue;
        };
        let Some(output) = output_for_source(outputs, coverage, ctx.sm, source) else {
            continue;
        };
        let id = if canonicalize_ids { output.fn_map.len().to_string() } else { key.clone() };
        output.fn_map.insert(
            id.clone(),
            FnEntry { name: function.name.clone(), line: loc.start.line, decl, loc },
        );
        if let Some(hits) = coverage.f.get(key) {
            output.f.insert(id.clone(), *hits);
        }
        if let (Some(input_overlay), Some(output_overlay)) =
            (coverage.x_fallow_function_map.as_ref(), output.x_fallow_function_map.as_mut())
            && let Some(identity) = input_overlay.get(key)
        {
            output_overlay.insert(id, identity.clone());
        }
    }
}

fn fan_out_branches(
    outputs: &mut BTreeMap<String, FileCoverage>,
    coverage: &FileCoverage,
    ctx: &mut RemapContext<'_>,
    fallback_source: Option<u32>,
    canonicalize_ids: bool,
) {
    for (key, branch) in numeric_id_order(&coverage.branch_map) {
        let mapped_loc = mapped_or_direct_location(&branch.loc, ctx, fallback_source);
        let mut kept_indices = Vec::new();
        let mut locations = Vec::new();
        let mut branch_source = None;
        let mut first_mapped_arm = None;
        let mut source_mismatch = false;
        for (index, arm) in branch.locations.iter().enumerate() {
            if branch.branch_type == "if" && index > 0 && location_is_unknown(arm) {
                kept_indices.push(index);
                locations.push(arm.clone());
                continue;
            }
            let Some(mapped_arm) = mapped_or_direct_location(arm, ctx, fallback_source) else {
                continue;
            };
            if branch_source.is_some_and(|source| source != mapped_arm.source) {
                source_mismatch = true;
                break;
            }
            branch_source = Some(mapped_arm.source);
            first_mapped_arm.get_or_insert_with(|| mapped_arm.location.clone());
            kept_indices.push(index);
            locations.push(mapped_arm.location);
        }
        let Some(branch_source) = branch_source else {
            continue;
        };
        if source_mismatch || locations.is_empty() {
            continue;
        }
        let branch_loc = mapped_loc
            .map(|mapped| mapped.location)
            .or(first_mapped_arm)
            .expect("branch source comes from a mapped arm");
        let Some(output) = output_for_source(outputs, coverage, ctx.sm, branch_source) else {
            continue;
        };
        let id = if canonicalize_ids { output.branch_map.len().to_string() } else { key.clone() };
        output.branch_map.insert(
            id.clone(),
            BranchEntry {
                loc: branch_loc.clone(),
                line: branch_loc.start.line,
                branch_type: branch.branch_type.clone(),
                locations,
            },
        );
        if let Some(hits) = coverage.b.get(key) {
            output.b.insert(id.clone(), project_arm_counts(hits, &kept_indices));
        }
        if let (Some(input_b_t), Some(output_b_t)) = (coverage.b_t.as_ref(), output.b_t.as_mut())
            && let Some(hits) = input_b_t.get(key)
        {
            output_b_t.insert(id, project_arm_counts(hits, &kept_indices));
        }
    }
}

fn output_for_source<'a>(
    outputs: &'a mut BTreeMap<String, FileCoverage>,
    template: &FileCoverage,
    sm: &SourceMap,
    source: u32,
) -> Option<&'a mut FileCoverage> {
    let path = resolve_source_path(sm, source)?;
    Some(outputs.entry(path.clone()).or_insert_with(|| empty_file_coverage(template, path)))
}

fn mapped_or_direct_location(
    location: &Location,
    ctx: &mut RemapContext<'_>,
    fallback_source: Option<u32>,
) -> Option<MappedLocation> {
    if location.start.line != 0
        && location.end.line != 0
        && let Some(mapped) = get_mapped_location_cached(ctx, location)
    {
        return Some(mapped);
    }
    let source = fallback_source?;
    let mut location = location.clone();
    direct_remap_location(&mut location, ctx.sm);
    Some(MappedLocation { source, location })
}

fn matching_function_locations(
    function: &FnEntry,
    mapped_decl: Option<MappedLocation>,
    mapped_loc: Option<MappedLocation>,
    ctx: &RemapContext<'_>,
    fallback_source: Option<u32>,
) -> Option<(u32, Location, Location)> {
    match (mapped_decl, mapped_loc) {
        (Some(decl), Some(loc)) if decl.source == loc.source => {
            Some((decl.source, decl.location, loc.location))
        }
        _ => {
            let source = fallback_source?;
            let mut decl = function.decl.clone();
            let mut loc = function.loc.clone();
            direct_remap_location(&mut decl, ctx.sm);
            direct_remap_location(&mut loc, ctx.sm);
            Some((source, decl, loc))
        }
    }
}

fn project_arm_counts(hits: &[u32], indices: &[usize]) -> Vec<u32> {
    indices.iter().map(|index| hits.get(*index).copied().unwrap_or(0)).collect()
}

fn location_is_unknown(location: &Location) -> bool {
    location.start.line == 0
        && location.start.column == 0
        && location.end.line == 0
        && location.end.column == 0
}
