use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use sddk_domain::{Requirement, WorkflowManifest};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::docs::{GENERATED_WORKFLOW_DOC, render_workflow_docs};
use crate::inventory::{GENERATED_INVENTORY_DOC, render_inventory};

const WORKFLOW_MANIFEST: &str = "workflow/workflow.yaml";
const BROKEN_REFERENCE: &str = "SDDK001";
const UNRESOLVED_PLACEHOLDER: &str = "SDDK002";
const QUOTED_TILDE: &str = "SDDK003";
const UNDEFINED_SHELL_VARIABLE: &str = "SDDK004";
const INVALID_CONTRACT: &str = "SDDK005";
const UNDECLARED_WORKFLOW_ITEM: &str = "SDDK006";
const ARTIFACT_TOPOLOGY: &str = "SDDK007";
const PATH_NOT_TRAVERSABLE: &str = "SDDK008";
const GENERATED_DOC_STALE: &str = "SDDK009";
const GENERATED_INVENTORY_STALE: &str = "SDDK010";
const AGENT_NOT_IN_REGISTRY: &str = "SDDK011";
const REGISTRY_ORPHAN: &str = "SDDK012";
const AGENT_NAME_MISMATCH: &str = "SDDK013";
const INVALID_PACK_MANIFEST: &str = "SDDK014";

/// Diagnostic severity. Only errors make `sddk lint` fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A repository contract is invalid and lint exits nonzero.
    Error,
    /// A non-fatal consistency gap should be addressed.
    Warning,
}

/// One stable, structured repository diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Error or warning severity.
    pub severity: Severity,
    /// Repository-relative file path using forward slashes.
    pub file: String,
    /// One-based source line when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Human-readable problem statement.
    pub message: String,
    /// Suggested remediation.
    pub hint: String,
}

/// Aggregate diagnostic counts included in JSON output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LintSummary {
    /// Number of error diagnostics.
    pub errors: usize,
    /// Number of warning diagnostics.
    pub warnings: usize,
}

/// Deterministically sorted result of linting one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintReport {
    /// Aggregate counts.
    pub summary: LintSummary,
    /// Sorted diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl LintReport {
    /// Returns true when at least one error diagnostic was emitted.
    pub fn has_errors(&self) -> bool {
        self.summary.errors > 0
    }

    /// Renders stable human-readable diagnostics.
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        for diagnostic in &self.diagnostics {
            let location = diagnostic.line.map_or_else(
                || diagnostic.file.clone(),
                |line| format!("{}:{line}", diagnostic.file),
            );
            output.push_str(&format!(
                "{}[{}] {}: {}\n  help: {}\n",
                match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                },
                diagnostic.code,
                location,
                diagnostic.message,
                diagnostic.hint
            ));
        }
        output.push_str(&format!(
            "lint: {} error(s), {} warning(s)\n",
            self.summary.errors, self.summary.warnings
        ));
        output
    }
}

/// Fatal failures that prevent repository linting from starting.
#[derive(Debug, Error)]
pub enum LintError {
    /// The supplied root is not a directory.
    #[error("repository root is not a directory: {0}")]
    InvalidRoot(PathBuf),
}

/// Lints workflow contracts, references, executable snippets, and generated docs.
pub fn lint_repository(root: impl AsRef<Path>) -> Result<LintReport, LintError> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(LintError::InvalidRoot(root.to_path_buf()));
    }

    let mut diagnostics = Vec::new();
    validate_schema_catalog(root, &mut diagnostics);
    let workflow = lint_workflow(root, &mut diagnostics);
    scan_repository_sources(root, &mut diagnostics);
    if let Some(manifest) = workflow.as_ref() {
        lint_generated_docs(root, manifest, &mut diagnostics);
    }
    lint_generated_inventory(root, &mut diagnostics);
    lint_agent_registry(root, &mut diagnostics);
    lint_pack_manifest(root, &mut diagnostics);

    diagnostics.sort_by(|left, right| {
        (
            left.severity,
            &left.code,
            &left.file,
            left.line,
            &left.message,
        )
            .cmp(&(
                right.severity,
                &right.code,
                &right.file,
                right.line,
                &right.message,
            ))
    });
    diagnostics.dedup();
    let summary = LintSummary {
        errors: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count(),
        warnings: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count(),
    };
    Ok(LintReport {
        summary,
        diagnostics,
    })
}

