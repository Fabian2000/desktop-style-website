//! IndexedDB storage layer for the virtual filesystem
//!
//! Uses three object stores:
//! - `nodes`: File/directory metadata (FileNode as JSON)
//! - `content`: File contents (ArrayBuffer/Blob)
//! - `system_versions`: Hash versions for system file updates

use crate::filesystem::types::{FileNode, VfsError};
use js_sys::{Array, ArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransaction, IdbTransactionMode,
};

const DB_NAME: &str = "filesystem";
const DB_VERSION: u32 = 1;

const STORE_NODES: &str = "nodes";
const STORE_CONTENT: &str = "content";
const STORE_VERSIONS: &str = "system_versions";

/// Filesystem storage backed by IndexedDB
pub struct FsStorage {
    db: IdbDatabase,
}

impl FsStorage {
    /// Open or create the filesystem database
    pub async fn open() -> Result<Self, VfsError> {
        let window = web_sys::window().ok_or_else(|| {
            VfsError::StorageError("No window object available".to_string())
        })?;

        let idb = window
            .indexed_db()
            .map_err(|e| VfsError::StorageError(format!("IndexedDB error: {:?}", e)))?
            .ok_or_else(|| VfsError::StorageError("IndexedDB not supported".to_string()))?;

        let open_request = idb
            .open_with_u32(DB_NAME, DB_VERSION)
            .map_err(|e| VfsError::StorageError(format!("Failed to open database: {:?}", e)))?;

        // Handle database upgrade (create object stores)
        let onupgradeneeded = Closure::wrap(Box::new(move |event: web_sys::IdbVersionChangeEvent| {
            let target = event.target().unwrap();
            let request: IdbOpenDbRequest = target.dyn_into().unwrap();
            let db: IdbDatabase = request.result().unwrap().dyn_into().unwrap();

            // Create object stores (only called when version changes)
            // Try to create each store, ignore errors if they already exist
            let _ = db.create_object_store(STORE_NODES);
            let _ = db.create_object_store(STORE_CONTENT);
            let _ = db.create_object_store(STORE_VERSIONS);
        }) as Box<dyn FnMut(_)>);

        open_request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
        onupgradeneeded.forget();

        // Wait for database to open
        let db = Self::await_request(&open_request).await?;
        let db: IdbDatabase = db
            .dyn_into()
            .map_err(|_| VfsError::StorageError("Failed to cast database".to_string()))?;

        Ok(Self { db })
    }

    /// Check if the filesystem has been initialized
    pub async fn is_initialized(&self) -> Result<bool, VfsError> {
        // Check if /home node exists
        self.get_node("/home").await.map(|n| n.is_some())
    }

    // ============ Node Operations ============

