//! Release artifact provenance guards (#1383, #1396).

use std::collections::{BTreeMap, BTreeSet};

use super::workspace;
use crate::workflow::{
    self, ActionMatch, ArtifactName, ClosedFormRefusal, ClosedJob, ClosedStep, ClosedWorkflow,
    ClosedWorkflowError, Inputs, RefusalKind, Scalar, Workflow, WorkflowError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProvenanceProblem {
    UnreadableWorkflow {
        file: String,
        err: WorkflowError,
    },
    ClosedForm(ClosedFormRefusal),
    MissingJob(WorkflowError),
    MissingBuildMatrix,
    UnexpectedBuildMatrixShape,
    BuildMatrixRowMissingName,
    DuplicateBuildArtifactName(String),
    InvalidNfpmArch {
        name: String,
        raw: Scalar,
    },
    EmptyBuildMatrix,
    ConditionalJob {
        job_id: String,
        cond: String,
    },
    ContinueOnErrorJob {
        job_id: String,
    },
    ContinueOnErrorStep {
        job_id: String,
    },
    ConditionalUploadStep {
        job_id: String,
        cond: String,
    },
    ConditionalDownloadStep {
        job_id: String,
        cond: String,
    },
    BuildUsesPagesUpload,
    BuildUsesUploadSubAction(String),
    UnexpectedBuildUploadNames(Vec<Option<ArtifactName>>),
    UnparseableUses {
        source: String,
        job_id: String,
        uses: String,
    },
    UnparseableCalledWorkflow {
        source: String,
        job_id: String,
        target: String,
    },
    ExternalCalledWorkflow {
        source: String,
        job_id: String,
        target: String,
    },
    UncheckedCalledWorkflow(String),
    DownloadUsesPattern {
        job_id: String,
    },
    DownloadUsesMergeMultiple {
        job_id: String,
    },
    DownloadCrossRunInput {
        job_id: String,
        input: String,
    },
    DownloadUnexpectedInput {
        job_id: String,
        input: String,
    },
    DownloadWrongPath {
        job_id: String,
        actual: Option<Scalar>,
        expected: String,
    },
    DownloadMissingName {
        job_id: String,
    },
    DownloadNonLiteralName {
        job_id: String,
        raw: Scalar,
    },
    DownloadDisallowedName {
        job_id: String,
        name: String,
    },
    DownloadSetMismatch {
        job_id: String,
        seen: BTreeSet<String>,
        expected: BTreeSet<String>,
    },
    PagesDefaultUploadCollision {
        source: String,
        job_id: String,
    },
    UnnamedUpload {
        source: String,
        job_id: String,
    },
    UnresolvedUploadName {
        source: String,
        job_id: String,
        raw: Scalar,
    },
    CollidingUploadName {
        source: String,
        job_id: String,
        name: String,
    },
}

fn build_matrix_artifact_names(
    build: &ClosedJob,
    problems: &mut Vec<ProvenanceProblem>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut all, mut linux_pkg) = (BTreeSet::new(), BTreeSet::new());
    let Some(matrix) = build.matrix() else {
        problems.push(ProvenanceProblem::MissingBuildMatrix);
        return (all, linux_pkg);
    };
    if !matrix.axes.is_empty() {
        problems.push(ProvenanceProblem::UnexpectedBuildMatrixShape);
    }
    for row in &matrix.include {
        let Some(name) = row
            .get("name")
            .and_then(Scalar::as_str)
            .filter(|s| workflow::is_plain_artifact_literal(s))
        else {
            problems.push(ProvenanceProblem::BuildMatrixRowMissingName);
            continue;
        };
        if !all.insert(name.to_string()) {
            problems.push(ProvenanceProblem::DuplicateBuildArtifactName(
                name.to_string(),
            ));
        }
        if let Some(arch) = row.get("nfpm_arch") {
            if arch.as_literal().is_some() {
                linux_pkg.insert(name.to_string());
            } else {
                problems.push(ProvenanceProblem::InvalidNfpmArch {
                    name: name.to_string(),
                    raw: arch.clone(),
                });
            }
        }
    }
    if all.is_empty() || linux_pkg.is_empty() {
        problems.push(ProvenanceProblem::EmptyBuildMatrix);
    }
    (all, linux_pkg)
}

fn called_workflows_in_uses<'a>(
    source: &str,
    jobs: impl Iterator<Item = (&'a str, Option<&'a str>)>,
    problems: &mut Vec<ProvenanceProblem>,
) -> Vec<String> {
    let mut called = Vec::new();
    for (job_id, uses) in jobs {
        let Some(target) = uses else {
            continue;
        };
        if target.is_empty() || target.contains("${{") || target.contains(['\n', '\r']) {
            problems.push(ProvenanceProblem::UnparseableCalledWorkflow {
                source: source.to_string(),
                job_id: job_id.to_string(),
                target: target.to_string(),
            });
            continue;
        }
        match target
            .strip_prefix("./")
            .filter(|p| p.starts_with(".github/workflows/"))
        {
            Some(path) => called.push(path.to_string()),
            None => problems.push(ProvenanceProblem::ExternalCalledWorkflow {
                source: source.to_string(),
                job_id: job_id.to_string(),
                target: target.to_string(),
            }),
        }
    }
    called
}