fn lint_workflow(root: &Path, diagnostics: &mut Vec<Diagnostic>) -> Option<WorkflowManifest> {
    let relative = Path::new(WORKFLOW_MANIFEST);
    let path = root.join(relative);
    // Non-intrusive: when the repo has no workflow/workflow.yaml, lint falls
    // back to the canonical manifest embedded in the binary (ADR-0011). A
    // project must never be required to carry framework files.
    let yaml = match fs::read_to_string(&path) {
        Ok(yaml) => yaml,
        Err(_) => crate::CANONICAL_WORKFLOW.to_owned(),
    };

    match serde_saphyr::from_str::<Value>(&yaml) {
        Ok(value) => validate_workflow_contract(relative, &yaml, &value, diagnostics),
        Err(error) => diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            relative,
            None,
            format!("workflow is not valid YAML: {error}"),
            "fix the YAML syntax before validating workflow semantics",
        )),
    }

    let load_result = match fs::read_to_string(&path) {
        Ok(_) => sddk_engine::load_workflow_path(&path),
        Err(_) => sddk_engine::load_workflow_str(crate::CANONICAL_WORKFLOW),
    };
    match load_result {
        Ok(manifest) => {
            lint_workflow_topology(relative, &yaml, &manifest, diagnostics);
            Some(manifest)
        }
        Err(error) => {
            let code = match &error {
                sddk_engine::WorkflowLoadError::Validation(
                    sddk_engine::WorkflowValidationError::UnknownArtifactRequirement { .. }
                    | sddk_engine::WorkflowValidationError::UnknownGateRequirement { .. },
                ) => UNDECLARED_WORKFLOW_ITEM,
                _ => INVALID_CONTRACT,
            };
            diagnostics.push(diagnostic(
                code,
                Severity::Error,
                relative,
                None,
                format!("engine rejected canonical workflow: {error}"),
                "make workflow.yaml satisfy the canonical schema and engine invariants",
            ));
            None
        }
    }
}

