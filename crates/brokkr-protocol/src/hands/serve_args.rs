//! `brokkr hands serve` argument builders (decision 0043; #1277).
//!
//! Both spellings serialize the exact same JSON bytes of the sealed reach and
//! the hands spec through one builder ([`DocumentSpelling::build`]). Harnesses
//! that hold MCP configuration in JSON (`claude`, `codex`, `agy`, `qoder`) pass
//! `--reach` and `--spec` directly via [`serve_args`]. A dsh seat's `--patch`
//! overlay passes `--reach-base64url` and `--spec-base64url` via
//! [`serve_args_encoded`] so dsh's `--dump-config` (js-yaml) never folds a
//! long JSON scalar onto a `{`-led content line that the driver's line rule
//! refuses (#1277).
//!
//! `--workdir` and `--artifacts` stay plain paths in both spellings (B7): a
//! path dsh folds onto a line the line rule refuses (beginning with a YAML
//! flow indicator or a quote) stays refused, fail closed, and is deliberately
//! not encoded.

use std::path::Path;

use super::{HandsSpec, Sealed};

enum DocumentSpelling {
    Plain,
    Base64Url,
}

impl DocumentSpelling {
    fn build(
        self,
        workdir: &Path,
        artifacts: &Path,
        sealed: &Sealed,
        spec: &HandsSpec,
    ) -> Vec<String> {
        let reach = serde_json::to_string(sealed).unwrap_or_default();
        let spec = spec.to_value().to_string();
        let (reach_flag, reach_value, spec_flag, spec_value) = match self {
            Self::Plain => ("--reach", reach, "--spec", spec),
            Self::Base64Url => (
                "--reach-base64url",
                crate::base64::encode_base64url(reach.as_bytes()),
                "--spec-base64url",
                crate::base64::encode_base64url(spec.as_bytes()),
            ),
        };
        vec![
            "hands".to_string(),
            "serve".to_string(),
            "--workdir".to_string(),
            workdir.to_string_lossy().into_owned(),
            "--artifacts".to_string(),
            artifacts.to_string_lossy().into_owned(),
            reach_flag.to_string(),
            reach_value,
            spec_flag.to_string(),
            spec_value,
        ]
    }
}

/// The `brokkr hands serve …` argument vector a harness spawns with plain
/// JSON `--reach` and `--spec`. A sealed reach that is not UTF-8 is carried
/// empty, which no hands server reads.
pub fn serve_args(
    workdir: &Path,
    artifacts: &Path,
    sealed: &Sealed,
    spec: &HandsSpec,
) -> Vec<String> {
    DocumentSpelling::Plain.build(workdir, artifacts, sealed, spec)
}

/// The `brokkr hands serve …` argument vector a dsh seat's overlay writes
/// (#1277): the exact JSON bytes of `sealed` and `spec` encoded as unpadded
/// base64url under `--reach-base64url` and `--spec-base64url`. `--workdir`
/// and `--artifacts` stay plain paths (B7).
pub(crate) fn serve_args_encoded(
    workdir: &Path,
    artifacts: &Path,
    sealed: &Sealed,
    spec: &HandsSpec,
) -> Vec<String> {
    DocumentSpelling::Base64Url.build(workdir, artifacts, sealed, spec)
}