fn check_download_with_keys(job_id: &str, with: &Inputs, problems: &mut Vec<ProvenanceProblem>) {
    for key in with.keys() {
        let p = match key {
            "name" | "path" => continue,
            "pattern" => ProvenanceProblem::DownloadUsesPattern {
                job_id: job_id.to_string(),
            },
            "merge-multiple" => ProvenanceProblem::DownloadUsesMergeMultiple {
                job_id: job_id.to_string(),
            },
            "run-id" | "repository" | "github-token" => ProvenanceProblem::DownloadCrossRunInput {
                job_id: job_id.to_string(),
                input: key.to_string(),
            },
            other => ProvenanceProblem::DownloadUnexpectedInput {
                job_id: job_id.to_string(),
                input: other.to_string(),
            },
        };
        problems.push(p);
    }
}

fn check_build_uploads(build: &ClosedJob, problems: &mut Vec<ProvenanceProblem>) {
    if let Some(cond) = build.condition.raw() {
        problems.push(ProvenanceProblem::ConditionalJob {
            job_id: "build".to_string(),
            cond: cond.to_string(),
        });
    }
    if build.allows_failure() {
        problems.push(ProvenanceProblem::ContinueOnErrorJob {
            job_id: "build".to_string(),
        });
    }
    let mut upload_names = Vec::new();
    for step in &build.steps {
        let Some(uses) = step.uses() else {
            continue;
        };
        if matches!(
            workflow::action_repo_match(uses, "actions/upload-pages-artifact"),
            Some(ActionMatch::Exact | ActionMatch::SubAction(_))
        ) {
            problems.push(ProvenanceProblem::BuildUsesPagesUpload);
            continue;
        }
        match workflow::action_repo_match(uses, "actions/upload-artifact") {
            None => {
                problems.push(ProvenanceProblem::UnparseableUses {
                    source: "release.yml".to_string(),
                    job_id: "build".to_string(),
                    uses: uses.to_string(),
                });
                continue;
            }
            Some(ActionMatch::Other) => continue,
            Some(ActionMatch::SubAction(sub)) => {
                problems.push(ProvenanceProblem::BuildUsesUploadSubAction(sub));
                upload_names.push(None);
                continue;
            }
            Some(ActionMatch::Exact) => {}
        }
        if let Some(cond) = step.condition.raw() {
            problems.push(ProvenanceProblem::ConditionalUploadStep {
                job_id: "build".to_string(),
                cond: cond.to_string(),
            });
        }
        if step.allows_failure() {
            problems.push(ProvenanceProblem::ContinueOnErrorStep {
                job_id: "build".to_string(),
            });
        }
        upload_names.push(step.artifact_name.clone());
    }
    if upload_names != [Some(ArtifactName::MatrixAxis("name".to_string()))] {
        problems.push(ProvenanceProblem::UnexpectedBuildUploadNames(upload_names));
    }
}

struct DownloadExpectation<'a> {
    expected: Option<&'a BTreeSet<String>>,
    expected_path: Option<&'a str>,
    allowed: &'a BTreeSet<String>,
}

impl<'a> DownloadExpectation<'a> {
    fn dist(expected: &'a BTreeSet<String>, allowed: &'a BTreeSet<String>) -> Self {
        Self {
            expected: Some(expected),
            expected_path: Some("dist"),
            allowed,
        }
    }
}

fn check_download_step(
    job_id: &str,
    step: &ClosedStep,
    spec: &DownloadExpectation<'_>,
    seen: &mut BTreeSet<String>,
    problems: &mut Vec<ProvenanceProblem>,
) {
    if spec.expected.is_some_and(|e| !e.is_empty()) {
        if let Some(cond) = step.condition.raw() {
            problems.push(ProvenanceProblem::ConditionalDownloadStep {
                job_id: job_id.to_string(),
                cond: cond.to_string(),
            });
        }
        if step.allows_failure() {
            problems.push(ProvenanceProblem::ContinueOnErrorStep {
                job_id: job_id.to_string(),
            });
        }
    }
    let Some(with) = step.with() else {
        problems.push(ProvenanceProblem::DownloadMissingName {
            job_id: job_id.to_string(),
        });
        return;
    };
    check_download_with_keys(job_id, with, problems);
    if let Some(req_path) = spec.expected_path {
        if with.get("path").and_then(Scalar::as_literal) != Some(req_path) {
            problems.push(ProvenanceProblem::DownloadWrongPath {
                job_id: job_id.to_string(),
                actual: with.get("path").cloned(),
                expected: req_path.to_string(),
            });
        }
    }
    match &step.artifact_name {
        None => problems.push(ProvenanceProblem::DownloadMissingName {
            job_id: job_id.to_string(),
        }),
        Some(ArtifactName::Literal(name)) => {
            if !spec.allowed.contains(name) {
                problems.push(ProvenanceProblem::DownloadDisallowedName {
                    job_id: job_id.to_string(),
                    name: name.clone(),
                });
            }
            seen.insert(name.clone());
        }
        Some(ArtifactName::MatrixAxis(_) | ArtifactName::JobIndex { .. }) => {
            problems.push(ProvenanceProblem::DownloadNonLiteralName {
                job_id: job_id.to_string(),
                raw: with.get("name").cloned().unwrap_or(Scalar::Null),
            });
        }
    }
}