fn validate_workflow_contract(
    file: &Path,
    yaml: &str,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(root) = value.as_object() else {
        diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            file,
            Some(1),
            "workflow root must be a mapping",
            "use the object shape declared by schemas/workflow.schema.json",
        ));
        return;
    };

    check_object_keys(
        file,
        yaml,
        root,
        &[
            "schema_version",
            "workflow",
            "statuses",
            "phases",
            "paths",
            "policies",
            "transitions",
            "artifacts",
            "gates",
            "forge",
            "storage",
            "project_identity",
        ],
        &[
            "schema_version",
            "workflow",
            "statuses",
            "phases",
            "transitions",
        ],
        "workflow root",
        diagnostics,
    );

    if let Some(workflow) = root.get("workflow").and_then(Value::as_object) {
        check_object_keys(
            file,
            yaml,
            workflow,
            &["id", "version", "description"],
            &["id", "version", "description"],
            "workflow metadata",
            diagnostics,
        );
        if let Some(version) = workflow.get("version").and_then(Value::as_str)
            && !Regex::new(r"^\d+\.\d+\.\d+$")
                .expect("valid semantic-version regex")
                .is_match(version)
        {
            diagnostics.push(diagnostic(
                INVALID_CONTRACT,
                Severity::Error,
                file,
                line_of(yaml, version),
                format!("workflow version {version:?} is not MAJOR.MINOR.PATCH"),
                "use a numeric semantic version such as 1.2.3",
            ));
        }
    }

    if let Some(transitions) = root.get("transitions").and_then(Value::as_array) {
        for transition in transitions {
            let Some(transition) = transition.as_object() else {
                diagnostics.push(diagnostic(
                    INVALID_CONTRACT,
                    Severity::Error,
                    file,
                    None,
                    "each workflow transition must be a mapping",
                    "replace scalar transition entries with transition objects",
                ));
                continue;
            };
            check_object_keys(
                file,
                yaml,
                transition,
                &[
                    "id",
                    "from",
                    "to",
                    "requires",
                    "paths",
                    "produces",
                    "implementation_binding",
                    "on_failure",
                ],
                &["id", "to", "requires"],
                "transition",
                diagnostics,
            );
            for state_key in ["from", "to", "on_failure"] {
                if let Some(state) = transition.get(state_key).and_then(Value::as_object) {
                    check_object_keys(
                        file,
                        yaml,
                        state,
                        &["status", "phase"],
                        &["status"],
                        "state reference",
                        diagnostics,
                    );
                }
            }
            if let Some(requirements) = transition.get("requires").and_then(Value::as_array) {
                for requirement in requirements {
                    if requirement.is_string() {
                        continue;
                    }
                    let Some(requirement) = requirement.as_object() else {
                        diagnostics.push(diagnostic(
                            INVALID_CONTRACT,
                            Severity::Error,
                            file,
                            None,
                            "transition requirement must be a string or {kind, name} mapping",
                            "use a simple precondition string or a typed artifact/gate requirement",
                        ));
                        continue;
                    };
                    check_object_keys(
                        file,
                        yaml,
                        requirement,
                        &["kind", "name"],
                        &["kind", "name"],
                        "transition requirement",
                        diagnostics,
                    );
                }
            }
        }
    }

    if let Some(paths) = root.get("paths").and_then(Value::as_object) {
        for path in paths.values().filter_map(Value::as_object) {
            check_object_keys(
                file,
                yaml,
                path,
                &["description", "debt_verification", "phases"],
                &["description", "debt_verification", "phases"],
                "path",
                diagnostics,
            );
        }
    }
    if let Some(artifacts) = root.get("artifacts").and_then(Value::as_object) {
        for artifact in artifacts.values().filter_map(Value::as_object) {
            check_object_keys(
                file,
                yaml,
                artifact,
                &[
                    "producer",
                    "consumers",
                    "required",
                    "terminal",
                    "description",
                ],
                &["producer", "consumers"],
                "artifact",
                diagnostics,
            );
        }
    }
    if let Some(gates) = root.get("gates").and_then(Value::as_object) {
        for gate in gates.values().filter_map(Value::as_object) {
            check_object_keys(
                file,
                yaml,
                gate,
                &["gate_type", "description"],
                &[],
                "gate",
                diagnostics,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_object_keys(
    file: &Path,
    source: &str,
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    required: &[&str],
    context: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for key in object.keys().filter(|key| !allowed.contains(&key.as_str())) {
        diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            file,
            line_of(source, &format!("{key}:")),
            format!("unknown {context} field {key:?}"),
            "use only canonical snake_case wire fields declared by the schema",
        ));
    }
    for key in required.iter().filter(|key| !object.contains_key(**key)) {
        diagnostics.push(diagnostic(
            INVALID_CONTRACT,
            Severity::Error,
            file,
            None,
            format!("{context} is missing required field {key:?}"),
            "add the required canonical wire field",
        ));
    }
}

fn lint_workflow_topology(
    file: &Path,
    yaml: &str,
    manifest: &WorkflowManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for transition in &manifest.transitions {
        for produced in &transition.produces {
            if !manifest.artifacts.contains_key(produced) {
                diagnostics.push(diagnostic(
                    UNDECLARED_WORKFLOW_ITEM,
                    Severity::Error,
                    file,
                    line_of(yaml, &format!("- {produced}")),
                    format!(
                        "transition {} produces undeclared artifact {produced:?}",
                        transition.id
                    ),
                    "declare the artifact under artifacts or remove it from produces",
                ));
            }
        }
        for requirement in &transition.requires {
            if let Requirement::Structured { kind, name } = requirement {
                let declared = match kind.as_str() {
                    "artifact" => manifest.artifacts.contains_key(name),
                    "gate" => manifest.gates.contains_key(name),
                    _ => true,
                };
                if !declared {
                    diagnostics.push(diagnostic(
                        UNDECLARED_WORKFLOW_ITEM,
                        Severity::Error,
                        file,
                        line_of(yaml, &format!("name: {name}")),
                        format!(
                            "transition {} requires undeclared {kind} {name:?}",
                            transition.id
                        ),
                        format!("declare {name:?} under {kind}s or remove the requirement"),
                    ));
                }
            }
        }
    }

    let mut artifacts = manifest.artifacts.iter().collect::<Vec<_>>();
    artifacts.sort_by_key(|(name, _)| *name);
    for (name, artifact) in artifacts {
        if artifact.producer.trim().is_empty() {
            diagnostics.push(diagnostic(
                ARTIFACT_TOPOLOGY,
                Severity::Warning,
                file,
                line_of(yaml, &format!("{name}:")),
                format!("artifact {name:?} has no producer"),
                "name the phase, agent, runtime, or provider that produces this artifact",
            ));
        }
        if artifact.consumers.is_empty() && !artifact.terminal {
            diagnostics.push(diagnostic(
                ARTIFACT_TOPOLOGY,
                Severity::Warning,
                file,
                line_of(yaml, &format!("{name}:")),
                format!("artifact {name:?} has no declared consumers"),
                "declare at least one consumer or document why the terminal artifact is retained",
            ));
        }
    }

    let mut paths = manifest.paths.iter().collect::<Vec<_>>();
    paths.sort_by_key(|(name, _)| *name);
    for (path_name, path) in paths {
        let start_phase = manifest
            .transitions
            .iter()
            .find(|transition| {
                transition.from.is_none() && transition_applies_to_path(transition, path_name)
            })
            .and_then(|transition| transition.to.phase)
            .map(|phase| wire(&phase));
        if let (Some(start_phase), Some(first)) = (start_phase.as_ref(), path.phases.first())
            && first != start_phase
        {
            diagnostics.push(diagnostic(
                PATH_NOT_TRAVERSABLE,
                Severity::Warning,
                file,
                line_of(yaml, &format!("{path_name}:")),
                format!("path {path_name} starts at {first}, but cycle.start enters {start_phase}"),
                "declare a path-specific entry transition or align the first path phase",
            ));
        }
        let edges = manifest
            .transitions
            .iter()
            .filter(|transition| transition_applies_to_path(transition, path_name))
            .filter_map(|transition| {
                Some((
                    wire(&transition.from.as_ref()?.phase?),
                    wire(&transition.to.phase?),
                ))
            })
            .collect::<HashSet<_>>();
        for pair in path.phases.windows(2) {
            if !edges.contains(&(pair[0].clone(), pair[1].clone())) {
                diagnostics.push(diagnostic(
                    PATH_NOT_TRAVERSABLE,
                    Severity::Warning,
                    file,
                    line_of(yaml, &format!("{path_name}:")),
                    format!(
                        "path {path_name} cannot traverse {} -> {} through a declared transition",
                        pair[0], pair[1]
                    ),
                    "declare the missing transition edge or change the path phase sequence",
                ));
            }
        }
    }
}

fn transition_applies_to_path(transition: &sddk_domain::workflow::Transition, path: &str) -> bool {
    transition.paths.is_empty() || transition.paths.iter().any(|candidate| candidate == path)
}

fn validate_schema_catalog(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let schema_dir = root.join("schemas");
    let mut schemas = match fs::read_dir(&schema_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            diagnostics.push(diagnostic(
                INVALID_CONTRACT,
                Severity::Error,
                Path::new("schemas"),
                None,
                format!("cannot read canonical schema directory: {error}"),
                "create schemas/ and add the canonical JSON Schema contracts",
            ));
            return;
        }
    };
    schemas.sort();

    for path in schemas {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(diagnostic(
                    INVALID_CONTRACT,
                    Severity::Error,
                    relative,
                    None,
                    format!("cannot read schema: {error}"),
                    "make the schema readable",
                ));
                continue;
            }
        };
        let schema = match serde_json::from_str::<Value>(&source) {
            Ok(schema) => schema,
            Err(error) => {
                diagnostics.push(diagnostic(
                    INVALID_CONTRACT,
                    Severity::Error,
                    relative,
                    Some(error.line()),
                    format!("schema is not valid JSON: {error}"),
                    "fix JSON syntax before using this contract",
                ));
                continue;
            }
        };
        if !schema.is_object() {
            diagnostics.push(diagnostic(
                INVALID_CONTRACT,
                Severity::Error,
                relative,
                Some(1),
                "schema root must be an object",
                "use a JSON Schema object at the document root",
            ));
        }
        validate_local_schema_refs(root, relative, &schema, diagnostics);
    }
}

