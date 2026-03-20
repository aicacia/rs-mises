#![cfg(feature = "std")]

use std::{env, fs, path::PathBuf};

use mises_file_store::{FileMetadata, FileStore, FsFileStore};

fn make_temp_dir() -> PathBuf {
  let mut path = env::temp_dir();
  path.push(format!("mises-file-store-test-{}", uuid::Uuid::new_v4()));
  path
}

#[test]
fn put_get_delete_and_list() {
  let root = make_temp_dir();
  fs::remove_dir_all(&root).ok();

  let store = FsFileStore::new(&root).expect("create store");

  let resource = "resource-1";
  let key = "folder/hello.txt";
  let data = b"Hi there!";

  let metadata = FileMetadata {
    content_type: Some("text/plain".to_string()),
    size: 0,
    created_at: None,
    updated_at: None,
    tags: None,
  };

  store.put(resource, key, data, &metadata).expect("put");

  let got = store.get(resource, key).expect("get").expect("found");
  assert_eq!(got.data, data);
  assert!(got.metadata.is_some());
  let got_meta = got.metadata.unwrap();
  assert_eq!(got_meta.content_type.as_deref(), Some("text/plain"));
  assert_eq!(got_meta.size, data.len() as u64);
  assert!(got_meta.created_at.is_some());
  assert!(got_meta.updated_at.is_some());

  let listed = store.list(resource, None).expect("list");
  assert_eq!(listed, vec![key.to_string()]);

  let listed_prefixed = store.list(resource, Some("folder/")).expect("list prefix");
  assert_eq!(listed_prefixed, vec![key.to_string()]);

  store.delete(resource, key).expect("delete");
  assert!(
    store
      .get(resource, key)
      .expect("get after delete")
      .is_none()
  );

  fs::remove_dir_all(&root).unwrap();
}

#[test]
fn reject_path_traversal_on_put() {
  let root = make_temp_dir();
  fs::remove_dir_all(&root).ok();

  let store = FsFileStore::new(&root).expect("create store");

  let resource = "resource-2";
  let key = "../evil.txt";
  let data = b"evil";

  let metadata = FileMetadata {
    content_type: None,
    size: 0,
    created_at: None,
    updated_at: None,
    tags: None,
  };

  let err = store
    .put(resource, key, data, &metadata)
    .expect_err("expected error");
  assert!(matches!(err, mises_file_store::FileStoreError::InvalidPath));

  fs::remove_dir_all(&root).unwrap();
}
