use std::path::{Path, PathBuf};

/// Take `path`, treat it as a path relative to `base_path`, normalize it, and return a new path
/// with the result.
pub fn normalize_path_relative_to(path: &Path, base_path: &Path) -> PathBuf {
    let is_relative = !path.starts_with(Path::new("/"));

    let path = if is_relative {
        let base = base_path.parent().unwrap_or_else(|| Path::new(""));
        base.join(path)
    } else {
        path.to_path_buf()
    };

    normalize_path(&path)
}

/// Normalize a path
pub fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ std::path::Component::Prefix(..)) = components.peek() {
        let buf = std::path::PathBuf::from(c.as_os_str());
        components.next();
        buf
    } else {
        std::path::PathBuf::new()
    };

    for component in components {
        match component {
            std::path::Component::Prefix(..) => unreachable!(),
            std::path::Component::RootDir => {
                ret.push(component.as_os_str());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                ret.pop();
            }
            std::path::Component::Normal(c) => {
                ret.push(c);
            }
        }
    }

    ret
}
