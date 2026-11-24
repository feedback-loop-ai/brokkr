//! Explicit upgrade of the machine-wide host file (decision 0125, ask F).

use std::fmt;
use std::fs::{OpenOptions, Permissions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brokkr_runtime::admission::{host_file, host_upgrade, HostError, CURRENT_HOST_SCHEMA};
use tempfile::NamedTempFile;

use crate::Exit;

pub(crate) const UPGRADE_COMMAND: &str = "brokkr system host upgrade";

#[derive(clap::Subcommand)]
pub(crate) enum HostCmd {
    /// Upgrade the machine's host configuration to forge.host/v5.
    Upgrade {
        /// Print the diff and return a failure status if an upgrade is needed.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug)]
enum UpgradeError {
    Invalid(HostError),
    Missing(PathBuf),
    Link(PathBuf),
    Read(PathBuf, io::Error),
    Write(PathBuf, io::Error),
    Output(io::Error),
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => error.fmt(f),
            Self::Missing(path) => write!(
                f,
                "no host configuration at {}; `brokkr init` creates one",
                path.display()
            ),
            Self::Link(path) => write!(
                f,
                "the host configuration at {} is a symbolic link",
                path.display()
            ),
            Self::Read(path, error) => write!(
                f,
                "cannot read the host configuration at {}: {error}",
                path.display()
            ),
            Self::Write(path, error) => write!(
                f,
                "cannot write the host configuration at {}: {error}",
                path.display()
            ),
            Self::Output(error) => write!(f, "cannot print the host upgrade: {error}"),
        }
    }
}

impl std::error::Error for UpgradeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid(error) => std::error::Error::source(error),
            Self::Read(_, error) | Self::Write(_, error) | Self::Output(error) => Some(error),
            Self::Missing(_) | Self::Link(_) => None,
        }
    }
}

pub(super) fn run(command: HostCmd) -> anyhow::Result<ExitCode> {
    let path = host_file(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )?;
    let HostCmd::Upgrade { check } = command;
    Ok(upgrade_with(&path, check, &mut io::stdout().lock(), persist)?.into())
}

struct Existing {
    text: String,
    permissions: Permissions,
}

fn read(path: &Path) -> io::Result<Existing> {
    // NONBLOCK lets a FIFO be refused as a non-regular file by its open
    // handle. NOFOLLOW refuses the leaf even if it is raced in.
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::other("expected a regular file"));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(Existing {
        text,
        permissions: metadata.permissions(),
    })
}

fn read_error(path: &Path, source: io::Error) -> UpgradeError {
    match (source.kind(), source.raw_os_error()) {
        (io::ErrorKind::NotFound, _) => UpgradeError::Missing(path.into()),
        (_, Some(libc::ELOOP)) => UpgradeError::Link(path.into()),
        _ => UpgradeError::Read(path.into(), source),
    }
}

fn upgrade_with(
    path: &Path,
    check: bool,
    out: &mut impl Write,
    finish: impl FnOnce(NamedTempFile, &Path) -> io::Result<()>,
) -> Result<Exit, UpgradeError> {
    let existing = read(path).map_err(|source| read_error(path, source))?;
    let Some(upgraded) = host_upgrade(path, &existing.text).map_err(UpgradeError::Invalid)? else {
        writeln!(out, "{} is current ({CURRENT_HOST_SCHEMA})", path.display())
            .map_err(UpgradeError::Output)?;
        return Ok(Exit::Completed);
    };
    diff(out, path, &existing.text, &upgraded).map_err(UpgradeError::Output)?;
    out.flush().map_err(UpgradeError::Output)?;
    if check {
        return Ok(Exit::Failed);
    }
    replace(path, &upgraded, existing.permissions, finish)
        .map_err(|source| UpgradeError::Write(path.into(), source))?;
    Ok(Exit::Completed)
}

fn replace(
    path: &Path,
    text: &str,
    permissions: Permissions,
    finish: impl FnOnce(NamedTempFile, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let mut staged = NamedTempFile::new_in(parent(path))?;
    staged.write_all(text.as_bytes())?;
    staged.as_file().set_permissions(permissions)?;
    staged.as_file().sync_all()?;
    finish(staged, path)
}

fn parent(path: &Path) -> &Path {
    path.parent().expect("the host file has a parent directory")
}

fn persist(staged: NamedTempFile, path: &Path) -> io::Result<()> {
    staged.persist(path).map(drop).map_err(|error| error.error)
}

fn diff(out: &mut impl Write, path: &Path, before: &str, after: &str) -> io::Result<()> {
    let lines = before
        .split_inclusive('\n')
        .zip(after.split_inclusive('\n'));
    let (line, (old, new)) = lines
        .enumerate()
        .find(|(_, (old, new))| old != new)
        .expect("an upgrade changes the schema token");
    let number = line + 1;
    let path = path.display();
    writeln!(out, "--- {path}\n+++ {path}\n@@ -{number} +{number} @@")?;
    diff_line(out, '-', old)?;
    diff_line(out, '+', new)
}

fn diff_line(out: &mut impl Write, prefix: char, line: &str) -> io::Result<()> {
    write!(out, "{prefix}{line}")?;
    if !line.ends_with('\n') {
        writeln!(out, "\n\\ No newline at end of file")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