fn check_job_downloads(
    job: &ClosedJob,
    spec: &DownloadExpectation<'_>,
    problems: &mut Vec<ProvenanceProblem>,
) {
    let id = job.id().to_string();
    if spec.expected.is_some_and(|e| !e.is_empty()) {
        if let Some(cond) = job.condition.raw() {
            problems.push(ProvenanceProblem::ConditionalJob {
                job_id: id.clone(),
                cond: cond.to_string(),
            });
        }
        if job.allows_failure() {
            problems.push(ProvenanceProblem::ContinueOnErrorJob { job_id: id.clone() });
        }
    }
    let mut seen = BTreeSet::new();
    for step in &job.steps {
        let Some(uses) = step.uses() else {
            continue;
        };
        match workflow::action_repo_match(uses, "actions/download-artifact") {
            None => problems.push(ProvenanceProblem::UnparseableUses {
                source: "release.yml".to_string(),
                job_id: id.clone(),
                uses: uses.to_string(),
            }),
            Some(ActionMatch::Other) => {}
            Some(ActionMatch::Exact | ActionMatch::SubAction(_)) => {
                check_download_step(&id, step, spec, &mut seen, problems);
            }
        }
    }
    if let Some(expected) = spec.expected.filter(|exp| *exp != &seen) {
        problems.push(ProvenanceProblem::DownloadSetMismatch {
            job_id: id.clone(),
            seen,
            expected: expected.clone(),
        });
    }
}

fn check_non_build_uploads(
    source: &str,
    job: &ClosedJob,
    build_names: &BTreeSet<String>,
    problems: &mut Vec<ProvenanceProblem>,
) {
    for step in &job.steps {
        let Some(uses) = step.uses() else {
            continue;
        };
        let is_artifact = match workflow::action_repo_match(uses, "actions/upload-artifact") {
            None => {
                problems.push(ProvenanceProblem::UnparseableUses {
                    source: source.to_string(),
                    job_id: job.id().to_string(),
                    uses: uses.to_string(),
                });
                continue;
            }
            Some(ActionMatch::Exact | ActionMatch::SubAction(_)) => true,
            Some(ActionMatch::Other) => false,
        };
        let is_pages = matches!(
            workflow::action_repo_match(uses, "actions/upload-pages-artifact"),
            Some(ActionMatch::Exact | ActionMatch::SubAction(_))
        );
        if !is_artifact && !is_pages {
            continue;
        }
        match (is_pages, &step.artifact_name) {
            (true, None) if build_names.contains("github-pages") => {
                problems.push(ProvenanceProblem::PagesDefaultUploadCollision {
                    source: source.to_string(),
                    job_id: job.id().to_string(),
                });
            }
            (true, None) => {}
            (false, None) => problems.push(ProvenanceProblem::UnnamedUpload {
                source: source.to_string(),
                job_id: job.id().to_string(),
            }),
            (_, Some(ArtifactName::Literal(name))) if build_names.contains(name) => {
                problems.push(ProvenanceProblem::CollidingUploadName {
                    source: source.to_string(),
                    job_id: job.id().to_string(),
                    name: name.clone(),
                });
            }
            (_, Some(ArtifactName::Literal(_))) => {}
            (_, Some(ArtifactName::MatrixAxis(_) | ArtifactName::JobIndex { .. })) => {
                problems.push(ProvenanceProblem::UnresolvedUploadName {
                    source: source.to_string(),
                    job_id: job.id().to_string(),
                    raw: step.with_value("name").cloned().unwrap_or(Scalar::Null),
                });
            }
        }
    }
}

fn release_artifact_provenance_problems(
    release: &str,
    called_workflows: &BTreeMap<String, String>,
) -> Vec<ProvenanceProblem> {
    let mut problems = Vec::new();
    let wf = match ClosedWorkflow::parse("release.yml", release) {
        Ok(wf) => wf,
        Err(ClosedWorkflowError::Parse { file, err }) => {
            return vec![ProvenanceProblem::UnreadableWorkflow { file, err }];
        }
        Err(ClosedWorkflowError::Refusal(refusal)) => {
            return vec![ProvenanceProblem::ClosedForm(refusal)];
        }
    };
    let build = match wf.require_job("release.yml", "build") {
        Ok(job) => job,
        Err(err) => return vec![ProvenanceProblem::MissingJob(err)],
    };
    check_build_uploads(build, &mut problems);
    let (build_names, linux_pkg_names) = build_matrix_artifact_names(build, &mut problems);
    let no_artifacts = BTreeSet::new();
    let (mut saw_publish, mut saw_pages) = (false, false);
    for job in &wf.jobs {
        if job.id() == "build" {
            let spec = DownloadExpectation {
                expected: Some(&no_artifacts),
                expected_path: None,
                allowed: &no_artifacts,
            };
            check_job_downloads(job, &spec, &mut problems);
            continue;
        }
        check_non_build_uploads("release.yml", job, &build_names, &mut problems);
        let spec = match job.id() {
            "publish" => {
                saw_publish = true;
                DownloadExpectation::dist(&build_names, &build_names)
            }
            "pages" => {
                saw_pages = true;
                DownloadExpectation::dist(&linux_pkg_names, &build_names)
            }
            _ => DownloadExpectation {
                expected: None,
                expected_path: None,
                allowed: &build_names,
            },
        };
        check_job_downloads(job, &spec, &mut problems);
    }
    for (seen, id) in [(saw_publish, "publish"), (saw_pages, "pages")] {
        if !seen {
            problems.push(ProvenanceProblem::MissingJob(WorkflowError::MissingJob {
                file: "release.yml".to_string(),
                id: id.to_string(),
            }));
        }
    }
    check_called_workflows(&wf, called_workflows, &build_names, &mut problems);
    problems
}