fn validate_local_schema_refs(
    root: &Path,
    schema_file: &Path,
    value: &Value,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str)
                && !reference.starts_with('#')
                && !reference.contains("://")
            {
                let reference_path = reference.split('#').next().unwrap_or(reference);
                let target = schema_file
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(reference_path);
                if !root.join(&target).is_file() {
                    diagnostics.push(diagnostic(
                        BROKEN_REFERENCE,
                        Severity::Error,
                        schema_file,
                        None,
                        format!("schema reference {reference:?} does not exist"),
                        "add the referenced schema or correct the relative $ref",
                    ));
                }
            }
            for nested in object.values() {
                validate_local_schema_refs(root, schema_file, nested, diagnostics);
            }
        }
        Value::Array(array) => {
            for nested in array {
                validate_local_schema_refs(root, schema_file, nested, diagnostics);
            }
        }
        _ => {}
    }
}

fn scan_repository_sources(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let patterns = SourcePatterns::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_source(entry.path()))
    {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        scan_references(root, relative, &source, &patterns, diagnostics);
        scan_shell_fences(relative, &source, &patterns, diagnostics);
    }
}

struct SourcePatterns {
    markdown_link: Regex,
    yaml_reference: Regex,
    shell_fence: Regex,
    placeholder: Regex,
    quoted_tilde: Regex,
    assignment: Regex,
    loop_variable: Regex,
    shell_variable: Regex,
}

