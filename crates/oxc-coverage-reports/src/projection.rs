use oxc_coverage_types::{BranchEntry, FileCoverage};

pub fn aligned_branch_hits(
    file: &FileCoverage,
    id: &str,
    entry: &BranchEntry,
) -> Vec<u32> {
    let stored = file.b.get(id).map(Vec::as_slice).unwrap_or_default();
    (0..entry.locations.len())
        .map(|index| stored.get(index).copied().unwrap_or(0))
        .collect()
}

pub fn branch_counts(file: &FileCoverage) -> (u32, u32) {
    file.branch_map.iter().fold((0, 0), |(total, covered), (id, entry)| {
        let hits = aligned_branch_hits(file, id, entry);
        (
            total + hits.len() as u32,
            covered + hits.iter().filter(|&&value| value > 0).count() as u32,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_coverage_types::parse_coverage_map;

    const BRANCH_FIXTURE: &str = r#"{"a.js":{"path":"a.js","statementMap":{},"fnMap":{},"branchMap":{"0":{"loc":{"start":{"line":1,"column":0},"end":{"line":1,"column":3}},"line":1,"type":"if","locations":[{"start":{"line":1,"column":0},"end":{"line":1,"column":1}},{"start":{"line":1,"column":2},"end":{"line":1,"column":3}}]}},"s":{},"f":{},"b":{"0":[4,0,9]}}}"#;

    #[test]
    fn alignment_defaults_missing_arms_and_ignores_extra_arms() {
        let map = parse_coverage_map(BRANCH_FIXTURE).unwrap();
        let file = &map["a.js"];
        let entry = &file.branch_map["0"];
        assert_eq!(aligned_branch_hits(file, "0", entry), vec![4, 0]);
        assert_eq!(branch_counts(file), (2, 1));
    }
}