    /// Get a file/directory node by path
    pub async fn get_node(&self, path: &str) -> Result<Option<FileNode>, VfsError> {
        let transaction = self.transaction(&[STORE_NODES], IdbTransactionMode::Readonly)?;
        let store = transaction.object_store(STORE_NODES).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .get(&JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to get node: {:?}", e)))?;

        let result = Self::await_request(&request).await?;

        if result.is_undefined() || result.is_null() {
            Ok(None)
        } else {
            let node: FileNode = serde_wasm_bindgen::from_value(result)
                .map_err(|e| VfsError::SerializationError(format!("{}", e)))?;
            Ok(Some(node))
        }
    }

    /// Save a file/directory node
    pub async fn put_node(&self, node: &FileNode) -> Result<(), VfsError> {
        let transaction = self.transaction(&[STORE_NODES], IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_NODES).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let value = serde_wasm_bindgen::to_value(node)
            .map_err(|e| VfsError::SerializationError(format!("{}", e)))?;

        let request = store
            .put_with_key(&value, &JsValue::from_str(&node.path))
            .map_err(|e| VfsError::StorageError(format!("Failed to put node: {:?}", e)))?;

        Self::await_request(&request).await?;
        Ok(())
    }

    /// Delete a file/directory node
    pub async fn delete_node(&self, path: &str) -> Result<(), VfsError> {
        let transaction = self.transaction(&[STORE_NODES], IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_NODES).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .delete(&JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to delete node: {:?}", e)))?;

        Self::await_request(&request).await?;
        Ok(())
    }

    /// List all child nodes of a directory
    pub async fn list_children(&self, parent_path: &str) -> Result<Vec<FileNode>, VfsError> {
        let transaction = self.transaction(&[STORE_NODES], IdbTransactionMode::Readonly)?;
        let store = transaction.object_store(STORE_NODES).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        // Get all nodes and filter by parent
        let request = store
            .get_all()
            .map_err(|e| VfsError::StorageError(format!("Failed to get all nodes: {:?}", e)))?;

        let result = Self::await_request(&request).await?;
        let array: Array = result.dyn_into().map_err(|_| {
            VfsError::StorageError("Failed to cast result to array".to_string())
        })?;

        let parent_prefix = if parent_path.ends_with('/') {
            parent_path.to_string()
        } else {
            format!("{}/", parent_path)
        };

        let mut children = Vec::new();
        for i in 0..array.length() {
            let value = array.get(i);
            if let Ok(node) = serde_wasm_bindgen::from_value::<FileNode>(value) {
                // Check if this is a direct child (not nested deeper)
                if node.path.starts_with(&parent_prefix) {
                    let remainder = &node.path[parent_prefix.len()..];
                    // Direct child has no more slashes in the remainder
                    if !remainder.contains('/') {
                        children.push(node);
                    }
                }
            }
        }

        Ok(children)
    }

    /// Get all nodes that start with a given prefix (for recursive operations)
    pub async fn get_nodes_with_prefix(&self, prefix: &str) -> Result<Vec<FileNode>, VfsError> {
        let transaction = self.transaction(&[STORE_NODES], IdbTransactionMode::Readonly)?;
        let store = transaction.object_store(STORE_NODES).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .get_all()
            .map_err(|e| VfsError::StorageError(format!("Failed to get all nodes: {:?}", e)))?;

        let result = Self::await_request(&request).await?;
        let array: Array = result.dyn_into().map_err(|_| {
            VfsError::StorageError("Failed to cast result to array".to_string())
        })?;

        let prefix_with_slash = if prefix.ends_with('/') {
            prefix.to_string()
        } else {
            format!("{}/", prefix)
        };

        let mut nodes = Vec::new();
        for i in 0..array.length() {
            let value = array.get(i);
            if let Ok(node) = serde_wasm_bindgen::from_value::<FileNode>(value) {
                if node.path == prefix || node.path.starts_with(&prefix_with_slash) {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    // ============ Content Operations ============

    /// Get file content as bytes
    pub async fn get_content(&self, path: &str) -> Result<Option<Vec<u8>>, VfsError> {
        let transaction = self.transaction(&[STORE_CONTENT], IdbTransactionMode::Readonly)?;
        let store = transaction.object_store(STORE_CONTENT).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .get(&JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to get content: {:?}", e)))?;

        let result = Self::await_request(&request).await?;

        if result.is_undefined() || result.is_null() {
            Ok(None)
        } else {
            let array_buffer: ArrayBuffer = result.dyn_into().map_err(|_| {
                VfsError::StorageError("Failed to cast content to ArrayBuffer".to_string())
            })?;
            let uint8_array = Uint8Array::new(&array_buffer);
            Ok(Some(uint8_array.to_vec()))
        }
    }

    /// Save file content
    pub async fn put_content(&self, path: &str, data: &[u8]) -> Result<(), VfsError> {
        let transaction = self.transaction(&[STORE_CONTENT], IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_CONTENT).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let uint8_array = Uint8Array::from(data);
        let array_buffer = uint8_array.buffer();

        let request = store
            .put_with_key(&array_buffer, &JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to put content: {:?}", e)))?;

        Self::await_request(&request).await?;
        Ok(())
    }

    /// Delete file content
    pub async fn delete_content(&self, path: &str) -> Result<(), VfsError> {
        let transaction = self.transaction(&[STORE_CONTENT], IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_CONTENT).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .delete(&JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to delete content: {:?}", e)))?;

        Self::await_request(&request).await?;
        Ok(())
    }

    // ============ System Version Operations ============

    /// Get the version hash of a system file
    pub async fn get_system_version(&self, path: &str) -> Result<Option<String>, VfsError> {
        let transaction = self.transaction(&[STORE_VERSIONS], IdbTransactionMode::Readonly)?;
        let store = transaction.object_store(STORE_VERSIONS).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .get(&JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to get version: {:?}", e)))?;

        let result = Self::await_request(&request).await?;

        if result.is_undefined() || result.is_null() {
            Ok(None)
        } else {
            Ok(result.as_string())
        }
    }

    /// Set the version hash of a system file
    pub async fn set_system_version(&self, path: &str, hash: &str) -> Result<(), VfsError> {
        let transaction = self.transaction(&[STORE_VERSIONS], IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_VERSIONS).map_err(|e| {
            VfsError::StorageError(format!("Failed to get object store: {:?}", e))
        })?;

        let request = store
            .put_with_key(&JsValue::from_str(hash), &JsValue::from_str(path))
            .map_err(|e| VfsError::StorageError(format!("Failed to put version: {:?}", e)))?;

        Self::await_request(&request).await?;
        Ok(())
    }

    // ============ Helpers ============

    /// Create a transaction
    fn transaction(
        &self,
        stores: &[&str],
        mode: IdbTransactionMode,
    ) -> Result<IdbTransaction, VfsError> {
        let store_names = Array::new();
        for store in stores {
            store_names.push(&JsValue::from_str(store));
        }

        self.db
            .transaction_with_str_sequence_and_mode(&store_names, mode)
            .map_err(|e| VfsError::StorageError(format!("Failed to create transaction: {:?}", e)))
    }

    /// Wait for an IDB request to complete
    async fn await_request(request: &IdbRequest) -> Result<JsValue, VfsError> {
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();

            let onsuccess = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let target = event.target().unwrap();
                let request: IdbRequest = target.dyn_into().unwrap();
                let result = request.result().unwrap_or(JsValue::UNDEFINED);
                let _ = resolve_clone.call1(&JsValue::UNDEFINED, &result);
            }) as Box<dyn FnMut(_)>);

            let onerror = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let target = event.target().unwrap();
                let request: IdbRequest = target.dyn_into().unwrap();
                let error = request.error().ok().flatten();
                let msg = error
                    .map(|e| format!("{:?}", e))
                    .unwrap_or_else(|| "Unknown error".to_string());
                let _ = reject_clone.call1(&JsValue::UNDEFINED, &JsValue::from_str(&msg));
            }) as Box<dyn FnMut(_)>);

            request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
            request.set_onerror(Some(onerror.as_ref().unchecked_ref()));

            onsuccess.forget();
            onerror.forget();
        });

        JsFuture::from(promise)
            .await
            .map_err(|e| VfsError::StorageError(format!("Request failed: {:?}", e)))
    }
}