fn check_called_workflows(
    release_wf: &ClosedWorkflow,
    called_workflows: &BTreeMap<String, String>,
    build_names: &BTreeSet<String>,
    problems: &mut Vec<ProvenanceProblem>,
) {
    let mut pending = called_workflows_in_uses(
        "release.yml",
        release_wf.jobs.iter().map(|j| (j.id(), j.uses())),
        problems,
    );
    let mut visited = BTreeSet::new();
    while let Some(path) = pending.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let Some(text) = called_workflows.get(&path) else {
            problems.push(ProvenanceProblem::UncheckedCalledWorkflow(path));
            continue;
        };
        let called_wf = match ClosedWorkflow::parse(&path, text) {
            Ok(wf) => wf,
            Err(ClosedWorkflowError::Parse { file, err }) => {
                problems.push(ProvenanceProblem::UnreadableWorkflow { file, err });
                continue;
            }
            Err(ClosedWorkflowError::Refusal(refusal)) => {
                problems.push(ProvenanceProblem::ClosedForm(refusal));
                continue;
            }
        };
        for job in &called_wf.jobs {
            check_non_build_uploads(&path, job, build_names, problems);
        }
        pending.extend(called_workflows_in_uses(
            &path,
            called_wf.jobs.iter().map(|j| (j.id(), j.uses())),
            problems,
        ));
    }
}

fn release_and_called() -> (String, BTreeMap<String, String>) {
    let root = workspace();
    let release = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let release_wf = Workflow::parse(&release).expect("release.yml parses");
    let mut problems = Vec::new();
    let mut pending = called_workflows_in_uses(
        "release.yml",
        release_wf
            .jobs
            .iter()
            .map(|j| (j.id.as_str(), j.uses.as_deref())),
        &mut problems,
    );
    assert_eq!(problems, Vec::<ProvenanceProblem>::new());
    let mut called = BTreeMap::new();
    while let Some(path) = pending.pop() {
        if called.contains_key(&path) {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&path)).unwrap();
        let wf = Workflow::parse(&text).unwrap_or_else(|e| panic!("{path} parses: {e}"));
        pending.extend(called_workflows_in_uses(
            &path,
            wf.jobs.iter().map(|j| (j.id.as_str(), j.uses.as_deref())),
            &mut problems,
        ));
        assert_eq!(problems, Vec::<ProvenanceProblem>::new());
        called.insert(path, text);
    }
    (release, called)
}

fn map_shards(
    called: &BTreeMap<String, String>,
    f: impl FnOnce(&str) -> String,
) -> BTreeMap<String, String> {
    let mut mutated = called.clone();
    let entry = mutated
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap();
    *entry = f(entry);
    mutated
}

fn with_shard_upload_name(
    called: &BTreeMap<String, String>,
    replacement: &str,
) -> BTreeMap<String, String> {
    map_shards(called, |s| {
        s.replacen(
            "          name: r3-native-diagnostics\n",
            &format!("          name: {replacement}\n"),
            1,
        )
    })
}

fn reindent_jobs(workflow: &str, extra_spaces: &str) -> String {
    let (head, tail) = workflow.split_once("jobs:\n").expect("jobs:");
    let indented = tail
        .lines()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{extra_spaces}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{head}jobs:\n{indented}\n")
}

#[test]
fn release_publishes_only_build_matrix_artifacts() {
    let (release, called) = release_and_called();
    assert!(called.contains_key(".github/workflows/macos-test-shards.yml"));
    assert_eq!(
        release_artifact_provenance_problems(&release, &called),
        vec![]
    );

    let dl = "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n";
    let widened_publish = release.replacen(
        &format!("{dl}          name: brokkr-linux-x86_64\n          path: dist\n"),
        &format!("{dl}          path: dist\n          merge-multiple: true\n"),
        1,
    );
    let widened_problems = release_artifact_provenance_problems(&widened_publish, &called);
    assert!(
        widened_problems.contains(&ProvenanceProblem::DownloadUsesMergeMultiple {
            job_id: "publish".to_string(),
        })
    );
    assert!(
        widened_problems.contains(&ProvenanceProblem::DownloadMissingName {
            job_id: "publish".to_string(),
        })
    );

    for pattern_line in [
        "          pattern: brokkr-*\n",
        "          Pattern: brokkr-*\n",
    ] {
        let pattern_publish =
            release.replacen("          name: brokkr-linux-x86_64\n", pattern_line, 1);
        assert!(
            release_artifact_provenance_problems(&pattern_publish, &called).contains(
                &ProvenanceProblem::DownloadUsesPattern {
                    job_id: "publish".to_string()
                }
            )
        );
    }

    let widened_pages = release.replacen(
        &format!("{dl}          name: brokkr-linux-aarch64\n          path: dist\n      - name: repository tooling"),
        &format!("{dl}          path: dist\n          merge-multiple: true\n      - name: repository tooling"),
        1,
    );
    assert!(
        release_artifact_provenance_problems(&widened_pages, &called).contains(
            &ProvenanceProblem::DownloadUsesMergeMultiple {
                job_id: "pages".to_string()
            }
        )
    );

    let called_extra = map_shards(&called, |s| {
        format!("{s}      - name: extra diagnostics\n        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n          name: extra-macos-diagnostics\n          path: target/extra.log\n")
    });
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_extra),
        vec![]
    );

    let called_collide = with_shard_upload_name(&called, "brokkr-macos-arm64");
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_collide),
        vec![ProvenanceProblem::CollidingUploadName {
            source: ".github/workflows/macos-test-shards.yml".to_string(),
            job_id: "macos-machines".to_string(),
            name: "brokkr-macos-arm64".to_string(),
        }]
    );
}

