//! The history the registration and no-tuning checks read, through the `git` the workspace is kept in.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct Git {
    root: PathBuf,
}

/// A path a commit touched and how: `A` added, `M` modified, `D` deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Touched {
    pub status: char,
    pub path: String,
}

impl Git {
    pub fn at(root: &Path) -> Git {
        Git { root: root.to_path_buf() }
    }

    fn run(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()
            .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
        if !out.status.success() {
            return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
        }
        String::from_utf8(out.stdout).map_err(|e| format!("git {}: {e}", args.join(" ")))
    }

    /// Refuses a history the checks cannot read whole.
    pub fn complete(&self) -> Result<(), String> {
        if self.run(&["rev-parse", "--is-shallow-repository"])?.trim() == "true" {
            return Err("the history is shallow; fetch it whole".to_owned());
        }
        Ok(())
    }

    /// The first commit of the first-parent history that added `path`, if any has.
    pub fn added(&self, path: &str) -> Result<Option<String>, String> {
        let out = self.run(&["log", "--first-parent", "--diff-filter=A", "--format=%H", "--", path])?;
        Ok(out.lines().last().map(str::to_owned))
    }

    /// The first-parent commits after `from`, oldest first.
    pub fn since(&self, from: &str) -> Result<Vec<String>, String> {
        let range = format!("{from}..HEAD");
        Ok(self.run(&["rev-list", "--first-parent", "--reverse", &range])?.lines().map(str::to_owned).collect())
    }

    /// The first-parent commits that touched `path`, oldest first.
    pub fn touching(&self, path: &str) -> Result<Vec<String>, String> {
        let out = self.run(&["log", "--first-parent", "--reverse", "--format=%H", "--", path])?;
        Ok(out.lines().map(str::to_owned).collect())
    }

    /// What a commit changed under `dir`, against its first parent; a root commit adds everything it holds.
    pub fn changed(&self, commit: &str, dir: &str) -> Result<Vec<Touched>, String> {
        let out = self.run(&[
            "diff-tree",
            "-r",
            "--no-commit-id",
            "--name-status",
            "--root",
            "-m",
            "--first-parent",
            commit,
            "--",
            dir,
        ])?;
        let mut touched = Vec::new();
        for line in out.lines() {
            let mut parts = line.split('\t');
            let (Some(status), Some(path)) = (parts.next(), parts.next()) else { continue };
            let Some(first) = status.chars().next() else { continue };
            touched.push(Touched { status: first, path: path.to_owned() });
        }
        Ok(touched)
    }

    /// A file as a commit holds it, or none where it does not.
    pub fn show(&self, commit: &str, path: &str) -> Option<String> {
        self.run(&["show", &format!("{commit}:{path}")]).ok()
    }

    /// A commit's message; a merge's carries those of every commit it brings in, whose trailers it answers for.
    pub fn message(&self, commit: &str) -> Result<String, String> {
        let own = self.run(&["log", "-1", "--format=%B", commit])?;
        if self.run(&["rev-parse", "--verify", "-q", &format!("{commit}^2")]).is_err() {
            return Ok(own);
        }
        let brought = self.run(&["log", "--format=%B", &format!("{commit}^1..{commit}")])?;
        Ok(format!("{own}\n{brought}"))
    }

    pub fn parent(&self, commit: &str) -> Option<String> {
        self.run(&["rev-parse", "--verify", "-q", &format!("{commit}^1")]).ok().map(|s| s.trim().to_owned())
    }

    /// Whether `a` is an ancestor of `b`, or `b` itself.
    pub fn is_ancestor(&self, a: &str, b: &str) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["merge-base", "--is-ancestor", a, b])
            .status()
            .is_ok_and(|s| s.success())
    }
}

/// A commit message's trailers of one kind: the text after `<key>: `.
pub fn trailers<'a>(message: &'a str, key: &str) -> Vec<&'a str> {
    let prefix = format!("{key}: ");
    message.lines().filter_map(|l| l.trim().strip_prefix(prefix.as_str())).map(str::trim).collect()
}