impl SourcePatterns {
    fn new() -> Self {
        Self {
            markdown_link: Regex::new(r"\[[^\]]*\]\(([^)]+)\)")
                .expect("valid Markdown link regex"),
            yaml_reference: Regex::new(
                r#"(?m)^[ \t]*(agent|skill|plugin|agent_(?:path|ref)|skill_(?:path|ref)|plugin_(?:path|ref)|prompt_(?:path|ref)|path|file):[ \t]*["']?([^\s"'#]+)"#,
            )
            .expect("valid YAML reference regex"),
            // Legacy prose uses shell fences as templates. Requiring an execution marker keeps
            // these checks on snippets that claim to be directly runnable.
            shell_fence: Regex::new(
                r"(?ms)^```(?:bash|sh|shell)[ \t]+(?:executable|lint)[ \t]*\n(.*?)^```\s*$",
            )
            .expect("valid executable shell-fence regex"),
            placeholder: Regex::new(r"(^|[^$])\{([A-Za-z_][A-Za-z0-9_-]*)\}")
                .expect("valid placeholder regex"),
            quoted_tilde: Regex::new(r#"["']~(?:/|["'])"#).expect("valid tilde regex"),
            assignment: Regex::new(
                r"(?m)^\s*(?:export\s+|local\s+|readonly\s+)?([A-Za-z_][A-Za-z0-9_]*)=",
            )
            .expect("valid assignment regex"),
            loop_variable: Regex::new(r"(?m)^\s*for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\b")
                .expect("valid loop-variable regex"),
            shell_variable: Regex::new(
                r"\$(?:\{([A-Za-z_][A-Za-z0-9_]*)(?::[-+?=][^}]*)?\}|([A-Za-z_][A-Za-z0-9_]*))",
            )
            .expect("valid shell-variable regex"),
        }
    }
}

fn scan_references(
    root: &Path,
    file: &Path,
    source: &str,
    patterns: &SourcePatterns,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for capture in patterns.markdown_link.captures_iter(source) {
        let raw = capture[1].split_whitespace().next().unwrap_or_default();
        check_reference(
            root,
            file,
            source,
            raw,
            capture.get(1).map(|found| found.start()),
            None,
            true,
            diagnostics,
        );
    }
    for capture in patterns.yaml_reference.captures_iter(source) {
        check_reference(
            root,
            file,
            source,
            &capture[2],
            capture.get(2).map(|found| found.start()),
            Some(&capture[1]),
            matches!(&capture[1], "path" | "file"),
            diagnostics,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_reference(
    root: &Path,
    file: &Path,
    source: &str,
    raw: &str,
    offset: Option<usize>,
    kind: Option<&str>,
    allow_relative: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((candidate, target)) = reference_target(file, raw, kind, allow_relative) else {
        return;
    };
    let target = normalize_path(&target);
    if reference_exists(root, &target) {
        return;
    }
    diagnostics.push(diagnostic(
        BROKEN_REFERENCE,
        Severity::Error,
        file,
        offset.map(|offset| line_at(source, offset)),
        format!("explicit repository reference {candidate:?} does not exist"),
        format!(
            "create {} or correct the explicit reference",
            slash(&target)
        ),
    ));
}

fn reference_target(
    file: &Path,
    raw: &str,
    kind: Option<&str>,
    allow_relative: bool,
) -> Option<(String, PathBuf)> {
    let candidate = raw
        .trim_matches(|character: char| matches!(character, '<' | '>' | '"' | '\'' | ',' | ';'))
        .split('#')
        .next()
        .unwrap_or_default()
        .trim_end_matches(|character: char| character == ':' || character.is_ascii_digit())
        .trim_end_matches('/');
    if candidate.is_empty()
        || candidate.contains(char::is_whitespace)
        || candidate.contains(['*', '{', '}', '$'])
        || candidate.contains("://")
    {
        return None;
    }

    let owned_prefix = ["agents/", "skills/", "plugins/", "prompts/"]
        .iter()
        .any(|prefix| candidate.starts_with(prefix));
    let explicit_relative = candidate.starts_with("./") || candidate.starts_with("../");
    let target = if owned_prefix {
        PathBuf::from(candidate)
    } else if explicit_relative && allow_relative {
        file.parent()
            .unwrap_or_else(|| Path::new(""))
            .join(candidate)
    } else {
        match kind {
            Some("agent" | "agent_path" | "agent_ref") => PathBuf::from("agents").join(candidate),
            Some("skill" | "skill_path" | "skill_ref") => PathBuf::from("skills").join(candidate),
            Some("plugin" | "plugin_path" | "plugin_ref") => {
                PathBuf::from("plugins").join(candidate)
            }
            Some("prompt_path" | "prompt_ref") => PathBuf::from("prompts").join(candidate),
            _ => return None,
        }
    };
    Some((candidate.to_owned(), target))
}

fn reference_exists(root: &Path, target: &Path) -> bool {
    let full = root.join(target);
    full.exists()
        || full.with_extension("md").exists()
        || (full.is_dir() && full.join("SKILL.md").is_file())
}

fn scan_shell_fences(
    file: &Path,
    source: &str,
    patterns: &SourcePatterns,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for fence in patterns.shell_fence.captures_iter(source) {
        let Some(body_match) = fence.get(1) else {
            continue;
        };
        let body = body_match.as_str();
        for capture in patterns.placeholder.captures_iter(body) {
            let Some(found) = capture.get(0) else {
                continue;
            };
            let placeholder = capture.get(2).map_or("placeholder", |value| value.as_str());
            diagnostics.push(diagnostic(
                UNRESOLVED_PLACEHOLDER,
                Severity::Error,
                file,
                Some(line_at(source, body_match.start() + found.start())),
                format!("unresolved literal placeholder {{{placeholder}}} in shell snippet"),
                "replace the placeholder before execution or use a defined shell variable",
            ));
        }
        for found in patterns.quoted_tilde.find_iter(body) {
            diagnostics.push(diagnostic(
                QUOTED_TILDE,
                Severity::Error,
                file,
                Some(line_at(source, body_match.start() + found.start())),
                "quoted tilde will not expand in a shell path",
                "use $HOME, leave the tilde unquoted, or quote only the suffix",
            ));
        }
        scan_shell_variables(
            file,
            source,
            body_match.start(),
            body,
            patterns,
            diagnostics,
        );
    }
}

fn scan_shell_variables(
    file: &Path,
    source: &str,
    body_offset: usize,
    body: &str,
    patterns: &SourcePatterns,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut defined = patterns
        .assignment
        .captures_iter(body)
        .map(|capture| capture[1].to_owned())
        .collect::<BTreeSet<_>>();
    defined.extend(
        patterns
            .loop_variable
            .captures_iter(body)
            .map(|capture| capture[1].to_owned()),
    );
    let searchable = strip_single_quoted(body);
    let mut emitted = BTreeSet::new();
    for capture in patterns.shell_variable.captures_iter(&searchable) {
        let Some(found) = capture.get(0) else {
            continue;
        };
        let variable = capture
            .get(1)
            .or_else(|| capture.get(2))
            .map_or("", |value| value.as_str());
        if variable.is_empty()
            || defined.contains(variable)
            || variable
                .chars()
                .all(|character| !character.is_ascii_lowercase())
            || !emitted.insert(variable.to_owned())
        {
            continue;
        }
        diagnostics.push(diagnostic(
            UNDEFINED_SHELL_VARIABLE,
            Severity::Error,
            file,
            Some(line_at(source, body_offset + found.start())),
            format!("shell variable ${variable} is not defined in this executable snippet"),
            "assign the variable in the snippet or use an explicit environment contract",
        ));
    }
}

fn lint_generated_docs(
    root: &Path,
    manifest: &WorkflowManifest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected = render_workflow_docs(manifest);
    let path = root.join(GENERATED_WORKFLOW_DOC);
    if fs::read_to_string(&path).is_ok_and(|actual| actual == expected) {
        return;
    }
    diagnostics.push(diagnostic(
        GENERATED_DOC_STALE,
        Severity::Error,
        Path::new(GENERATED_WORKFLOW_DOC),
        None,
        "generated workflow documentation is missing or stale",
        "run `sddk generate docs --root .` and commit the result",
    ));
}

fn lint_generated_inventory(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let expected = match render_inventory(root) {
        Ok(expected) => expected,
        Err(error) => {
            diagnostics.push(diagnostic(
                GENERATED_INVENTORY_STALE,
                Severity::Error,
                Path::new(GENERATED_INVENTORY_DOC),
                None,
                format!("cannot render generated repository inventory: {error}"),
                "make repository agent and skill paths readable UTF-8 paths",
            ));
            return;
        }
    };
    let path = root.join(GENERATED_INVENTORY_DOC);
    if fs::read_to_string(&path).is_ok_and(|actual| actual == expected) {
        return;
    }
    diagnostics.push(diagnostic(
        GENERATED_INVENTORY_STALE,
        Severity::Error,
        Path::new(GENERATED_INVENTORY_DOC),
        None,
        "generated repository inventory is missing or stale",
        "run `sddk generate inventory --root .` and commit the result",
    ));
}

fn should_descend(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry.file_name().to_str(),
        Some(".git" | "target" | ".atl" | "node_modules" | ".venv" | "__pycache__")
    )
}

fn is_source(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if components
        .windows(2)
        .any(|pair| pair == ["tests", "fixtures"])
    {
        return false;
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name)
                if matches!(name.to_str(), Some(".git" | "target" | ".atl"))
        )
    }) {
        return false;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("md" | "yaml" | "yml")
    ) && !matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("zip" | "gz" | "tgz" | "tar" | "7z" | "rar" | "exe" | "dll" | "so" | "a")
    )
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => return path.to_path_buf(),
        }
    }
    normalized
}