#[test]
fn release_provenance_closes_matrix_path_pages_upload_and_indentation_gaps() {
    let (release, called) = release_and_called();

    let block_row = release.replacen(
        "        include:\n",
        "        include:\n          - os: ubuntu-latest\n            target: riscv64-unknown-linux-gnu\n            name: brokkr-linux-riscv64\n",
        1,
    );
    let block_problems = release_artifact_provenance_problems(&block_row, &called);
    assert!(matches!(
        block_problems.as_slice(),
        [ProvenanceProblem::DownloadSetMismatch { job_id, expected, .. }]
            if job_id == "publish" && expected.contains("brokkr-linux-riscv64")
    ));

    let dup_row = release.replacen(
        "        include:\n",
        "        include:\n          - { os: ubuntu-latest, target: x86_64-unknown-linux-gnu, name: brokkr-linux-x86_64, nfpm_arch: amd64 }\n",
        1,
    );
    assert!(
        release_artifact_provenance_problems(&dup_row, &called).contains(
            &ProvenanceProblem::DuplicateBuildArtifactName("brokkr-linux-x86_64".to_string())
        )
    );
    let omitted_linux = release.replacen(", nfpm_arch: amd64", "", 1);
    let omitted_probs = release_artifact_provenance_problems(&omitted_linux, &called);
    assert!(matches!(
        omitted_probs.as_slice(),
        [ProvenanceProblem::DownloadSetMismatch { job_id, expected, .. }]
            if job_id == "pages" && !expected.contains("brokkr-linux-x86_64")
    ));
    for falsy in ["false", "''", "null", "0"] {
        let falsy_linux = release.replacen("nfpm_arch: amd64", &format!("nfpm_arch: {falsy}"), 1);
        let probs = release_artifact_provenance_problems(&falsy_linux, &called);
        assert!(matches!(
            probs.as_slice(),
            [ProvenanceProblem::ClosedForm(r)]
                if r.file == "release.yml"
                    && r.place == "job build step 5 if"
                    && r.kind == RefusalKind::ConditionBarePath
        ));
        let falsy_macos = release.replacen(
            "name: brokkr-macos-arm64 }",
            &format!("name: brokkr-macos-arm64, nfpm_arch: {falsy} }}"),
            1,
        );
        let mac_probs = release_artifact_provenance_problems(&falsy_macos, &called);
        assert!(matches!(
            mac_probs.as_slice(),
            [ProvenanceProblem::ClosedForm(r)]
                if r.file == "release.yml"
                    && r.place == "job build step 5 if"
                    && r.kind == RefusalKind::ConditionBarePath
        ));
    }

    let pub_redir = release.replacen(
        "          name: brokkr-linux-x86_64\n          path: dist\n",
        "          name: brokkr-linux-x86_64\n          path: elsewhere\n",
        1,
    );
    assert_eq!(
        release_artifact_provenance_problems(&pub_redir, &called),
        vec![ProvenanceProblem::DownloadWrongPath {
            job_id: "publish".to_string(),
            actual: Some(Scalar::String("elsewhere".to_string())),
            expected: "dist".to_string(),
        }]
    );
    let pages_redir = release.replacen(
        "          name: brokkr-linux-aarch64\n          path: dist\n      - name: repository tooling",
        "          name: brokkr-linux-aarch64\n          path: staging\n      - name: repository tooling",
        1,
    );
    assert_eq!(
        release_artifact_provenance_problems(&pages_redir, &called),
        vec![ProvenanceProblem::DownloadWrongPath {
            job_id: "pages".to_string(),
            actual: Some(Scalar::String("staging".to_string())),
            expected: "dist".to_string(),
        }]
    );

    let reindented = reindent_jobs(&release, "  ");
    assert_eq!(
        release_artifact_provenance_problems(&reindented, &called),
        vec![]
    );
    let called_reindented_collide = map_shards(
        &with_shard_upload_name(&called, "brokkr-macos-arm64"),
        |s| reindent_jobs(s, "  "),
    );
    assert_eq!(
        release_artifact_provenance_problems(&reindented, &called_reindented_collide),
        vec![ProvenanceProblem::CollidingUploadName {
            source: ".github/workflows/macos-test-shards.yml".to_string(),
            job_id: "macos-machines".to_string(),
            name: "brokkr-macos-arm64".to_string(),
        }]
    );
}

