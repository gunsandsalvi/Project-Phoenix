use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::{DependencyKind, MetadataCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Foundation,
    Kernel,
    Interfaces,
    Systems,
    Assembly,
    Apps,
}

impl Layer {
    pub fn from_dir(name: &str) -> Option<Layer> {
        match name {
            "foundation" => Some(Layer::Foundation),
            "kernel" => Some(Layer::Kernel),
            "interfaces" => Some(Layer::Interfaces),
            "systems" => Some(Layer::Systems),
            "assembly" => Some(Layer::Assembly),
            "apps" => Some(Layer::Apps),
            _ => None,
        }
    }

    /// World crates carry the world's rules; the applications are exempt from all but layering.
    pub fn is_world(self) -> bool {
        self != Layer::Apps
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub kind: DepKind,
    /// A path dependency, which in this workspace is always another workspace crate.
    pub internal: bool,
}

#[derive(Debug, Clone)]
pub struct Source {
    /// Relative to the workspace root, with `/` separators.
    pub path: String,
    pub text: String,
    pub file: Result<syn::File, String>,
}

impl Source {
    pub fn new(path: &str, text: &str) -> Source {
        Source { path: path.to_owned(), text: text.to_owned(), file: syn::parse_file(text).map_err(|e| e.to_string()) }
    }

    /// Integration tests and benchmarks are not mechanisms.
    pub fn is_test_or_bench(&self) -> bool {
        self.path.split('/').any(|part| part == "tests" || part == "benches")
    }

    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub layer: Layer,
    /// Relative to the workspace root, such as `crates/apps/phx-check`.
    pub dir: String,
    pub manifest: String,
    pub deps: Vec<Dependency>,
    pub sources: Vec<Source>,
    pub clippy: Option<String>,
}

impl Crate {
    pub fn manifest_path(&self) -> String {
        format!("{}/Cargo.toml", self.dir)
    }

    /// The manifest line that declares a dependency, so a breach points at it.
    pub fn dependency_line(&self, dep: &str) -> usize {
        let underscored = dep.replace('-', "_");
        self.manifest
            .lines()
            .position(|line| {
                let key = line.trim_start();
                [dep, underscored.as_str()]
                    .iter()
                    .any(|name| key.strip_prefix(name).is_some_and(|rest| rest.starts_with([' ', '=', '.'])))
            })
            .map_or(1, |index| index + 1)
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub crates: Vec<Crate>,
    pub root_clippy: String,
    pub ratchets: String,
    pub spec: String,
    pub architecture: String,
    pub plan: String,
}

pub const SPEC: &str = "docs/PROJECT_PHOENIX.md";
pub const ARCHITECTURE: &str = "docs/ARCHITECTURE.md";
pub const PLAN: &str = "docs/IMPLEMENTATION.md";
pub const RATCHETS: &str = "perf/ratchets.toml";

impl Workspace {
    #[cfg(test)]
    pub fn new(crates: Vec<Crate>) -> Workspace {
        Workspace {
            root: PathBuf::new(),
            crates,
            root_clippy: String::new(),
            ratchets: String::new(),
            spec: String::new(),
            architecture: String::new(),
            plan: String::new(),
        }
    }

    pub fn world_crates(&self) -> impl Iterator<Item = &Crate> {
        self.crates.iter().filter(|c| c.layer.is_world())
    }
}

pub fn load() -> Result<Workspace, String> {
    let metadata = MetadataCommand::new().no_deps().exec().map_err(|e| format!("cargo metadata: {e}"))?;
    let root = metadata.workspace_root.clone().into_std_path_buf();
    let mut crates = Vec::new();
    for package in metadata.workspace_packages() {
        let manifest_path = package.manifest_path.clone().into_std_path_buf();
        let dir = manifest_path.parent().ok_or_else(|| format!("{}: no directory", manifest_path.display()))?;
        let crate_dir = relative(&root, dir)?;
        let parts: Vec<&str> = crate_dir.split('/').collect();
        let (Some(&"crates"), Some(layer_dir), Some(_), None) =
            (parts.first(), parts.get(1), parts.get(2), parts.get(3))
        else {
            return Err(format!("{crate_dir}: a crate lives at crates/<layer>/<name>"));
        };
        let layer = Layer::from_dir(layer_dir).ok_or_else(|| format!("{crate_dir}: `{layer_dir}` is not a layer"))?;
        let deps = package
            .dependencies
            .iter()
            .map(|d| Dependency {
                name: d.name.clone(),
                kind: match d.kind {
                    DependencyKind::Development => DepKind::Dev,
                    DependencyKind::Build => DepKind::Build,
                    _ => DepKind::Normal,
                },
                internal: d.path.is_some(),
            })
            .collect();
        let mut sources = Vec::new();
        for path in rust_files(dir)? {
            let text = read(&path)?;
            sources.push(Source::new(&relative(&root, &path)?, &text));
        }
        let clippy_path = dir.join("clippy.toml");
        let clippy = if clippy_path.exists() { Some(read(&clippy_path)?) } else { None };
        crates.push(Crate {
            name: package.name.to_string(),
            layer,
            dir: crate_dir,
            manifest: read(&manifest_path)?,
            deps,
            sources,
            clippy,
        });
    }
    crates.sort_by(|a, b| a.dir.cmp(&b.dir));
    Ok(Workspace {
        root_clippy: read(&root.join("clippy.toml"))?,
        ratchets: read(&root.join(RATCHETS))?,
        spec: read(&root.join(SPEC))?,
        architecture: read(&root.join(ARCHITECTURE))?,
        plan: read(&root.join(PLAN))?,
        root,
        crates,
    })
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    let rel = path.strip_prefix(root).map_err(|_| format!("{} is outside the workspace", path.display()))?;
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    Ok(parts.join("/"))
}

/// Every `.rs` file under a crate, sorted so reports come out in one order.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = fs::read_dir(&next).map_err(|e| format!("{}: {e}", next.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| format!("{}: {e}", next.display()))?.path();
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if path.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(test)]
pub mod fixture {
    use super::{Crate, DepKind, Dependency, Layer, Source};

    pub fn krate(name: &str, layer: Layer) -> Crate {
        Crate {
            name: name.to_owned(),
            layer,
            dir: format!("crates/x/{name}"),
            manifest: String::new(),
            deps: Vec::new(),
            sources: Vec::new(),
            clippy: None,
        }
    }

    pub fn with_dep(mut c: Crate, dep: &str, internal: bool) -> Crate {
        c.deps.push(Dependency { name: dep.to_owned(), kind: DepKind::Normal, internal });
        c
    }

    pub fn with_source(mut c: Crate, file: &str, text: &str) -> Crate {
        let path = format!("{}/{file}", c.dir);
        c.sources.push(Source::new(&path, text));
        c
    }
}
