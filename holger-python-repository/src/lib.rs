use holger_traits::{ArtifactFormat, ArtifactId, RepositoryBackendTrait};
use std::fs;
use std::path::PathBuf;

/// Minimal filesystem backed Python package repository
pub struct PythonRepo {
    pub name: String,
    pub root: PathBuf,
    pub writable: bool,
}

impl PythonRepo {
    pub fn new(name: String, root: PathBuf, writable: bool) -> Self {
        PythonRepo {
            name,
            root,
            writable,
        }
    }

    fn file_path(&self, id: &ArtifactId) -> PathBuf {
        let mut path = self.root.clone();
        if let Some(ns) = &id.namespace {
            path.push(ns);
        }
        path.push(&id.name);
        let filename = format!("{}-{}.whl", id.name, id.version);
        path.push(filename);
        path
    }
}

impl RepositoryBackendTrait for PythonRepo {
    fn name(&self) -> &str {
        &self.name
    }

    fn format(&self) -> ArtifactFormat {
        ArtifactFormat::Pip
    }

    fn is_writable(&self) -> bool {
        self.writable
    }

    fn fetch(&self, id: &ArtifactId) -> anyhow::Result<Option<Vec<u8>>> {
        let path = self.file_path(id);
        if path.exists() {
            Ok(Some(fs::read(path)?))
        } else {
            Ok(None)
        }
    }

    fn put(&self, id: &ArtifactId, data: &[u8]) -> anyhow::Result<()> {
        if !self.writable {
            anyhow::bail!("Repository is read only");
        }
        let path = self.file_path(id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, data)?;
        Ok(())
    }

    fn handle_http2_request(
        &self,
        suburl: &str,
        _body: &[u8],
    ) -> anyhow::Result<(u16, Vec<(String, String)>, Vec<u8>)> {
        // Expect /<repo>/packages/<name>/<version>
        let parts: Vec<&str> = suburl.trim_start_matches('/').split('/').collect();
        match parts.as_slice() {
            [repo, "packages", name, version] if *repo == self.name() => {
                let id = ArtifactId {
                    namespace: None,
                    name: name.to_string(),
                    version: version.to_string(),
                };
                if let Some(data) = self.fetch(&id)? {
                    Ok((
                        200,
                        vec![("Content-Type".into(), "application/octet-stream".into())],
                        data,
                    ))
                } else {
                    Ok((404, Vec::new(), b"Not found".to_vec()))
                }
            }
            _ => Ok((404, Vec::new(), b"Not found".to_vec())),
        }
    }
}