#[test]
fn release_provenance_handles_case_insensitive_inputs_and_unsafe_yaml() {
    let (release, called) = release_and_called();
    for key in ["name", "Name", "NAME"] {
        let pages_upload = release.replacen(
            "      - uses: actions/upload-pages-artifact@56afc609e74202658d3ffba0e8f6dda462b719fa # v3.0.1\n        with:\n          path: site\n",
            &format!("      - uses: actions/upload-pages-artifact@56afc609e74202658d3ffba0e8f6dda462b719fa # v3.0.1\n        with:\n          {key}: brokkr-linux-x86_64\n          path: site\n"),
            1,
        );
        assert_eq!(
            release_artifact_provenance_problems(&pages_upload, &called),
            vec![ProvenanceProblem::CollidingUploadName {
                source: "release.yml".to_string(),
                job_id: "pages".to_string(),
                name: "brokkr-linux-x86_64".to_string(),
            }]
        );
    }
    let shard_upper_name = map_shards(&called, |s| {
        s.replacen(
            "          name: r3-native-diagnostics\n",
            "          Name: brokkr-macos-arm64\n",
            1,
        )
    });
    assert_eq!(
        release_artifact_provenance_problems(&release, &shard_upper_name),
        vec![ProvenanceProblem::CollidingUploadName {
            source: ".github/workflows/macos-test-shards.yml".to_string(),
            job_id: "macos-machines".to_string(),
            name: "brokkr-macos-arm64".to_string(),
        }]
    );

    let dup_key = release.replacen(
        "          name: brokkr-linux-x86_64\n          path: dist\n",
        "          name: brokkr-linux-x86_64\n          name: brokkr-linux-x86_64\n          path: dist\n",
        1,
    );
    assert!(matches!(
        Workflow::parse(&dup_key),
        Err(WorkflowError::Parse(_))
    ));
    let dup_case_input = release.replacen(
        "          name: brokkr-linux-x86_64\n          path: dist\n",
        "          name: brokkr-linux-x86_64\n          Name: brokkr-linux-x86_64\n          path: dist\n",
        1,
    );
    assert_eq!(
        Workflow::parse(&dup_case_input),
        Err(WorkflowError::DuplicateInput {
            ctx: "step.with",
            key: "name".to_string()
        })
    );
    let merge_key = "name: merge\non: push\njobs:\n  a:\n    runs-on: ubuntu-24.04\n    env:\n      <<: { A: B }\n";
    assert_eq!(Workflow::parse(merge_key), Err(WorkflowError::MergeKey));
}

#[test]
fn release_provenance_rejects_build_downloads_cross_run_inputs_and_disguised_uploads() {
    let (release, called) = release_and_called();

    let attest = "      - name: attest packaged binary and checksum\n";
    let dl = "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n";
    let build_dl_all = release.replacen(
        attest,
        &format!("{dl}          path: dist\n          merge-multiple: true\n{attest}"),
        1,
    );
    assert!(
        release_artifact_provenance_problems(&build_dl_all, &called).contains(
            &ProvenanceProblem::DownloadUsesMergeMultiple {
                job_id: "build".to_string()
            }
        )
    );
    let build_dl_named = release.replacen(
        attest,
        &format!("{dl}          name: brokkr-linux-x86_64\n          path: dist\n{attest}"),
        1,
    );
    assert!(
        release_artifact_provenance_problems(&build_dl_named, &called).contains(
            &ProvenanceProblem::DownloadDisallowedName {
                job_id: "build".to_string(),
                name: "brokkr-linux-x86_64".to_string(),
            }
        )
    );

    let target_dl = "          name: brokkr-linux-x86_64\n          path: dist\n";
    for (extra_input, expected) in [
        (
            "          run-id: 12345\n",
            ProvenanceProblem::DownloadCrossRunInput {
                job_id: "publish".into(),
                input: "run-id".into(),
            },
        ),
        (
            "          Run-Id: 12345\n",
            ProvenanceProblem::DownloadCrossRunInput {
                job_id: "publish".into(),
                input: "run-id".into(),
            },
        ),
        (
            "          repository: other/repo\n",
            ProvenanceProblem::DownloadCrossRunInput {
                job_id: "publish".into(),
                input: "repository".into(),
            },
        ),
        (
            "          github-token: ${{ secrets.GITHUB_TOKEN }}\n",
            ProvenanceProblem::DownloadCrossRunInput {
                job_id: "publish".into(),
                input: "github-token".into(),
            },
        ),
        (
            "          artifact-ids: 99\n",
            ProvenanceProblem::DownloadUnexpectedInput {
                job_id: "publish".into(),
                input: "artifact-ids".into(),
            },
        ),
    ] {
        let mutated = release.replacen(target_dl, &format!("{target_dl}{extra_input}"), 1);
        assert_eq!(
            release_artifact_provenance_problems(&mutated, &called),
            vec![expected]
        );
    }
}

