#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::{collections::BTreeMap, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{
  fs,
  io::Read,
  path::{Component, Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
  pub content_type: Option<String>,
  pub size: u64,
  pub created_at: Option<i64>,
  pub updated_at: Option<i64>,
  pub tags: Option<BTreeMap<String, String>>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileObject {
  pub data: Vec<u8>,
  pub metadata: Option<FileMetadata>,
}

#[cfg_attr(feature = "thiserror", derive(thiserror::Error))]
#[derive(Debug)]
#[cfg_attr(not(feature = "std"), derive(PartialEq, Eq))]
pub enum FileStoreError {
  #[cfg_attr(feature = "thiserror", error("paths must be relative and normalized"))]
  InvalidPath,

  #[cfg_attr(feature = "thiserror", error("resource invalid"))]
  InvalidResource,

  #[cfg_attr(feature = "thiserror", error("resource not found"))]
  NotFound,

  #[cfg_attr(feature = "thiserror", error("permission denied"))]
  Forbidden,
  #[cfg(all(feature = "std", feature = "thiserror"))]
  #[cfg_attr(feature = "thiserror", error("IO error: {0}"))]
  Io(#[cfg_attr(feature = "thiserror", from)] std::io::Error),

  #[cfg(all(feature = "std", feature = "thiserror"))]
  #[cfg_attr(feature = "thiserror", error("serialization error: {0}"))]
  Serde(#[cfg_attr(feature = "thiserror", from)] serde_json::Error),

  #[cfg(not(feature = "std"))]
  #[cfg_attr(feature = "thiserror", error("IO error: {0}"))]
  Io(String),

  #[cfg(not(feature = "std"))]
  #[cfg_attr(feature = "thiserror", error("serialization error: {0}"))]
  Serde(String),
}

pub type FileStoreResult<T> = core::result::Result<T, FileStoreError>;

pub trait FileStore {
  fn put(
    &self,
    resource_id: &str,
    path: &str,
    data: &[u8],
    metadata: &FileMetadata,
  ) -> FileStoreResult<()>;
  fn get(&self, resource_id: &str, path: &str) -> FileStoreResult<Option<FileObject>>;
  fn delete(&self, resource_id: &str, path: &str) -> FileStoreResult<()>;
  fn list(&self, resource_id: &str, prefix: Option<&str>) -> FileStoreResult<Vec<String>>;
}

#[cfg(not(feature = "std"))]
pub struct FsFileStore;

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct FsFileStore {
  root: PathBuf,
}

#[cfg(feature = "std")]
impl FsFileStore {
  pub fn new<P: AsRef<Path>>(root: P) -> FileStoreResult<Self> {
    let root = root.as_ref().to_path_buf();
    fs::create_dir_all(&root).map_err(|e| FileStoreError::Io(e))?;
    Ok(Self { root })
  }

  fn normalize_resource(resource_id: &str) -> FileStoreResult<&str> {
    if resource_id.is_empty() {
      return Err(FileStoreError::InvalidResource);
    }
    let path = Path::new(resource_id);
    if path.is_absolute() {
      return Err(FileStoreError::InvalidResource);
    }

    for component in path.components() {
      match component {
        Component::Normal(_) => (),
        _ => return Err(FileStoreError::InvalidResource),
      }
    }

    Ok(resource_id)
  }

  fn normalize_key(path: &str) -> FileStoreResult<PathBuf> {
    let src = Path::new(path);
    if src.is_absolute() {
      return Err(FileStoreError::InvalidPath);
    }

    let mut normalized = PathBuf::new();
    for component in src.components() {
      match component {
        Component::RootDir | Component::Prefix(_) => return Err(FileStoreError::InvalidPath),
        Component::CurDir => continue,
        Component::ParentDir => return Err(FileStoreError::InvalidPath),
        Component::Normal(segment) => {
          if segment.is_empty() {
            continue;
          }
          normalized.push(segment);
        }
      }
    }

    if normalized.as_os_str().is_empty() {
      return Err(FileStoreError::InvalidPath);
    }

    Ok(normalized)
  }

  fn resource_dir(&self, resource_id: &str) -> FileStoreResult<PathBuf> {
    let resource = Self::normalize_resource(resource_id)?;
    Ok(self.root.join(resource))
  }

  fn file_path(&self, resource_id: &str, key: &str) -> FileStoreResult<PathBuf> {
    let dir = self.resource_dir(resource_id)?;
    let key_path = Self::normalize_key(key)?;

    let file_path = dir.join(&key_path);
    if !file_path.starts_with(&dir) {
      return Err(FileStoreError::Forbidden);
    }

    Ok(file_path)
  }

  fn metadata_path(data_file: &Path) -> PathBuf {
    let mut meta_file_name = data_file
      .file_name()
      .map(|s| s.to_os_string())
      .unwrap_or_else(|| "".into());
    meta_file_name.push(".meta.json");

    let mut meta_path = data_file.to_path_buf();
    meta_path.set_file_name(meta_file_name);
    meta_path
  }

  fn timestamp_now() -> i64 {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_secs() as i64)
      .unwrap_or(0)
  }
}

#[cfg(feature = "std")]
impl FileStore for FsFileStore {
  fn put(
    &self,
    resource_id: &str,
    path: &str,
    data: &[u8],
    metadata: &FileMetadata,
  ) -> FileStoreResult<()> {
    let file_path = self.file_path(resource_id, path)?;
    let parent = file_path.parent().ok_or(FileStoreError::InvalidPath)?;

    fs::create_dir_all(parent).map_err(|e| FileStoreError::Io(e))?;
    fs::write(&file_path, data).map_err(|e| FileStoreError::Io(e))?;

    let mut metadata = metadata.clone();
    metadata.size = data.len() as u64;
    let now = Self::timestamp_now();
    if metadata.created_at.is_none() {
      metadata.created_at = Some(now);
    }
    metadata.updated_at = Some(now);

    let meta_path = Self::metadata_path(&file_path);
    let serialized = serde_json::to_vec(&metadata).map_err(|e| FileStoreError::Serde(e))?;
    fs::write(meta_path, serialized).map_err(|e| FileStoreError::Io(e))?;

    Ok(())
  }

  fn get(&self, resource_id: &str, path: &str) -> FileStoreResult<Option<FileObject>> {
    let file_path = self.file_path(resource_id, path)?;
    if !file_path.exists() {
      return Ok(None);
    }

    let mut file = fs::File::open(&file_path).map_err(|e| FileStoreError::Io(e))?;
    let mut buf = Vec::new();
    file
      .read_to_end(&mut buf)
      .map_err(|e| FileStoreError::Io(e))?;

    let meta_path = Self::metadata_path(&file_path);
    let metadata = if meta_path.exists() {
      let mut meta_file = fs::File::open(&meta_path).map_err(|e| FileStoreError::Io(e))?;
      let mut meta_buf = String::new();
      meta_file
        .read_to_string(&mut meta_buf)
        .map_err(|e| FileStoreError::Io(e))?;
      Some(serde_json::from_str(&meta_buf).map_err(|e| FileStoreError::Serde(e))?)
    } else {
      None
    };

    Ok(Some(FileObject {
      data: buf,
      metadata,
    }))
  }

  fn delete(&self, resource_id: &str, path: &str) -> FileStoreResult<()> {
    let file_path = self.file_path(resource_id, path)?;
    if file_path.exists() {
      fs::remove_file(&file_path).map_err(|e| FileStoreError::Io(e))?;
    }

    let meta_path = Self::metadata_path(&file_path);
    if meta_path.exists() {
      fs::remove_file(meta_path).map_err(|e| FileStoreError::Io(e))?;
    }

    Ok(())
  }

  fn list(&self, resource_id: &str, prefix: Option<&str>) -> FileStoreResult<Vec<String>> {
    let dir = self.resource_dir(resource_id)?;
    if !dir.exists() {
      return Ok(Vec::new());
    }

    let mut items = Vec::new();

    fn walk_dir(base: &Path, current: &Path, list: &mut Vec<String>) -> FileStoreResult<()> {
      for entry in fs::read_dir(current).map_err(|e| FileStoreError::Io(e))? {
        let entry = entry.map_err(|e| FileStoreError::Io(e))?;
        let path = entry.path();
        if path.is_dir() {
          walk_dir(base, &path, list)?;
          continue;
        }

        let filename = path.file_name().and_then(|os| os.to_str());
        if let Some(name) = filename {
          if name.ends_with(".meta.json") {
            continue;
          }
        }

        let rel = path
          .strip_prefix(base)
          .map_err(|_| FileStoreError::Forbidden)?
          .to_string_lossy()
          .into_owned();

        list.push(rel);
      }

      Ok(())
    }

    walk_dir(&dir, &dir, &mut items)?;

    if let Some(prefix) = prefix {
      let filtered: Vec<String> = items
        .into_iter()
        .filter(|key| key.starts_with(prefix))
        .collect();
      Ok(filtered)
    } else {
      Ok(items)
    }
  }
}