fn strip_single_quoted(value: &str) -> String {
    let mut quoted = false;
    value
        .chars()
        .map(|character| {
            if character == '\'' {
                quoted = !quoted;
                ' '
            } else if quoted {
                ' '
            } else {
                character
            }
        })
        .collect()
}

fn diagnostic(
    code: &str,
    severity: Severity,
    file: &Path,
    line: Option<usize>,
    message: impl Into<String>,
    hint: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity,
        file: slash(file),
        line,
        message: message.into(),
        hint: hint.into(),
    }
}

fn line_of(source: &str, needle: &str) -> Option<usize> {
    source.find(needle).map(|offset| line_at(source, offset))
}

fn line_at(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn wire<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("workflow enums are serializable")
        .as_str()
        .expect("workflow enums serialize as strings")
        .to_owned()
}

fn lint_agent_registry(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let registry_path = root.join("permissions.yaml");
    let policy = match sddk_gateway::PermissionPolicy::from_file(&registry_path) {
        Ok(policy) => policy,
        Err(error) => {
            diagnostics.push(diagnostic(
                AGENT_NOT_IN_REGISTRY,
                Severity::Error,
                Path::new("permissions.yaml"),
                None,
                format!("cannot load the agent permission registry: {error}"),
                "create permissions.yaml at the repository root with an `agents` mapping",
            ));
            return;
        }
    };
    let declared: BTreeSet<String> = policy.agents().map(str::to_owned).collect();

    let agents_dir = root.join("agents");
    if !agents_dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(&agents_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        let stem = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        if let Some(frontmatter_name) = agent_frontmatter_name(entry.path())
            && frontmatter_name != stem
        {
            diagnostics.push(diagnostic(
                AGENT_NAME_MISMATCH,
                Severity::Error,
                &Path::new("agents").join(format!("{stem}.md")),
                None,
                format!(
                    "agent frontmatter name {frontmatter_name:?} does not match file name {stem:?}"
                ),
                "align the frontmatter `name` with the file stem",
            ));
        }
        if !declared.contains(stem) {
            diagnostics.push(diagnostic(
                AGENT_NOT_IN_REGISTRY,
                Severity::Error,
                &Path::new("agents").join(format!("{stem}.md")),
                None,
                format!("agent {stem} is not declared in permissions.yaml"),
                "add the agent to the permission registry (default-deny unless declared)",
            ));
        }
    }

    for name in &declared {
        if !agents_dir.join(format!("{name}.md")).exists() {
            diagnostics.push(diagnostic(
                REGISTRY_ORPHAN,
                Severity::Warning,
                Path::new("permissions.yaml"),
                None,
                format!(
                    "permission registry declares agent {name:?} without an agents/{name}.md file"
                ),
                "remove the orphan entry or create the agent file",
            ));
        }
    }
}