#[test]
fn release_provenance_rejects_disguised_and_unresolved_upload_names() {
    let (release, called) = release_and_called();
    for bad_name in [
        "\"brokkr-macos-arm64\"",
        "'brokkr-macos-arm64'",
        "brokkr-macos-arm64 # diagnostic",
        "'brokkr-macos-arm64' # diagnostic",
        "\"brokkr-\\x6dacos-arm64\"",
    ] {
        assert_eq!(
            release_artifact_provenance_problems(
                &release,
                &with_shard_upload_name(&called, bad_name)
            ),
            vec![ProvenanceProblem::CollidingUploadName {
                source: ".github/workflows/macos-test-shards.yml".to_string(),
                job_id: "macos-machines".to_string(),
                name: "brokkr-macos-arm64".to_string(),
            }]
        );
    }
    // Unresolved expressions and leading/trailing blanks (#1417 finding 1) are
    // refused at closed-form admission, never trimmed.
    for bad_name in [
        "${{ matrix.name }}",
        "brokkr-${{ matrix.machine }}",
        "|",
        "\" brokkr-macos-arm64 \"",
        "' brokkr-macos-arm64 '",
        "\"brokkr-macos-arm64 \"",
        "\" brokkr-macos-arm64\"",
        "\" r3-native-diagnostics \"",
    ] {
        let probs = release_artifact_provenance_problems(
            &release,
            &with_shard_upload_name(&called, bad_name),
        );
        assert!(
            matches!(
                probs.as_slice(),
                [ProvenanceProblem::ClosedForm(r)]
                    if r.file == ".github/workflows/macos-test-shards.yml"
                        && r.place == "job macos-machines step 6 with.name"
                        && r.kind == RefusalKind::ArtifactName
            ),
            "upload name `{bad_name}` must be refused: {probs:?}"
        );
    }
    let called_ok = with_shard_upload_name(&called, "'r3-native-diagnostics' # human-readable");
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_ok),
        vec![]
    );
}

#[test]
fn release_provenance_scans_commented_steps_merge_subactions_and_quoted_calls() {
    let (release, called) = release_and_called();
    let expected_shard_collide = vec![ProvenanceProblem::CollidingUploadName {
        source: ".github/workflows/macos-test-shards.yml".to_string(),
        job_id: "macos-machines".to_string(),
        name: "brokkr-macos-arm64".to_string(),
    }];

    let called_commented = map_shards(
        &with_shard_upload_name(&called, "brokkr-macos-arm64"),
        |s| {
            s.replacen("    steps:\n", "    steps: # shard steps\n", 1)
                .replacen("        with:\n", "        with: # upload inputs\n", 1)
        },
    );
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_commented),
        expected_shard_collide
    );

    let called_flush = map_shards(
        &with_shard_upload_name(&called, "brokkr-macos-arm64"),
        |s| {
            let (head, tail) = s.split_once("    steps:\n").expect("steps:");
            let flush = tail
                .lines()
                .map(|l| l.strip_prefix("  ").unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{head}    steps:\n{flush}\n")
        },
    );
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_flush),
        expected_shard_collide
    );

    for (snippet, expected) in [
        ("      - name: merge diagnostics\n        uses: actions/upload-artifact/merge@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n          name: brokkr-macos-arm64\n", expected_shard_collide.clone()),
        ("      - uses: actions/upload-artifact/merge@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n", vec![ProvenanceProblem::UnnamedUpload { source: ".github/workflows/macos-test-shards.yml".into(), job_id: "macos-machines".into() }]),
        ("      - uses: actions/upload-artifact/merge@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n          name: merged-r3-diagnostics\n", vec![]),
    ] {
        let mutated = map_shards(&called, |s| format!("{s}{snippet}"));
        assert_eq!(release_artifact_provenance_problems(&release, &mutated), expected);
    }

    for quoted_call in [
        "'./.github/workflows/extra.yml'",
        "\"./.github/workflows/extra.yml\"",
        "'./.github/workflows/extra.yml' # extra",
    ] {
        let release_with_extra = format!("{release}\n  extra-caller:\n    uses: {quoted_call}\n");
        assert_eq!(
            release_artifact_provenance_problems(&release_with_extra, &called),
            vec![ProvenanceProblem::UncheckedCalledWorkflow(
                ".github/workflows/extra.yml".into()
            )]
        );
        let mut called_with_extra = called.clone();
        called_with_extra.insert(
            ".github/workflows/extra.yml".to_string(),
            "on:\n  workflow_call:\njobs:\n  sneaky:\n    runs-on: ubuntu-24.04\n    steps: # hidden\n      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02\n        with:\n          name: brokkr-linux-x86_64\n          path: dist/*\n".to_string(),
        );
        assert_eq!(
            release_artifact_provenance_problems(&release_with_extra, &called_with_extra),
            vec![ProvenanceProblem::CollidingUploadName {
                source: ".github/workflows/extra.yml".to_string(),
                job_id: "sneaky".to_string(),
                name: "brokkr-linux-x86_64".to_string(),
            }]
        );
    }

    let release_remote = format!("{release}\n  remote-caller:\n    uses: other/repo/.github/workflows/x.yml@11d5960a326750d5838078e36cf38b85af677262\n");
    assert_eq!(
        release_artifact_provenance_problems(&release_remote, &called),
        vec![ProvenanceProblem::ExternalCalledWorkflow {
            source: "release.yml".to_string(),
            job_id: "remote-caller".to_string(),
            target: "other/repo/.github/workflows/x.yml@11d5960a326750d5838078e36cf38b85af677262"
                .into(),
        }]
    );

    let called_flow = map_shards(&called, |s| {
        s.replacen(
            "        with:\n          name: r3-native-diagnostics\n          path: |\n            target/r3-startup.log\n            target/r3-startup-report.txt\n          if-no-files-found: warn\n",
            "        with: { name: brokkr-macos-arm64, path: target/r3-startup.log }\n",
            1,
        )
    });
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_flow),
        expected_shard_collide
    );
}

