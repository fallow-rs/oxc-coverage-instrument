//! Text setup for the standalone source API plus path-derived hashes.

use std::fmt::Write;

use oxc_coverage_types::FileCoverage;

/// Inputs to [`generate_preamble_source`].
pub struct PreambleInputs<'a> {
    pub coverage: &'a FileCoverage,
    pub coverage_json: &'a str,
    pub coverage_hash: &'a str,
    pub coverage_var: &'a str,
    pub coverage_name: &'a str,
    pub truthy_helper: Option<&'a str>,
    pub optional_chain_helper: Option<&'a str>,
}

/// Generate the fixed runtime setup as source text for the standalone API.
///
/// Host-owned AST instrumentation uses `runtime_setup` instead. Keeping this
/// source generator on the source-to-source path avoids constructing a large
/// setup AST that codegen immediately turns back into text.
pub fn generate_preamble_source(inputs: &PreambleInputs<'_>) -> String {
    let PreambleInputs {
        coverage,
        coverage_json,
        coverage_hash,
        coverage_var,
        coverage_name,
        truthy_helper,
        optional_chain_helper,
    } = *inputs;
    let mut buf = String::with_capacity(256 + coverage_json.len());
    let _ = write!(buf, "var {coverage_name} = (function () {{ var path = ");
    buf.push_str(
        &serde_json::to_string(&coverage.path).expect("serializing a String to JSON is infallible"),
    );
    let _ = write!(buf, "; var hash = ");
    buf.push_str(
        &serde_json::to_string(coverage_hash).expect("serializing a &str to JSON is infallible"),
    );
    let _ = write!(buf, "; var gcv = '{coverage_var}'; var coverageData = ");
    buf.push_str(coverage_json);
    if coverage.path == "__proto__" {
        let _ = writeln!(
            buf,
            "; coverageData.hash = hash; var coverage = typeof globalThis !== 'undefined' ? globalThis : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : this; if (!coverage[gcv]) {{ coverage[gcv] = {{}}; }} if (!Object.prototype.hasOwnProperty.call(coverage[gcv], path) || !coverage[gcv][path] || coverage[gcv][path].hash !== hash) {{ Object.defineProperty(coverage[gcv], path, {{ value: coverageData, enumerable: true, writable: true, configurable: true }}); }} var actualCoverage = coverage[gcv][path]; return actualCoverage; }})();"
        );
    } else {
        let _ = writeln!(
            buf,
            "; coverageData.hash = hash; var coverage = typeof globalThis !== 'undefined' ? globalThis : typeof global !== 'undefined' ? global : typeof self !== 'undefined' ? self : this; if (!coverage[gcv]) {{ coverage[gcv] = {{}}; }} if (!coverage[gcv][path] || coverage[gcv][path].hash !== hash) {{ coverage[gcv][path] = coverageData; }} var actualCoverage = coverage[gcv][path]; return actualCoverage; }})();"
        );
    }
    if let Some(name) = truthy_helper {
        append_logic_helper(&mut buf, name, coverage_name);
    }
    if let Some(name) = optional_chain_helper {
        append_optional_chain_helper(&mut buf, name, coverage_name);
    }
    buf
}

fn append_logic_helper(buf: &mut String, name: &str, coverage_name: &str) {
    let _ = writeln!(buf, "var {coverage_name}_temp;");
    let _ = writeln!(
        buf,
        "function {name}(val, id, idx) {{ {coverage_name}_temp = val; if ({coverage_name}_temp && (!Array.isArray({coverage_name}_temp) || {coverage_name}_temp.length) && (Object.getPrototypeOf({coverage_name}_temp) !== Object.prototype || Object.values({coverage_name}_temp).length)) {{ ++{coverage_name}.bT[id][idx]; }} return {coverage_name}_temp; }}"
    );
}

fn append_optional_chain_helper(buf: &mut String, name: &str, coverage_name: &str) {
    let _ = writeln!(
        buf,
        "function {name}(val, id) {{ ++{coverage_name}.b[id][val == null ? 0 : 1]; return val; }}"
    );
}

/// Stable DJB31 hex hash. Used for both the per-file coverage function name
/// and Istanbul's stale-cache guard hash on the embedded coverage object.
pub fn djb31_hex(input: &str) -> String {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    format!("{hash:x}")
}

/// Generate a deterministic coverage function name from the file path.
pub fn generate_cov_fn_name(file_path: &str) -> String {
    format!("cov_{}", djb31_hex(file_path))
}