fn agent_frontmatter_name(path: &Path) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    let rest = source.strip_prefix("---")?;
    let frontmatter = rest.split_once("\n---")?.0;
    frontmatter
        .lines()
        .find_map(|line| line.strip_prefix("name:"))
        .map(|value| value.trim().trim_matches('"').trim_matches('\'').to_owned())
        .filter(|value| !value.is_empty())
}

fn lint_pack_manifest(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let relative = Path::new("manifest.toml");
    let path = root.join(relative);
    let manifest = match sddk_domain::load_pack_manifest(&path) {
        Ok(manifest) => manifest,
        Err(sddk_domain::PackError::Io { .. }) => {
            diagnostics.push(diagnostic(
                INVALID_PACK_MANIFEST,
                Severity::Error,
                relative,
                None,
                "pack manifest manifest.toml is missing",
                "declare the framework pack with identity, commands, and fixtures",
            ));
            return;
        }
        Err(error) => {
            diagnostics.push(diagnostic(
                INVALID_PACK_MANIFEST,
                Severity::Error,
                relative,
                None,
                format!("pack manifest is invalid: {error}"),
                "fix the TOML syntax or align it with the pack model",
            ));
            return;
        }
    };
    for pack_diagnostic in sddk_domain::validate_pack_manifest(&manifest) {
        diagnostics.push(diagnostic(
            INVALID_PACK_MANIFEST,
            Severity::Error,
            relative,
            None,
            format!("{}: {}", pack_diagnostic.code, pack_diagnostic.message),
            pack_diagnostic.hint,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::SourcePatterns;

    #[test]
    fn yaml_reference_does_not_cross_line_after_agent_mapping_key() {
        let patterns = SourcePatterns::new();
        let source = "agent:\n  - agent.execution.started\n";

        assert!(patterns.yaml_reference.captures(source).is_none());
    }

    #[test]
    fn yaml_reference_still_matches_scalar_agent_reference() {
        let patterns = SourcePatterns::new();
        let captures = patterns
            .yaml_reference
            .captures("agent: sddk-apply\n")
            .expect("scalar agent reference must match");

        assert_eq!(&captures[1], "agent");
        assert_eq!(&captures[2], "sddk-apply");
    }
}