#[test]
fn release_provenance_rejects_conditional_build_uploads_and_downloads() {
    let (release, called) = release_and_called();

    let conditional_upload = release.replacen(
        "      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n",
        "      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        if: matrix.nfpm_arch\n        with:\n",
        1,
    );
    assert_eq!(
        release_artifact_provenance_problems(&conditional_upload, &called),
        vec![ProvenanceProblem::ConditionalUploadStep {
            job_id: "build".to_string(),
            cond: "matrix.nfpm_arch".to_string(),
        }]
    );

    let conditional_build_job = release.replacen(
        "  build:\n    name: build (${{ matrix.name }})\n",
        "  build:\n    name: build (${{ matrix.name }})\n    if: matrix.nfpm_arch\n",
        1,
    );
    assert_eq!(
        release_artifact_provenance_problems(&conditional_build_job, &called),
        vec![ProvenanceProblem::ConditionalJob {
            job_id: "build".to_string(),
            cond: "matrix.nfpm_arch".to_string(),
        }]
    );

    let conditional_publish_dl = release.replacen(
        "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n          name: brokkr-macos-arm64\n",
        "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        if: github.event_name == 'push'\n        with:\n          name: brokkr-macos-arm64\n",
        1,
    );
    assert_eq!(
        release_artifact_provenance_problems(&conditional_publish_dl, &called),
        vec![ProvenanceProblem::ConditionalDownloadStep {
            job_id: "publish".to_string(),
            cond: "github.event_name == 'push'".to_string(),
        }]
    );
    let non_closed_dl = release.replacen(
        "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n          name: brokkr-macos-arm64\n",
        "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        if: false\n        with:\n          name: brokkr-macos-arm64\n",
        1,
    );
    assert!(matches!(
        release_artifact_provenance_problems(&non_closed_dl, &called).as_slice(),
        [ProvenanceProblem::ClosedForm(r)]
            if r.file == "release.yml"
                && r.place == "job publish step 3 if"
                && r.kind == RefusalKind::ConditionSyntax
    ));
    let release_top_env = release.replacen(
        "permissions:\n",
        "env:\n  EXTRA: value\n\npermissions:\n",
        1,
    );
    assert!(matches!(
        release_artifact_provenance_problems(&release_top_env, &called).as_slice(),
        [ProvenanceProblem::ClosedForm(r)]
            if r.file == "release.yml"
                && r.place == "top-level env"
                && r.kind == RefusalKind::WorkflowEnv
    ));
}

#[test]
fn release_provenance_closes_round_2_closed_form_findings() {
    let (release, called) = release_and_called();

    // G1: comparing boolean/optional matrix.cross against 'true' is refused at admission.
    let cross_cmp = release.replacen(
        "        if: matrix.nfpm_arch\n",
        "        if: matrix.cross == 'true'\n",
        1,
    );
    assert_ne!(cross_cmp, release);
    assert!(matches!(
        release_artifact_provenance_problems(&cross_cmp, &called).as_slice(),
        [ProvenanceProblem::ClosedForm(r)]
            if r.file == "release.yml"
                && r.place == "job build step 5 if"
                && r.kind == RefusalKind::ConditionTypeMismatch
    ));

    // G3: unread matrix exclude on macos-tests is refused at admission.
    let exclude_verdict = release.replacen(
        "  macos-tests:\n    name: test (macos-latest)\n",
        "  macos-tests:\n    name: test (macos-latest)\n    strategy:\n      matrix:\n        x: [1]\n        exclude:\n          - x: 1\n",
        1,
    );
    assert_ne!(exclude_verdict, release);
    assert!(matches!(
        release_artifact_provenance_problems(&exclude_verdict, &called).as_slice(),
        [ProvenanceProblem::ClosedForm(r)]
            if r.file == "release.yml"
                && r.place == "job macos-tests strategy.matrix"
                && r.kind == RefusalKind::MatrixClause
    ));

    // G4: misspelled bare matrix axis absent from every row is refused at admission.
    let misspelled_axis = release.replacen(
        "        if: matrix.nfpm_arch\n",
        "        if: matrix.nfpm_arh\n",
        1,
    );
    assert_ne!(misspelled_axis, release);
    assert!(matches!(
        release_artifact_provenance_problems(&misspelled_axis, &called).as_slice(),
        [ProvenanceProblem::ClosedForm(r)]
            if r.file == "release.yml"
                && r.place == "job build step 5 if"
                && r.kind == RefusalKind::ConditionBarePath
    ));

    // G13: plain literal download pattern passes closed-form admission and hits DownloadUsesPattern.
    let plain_pattern = release.replacen(
        "          name: brokkr-linux-x86_64\n",
        "          pattern: brokkr-linux-x86_64\n",
        1,
    );
    assert_ne!(plain_pattern, release);
    let pat_probs = release_artifact_provenance_problems(&plain_pattern, &called);
    assert!(
        !pat_probs
            .iter()
            .any(|p| matches!(p, ProvenanceProblem::ClosedForm(_))),
        "plain literal download pattern must pass closed-form admission: {pat_probs:?}"
    );
    assert!(pat_probs.contains(&ProvenanceProblem::DownloadUsesPattern {
        job_id: "publish".to_string(),
    }));
}
