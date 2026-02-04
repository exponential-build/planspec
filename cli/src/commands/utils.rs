use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Scan a directory for YAML files, optionally recursively.
/// Returns paths sorted for deterministic ordering.
pub fn scan_directory(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    scan_directory_internal(path, recursive, &mut files)?;
    files.sort();
    Ok(files)
}

fn scan_directory_internal(path: &Path, recursive: bool, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("Failed to read entry in {}", path.display()))?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            if recursive {
                scan_directory_internal(&entry_path, recursive, files)?;
            }
        } else if is_yaml_file(&entry_path) {
            files.push(entry_path);
        }
    }

    Ok(())
}

/// Check if a path is a YAML file by extension
fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml") | Some("yml")
    )
}

/// Input source for commands that can accept file or directory
pub enum InputSource {
    File(PathBuf),
    Directory { path: PathBuf, recursive: bool },
}

impl InputSource {
    /// Create an InputSource from optional file/directory arguments.
    /// Returns an error if neither is provided.
    pub fn from_args(
        file: Option<String>,
        directory: Option<String>,
        recursive: bool,
    ) -> Result<Self> {
        match (file, directory) {
            (Some(f), None) => Ok(InputSource::File(PathBuf::from(f))),
            (None, Some(d)) => Ok(InputSource::Directory {
                path: PathBuf::from(d),
                recursive,
            }),
            (None, None) => anyhow::bail!("Either --file or --directory must be specified"),
            (Some(_), Some(_)) => anyhow::bail!("Cannot specify both --file and --directory"),
        }
    }

    /// Get all file paths from this input source
    pub fn get_files(&self) -> Result<Vec<PathBuf>> {
        match self {
            InputSource::File(path) => Ok(vec![path.clone()]),
            InputSource::Directory { path, recursive } => scan_directory(path, *recursive),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_directory_non_recursive() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        // Create test files
        fs::write(dir.join("a.yaml"), "test").unwrap();
        fs::write(dir.join("b.yml"), "test").unwrap();
        fs::write(dir.join("c.txt"), "test").unwrap(); // Should be ignored

        let files = scan_directory(dir, false).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("a.yaml")));
        assert!(files.iter().any(|f| f.ends_with("b.yml")));
    }

    #[test]
    fn test_scan_directory_recursive() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        // Create nested structure
        fs::write(dir.join("root.yaml"), "test").unwrap();
        fs::create_dir(dir.join("subdir")).unwrap();
        fs::write(dir.join("subdir").join("nested.yaml"), "test").unwrap();

        // Non-recursive should only find root
        let files = scan_directory(dir, false).unwrap();
        assert_eq!(files.len(), 1);

        // Recursive should find both
        let files = scan_directory(dir, true).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp = TempDir::new().unwrap();
        let files = scan_directory(temp.path(), false).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_input_source_file() {
        let source = InputSource::from_args(Some("test.yaml".into()), None, false).unwrap();
        match source {
            InputSource::File(p) => assert_eq!(p, PathBuf::from("test.yaml")),
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_input_source_directory() {
        let source = InputSource::from_args(None, Some("./manifests".into()), true).unwrap();
        match source {
            InputSource::Directory { path, recursive } => {
                assert_eq!(path, PathBuf::from("./manifests"));
                assert!(recursive);
            }
            _ => panic!("Expected Directory variant"),
        }
    }

    #[test]
    fn test_input_source_neither() {
        let result = InputSource::from_args(None, None, false);
        assert!(result.is_err());
    }
}
