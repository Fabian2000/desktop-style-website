use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use js_sys::*;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

// Benötigte Cargo.toml Features:
// [dependencies]
// wasm-bindgen = "0.2"
// wasm-bindgen-futures = "0.4" 
// web-sys = { version = "0.3", features = [
//     "Window",
//     "IdbFactory", 
//     "IdbDatabase",
//     "IdbOpenDbRequest", 
//     "IdbRequest",
//     "IdbObjectStore",
//     "IdbTransaction", 
//     "IdbTransactionMode",
//     "Event",
//     "EventTarget",
//     "DomException"  # <-- Für .error() support
// ]}
// js-sys = "0.3"

// Event system types - Updated to use JsValue
type EventCallback = Box<dyn Fn(&str, Option<&JsValue>) + Send + Sync>;
static EVENT_LISTENERS: LazyLock<Mutex<HashMap<String, EventCallback>>> = 
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug)]
pub enum IndexedDbError {
    JsError(String),
    DatabaseNotFound,
    TransactionFailed,
    InvalidOperation,
    SerializationError,
    ItemNotFound,
}

impl From<JsValue> for IndexedDbError {
    fn from(js_val: JsValue) -> Self {
        if let Some(error_str) = js_val.as_string() {
            IndexedDbError::JsError(error_str)
        } else {
            IndexedDbError::JsError("Unknown JavaScript error".to_string())
        }
    }
}

#[allow(dead_code)]
pub struct IndexedDb {
    db_name: String,
    store_name: String,
    db: Option<IdbDatabase>,
}

#[allow(dead_code)]
impl IndexedDb {
    pub async fn open(name: &str, store: &str) -> Result<Self, IndexedDbError> {
        let mut db = Self {
            db_name: name.to_string(),
            store_name: store.to_string(),
            db: None,
        };
        let window = match web_sys::window() {
            Some(w) => w,
            None => return Err(IndexedDbError::JsError("No window object".to_string())),
        };

        let idb = match window.indexed_db() {
            Ok(Some(idb)) => idb,
            Ok(None) => return Err(IndexedDbError::JsError("IndexedDB not supported".to_string())),
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let open_request = match idb.open_with_u32(name, 1) {
            Ok(req) => req,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        // Setup onupgradeneeded callback
        let store_name_clone = store.to_string();
        let onupgradeneeded_closure = Closure::wrap(Box::new(move |event: Event| {
            if let Some(target) = event.target() {
                if let Ok(request) = target.dyn_into::<IdbOpenDbRequest>() {
                    if let Ok(result) = request.result() {
                        if let Ok(database) = result.dyn_into::<IdbDatabase>() {
                            let _ = database.create_object_store(&store_name_clone);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(Event)>);

        open_request.set_onupgradeneeded(Some(onupgradeneeded_closure.as_ref().unchecked_ref()));
        onupgradeneeded_closure.forget();

        // Convert the request to a promise manually
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();
            
            let success_closure = Closure::wrap(Box::new(move |event: Event| {
                if let Some(target) = event.target() {
                    if let Ok(request) = target.dyn_into::<IdbOpenDbRequest>() {
                        match request.result() {
                            Ok(result) => {
                                let _ = resolve_clone.call1(&JsValue::UNDEFINED, &result);
                            },
                            Err(_) => {
                                let _ = reject_clone.call1(&JsValue::UNDEFINED, &JsValue::from_str("Failed to get result"));
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(Event)>);

            open_request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            success_closure.forget();

            let error_closure = Closure::wrap(Box::new(move |event: Event| {
                let error_msg = if let Some(target) = event.target() {
                    if let Ok(request) = target.dyn_into::<IdbOpenDbRequest>() {
                        if let Ok(Some(error)) = request.error() {
                            format!("{:?}", error)
                        } else {
                            "Unknown database error".to_string()
                        }
                    } else {
                        "Failed to get request target".to_string()
                    }
                } else {
                    "No event target".to_string()
                };
                let _ = reject.call1(&JsValue::UNDEFINED, &JsValue::from_str(&error_msg));
            }) as Box<dyn FnMut(Event)>);

            open_request.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            error_closure.forget();
        });

        let result = match JsFuture::from(promise).await {
            Ok(val) => val,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        db.db = Some(match result.dyn_into::<IdbDatabase>() {
            Ok(database) => database,
            Err(_) => return Err(IndexedDbError::JsError("Failed to cast to IdbDatabase".to_string())),
        });

        Ok(db)
    }

    pub async fn get_item(&self, key: &str) -> Result<JsValue, IndexedDbError> {
        let db = match &self.db {
            Some(db) => db,
            None => return Err(IndexedDbError::DatabaseNotFound),
        };

        let transaction = match db.transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readonly) {
            Ok(t) => t,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let store = match transaction.object_store(&self.store_name) {
            Ok(s) => s,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let request = match store.get(&JsValue::from_str(key)) {
            Ok(req) => req,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let promise = Self::request_to_promise(request);
        let result = match JsFuture::from(promise).await {
            Ok(val) => val,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        if result.is_undefined() || result.is_null() {
            Err(IndexedDbError::ItemNotFound)
        } else {
            Ok(result)
        }
    }

    pub async fn set_item(&self, key: &str, value: &JsValue) -> Result<(), IndexedDbError> {
        let db = match &self.db {
            Some(db) => db,
            None => return Err(IndexedDbError::DatabaseNotFound),
        };

        let transaction = match db.transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite) {
            Ok(t) => t,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let store = match transaction.object_store(&self.store_name) {
            Ok(s) => s,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let request = match store.put_with_key(value, &JsValue::from_str(key)) {
            Ok(req) => req,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let promise = Self::request_to_promise(request);
        match JsFuture::from(promise).await {
            Ok(_) => {
                Self::emit_event(key, Some(value));
                Ok(())
            },
            Err(e) => Err(IndexedDbError::from(e)),
        }
    }

    pub async fn has_item(&self, key: &str) -> bool {
        match self.get_item(key).await {
            Ok(_) => true,
            Err(IndexedDbError::ItemNotFound) => false,
            Err(_) => false, // Other errors also count as "not found" for has_item
        }
    }

    pub async fn delete_item(&self, key: &str) -> Result<(), IndexedDbError> {
        let db = match &self.db {
            Some(db) => db,
            None => return Err(IndexedDbError::DatabaseNotFound),
        };

        let transaction = match db.transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite) {
            Ok(t) => t,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let store = match transaction.object_store(&self.store_name) {
            Ok(s) => s,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let request = match store.delete(&JsValue::from_str(key)) {
            Ok(req) => req,
            Err(e) => return Err(IndexedDbError::from(e)),
        };

        let promise = Self::request_to_promise(request);
        match JsFuture::from(promise).await {
            Ok(_) => {
                Self::emit_event(key, None);
                Ok(())
            },
            Err(e) => Err(IndexedDbError::from(e)),
        }
    }

    // Helper method to convert IdbRequest to Promise - now static since open is static
    fn request_to_promise(request: IdbRequest) -> js_sys::Promise {
        js_sys::Promise::new(&mut |resolve, reject| {
            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();
            
            let success_closure = Closure::wrap(Box::new(move |event: Event| {
                if let Some(target) = event.target() {
                    if let Ok(req) = target.dyn_into::<IdbRequest>() {
                        match req.result() {
                            Ok(result) => {
                                let _ = resolve_clone.call1(&JsValue::UNDEFINED, &result);
                            },
                            Err(_) => {
                                let _ = resolve_clone.call1(&JsValue::UNDEFINED, &JsValue::UNDEFINED);
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(Event)>);

            request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            success_closure.forget();

            let error_closure = Closure::wrap(Box::new(move |event: Event| {
                let error_msg = if let Some(target) = event.target() {
                    if let Ok(req) = target.dyn_into::<IdbRequest>() {
                        if let Ok(Some(error)) = req.error() {
                            format!("{:?}", error)
                        } else {
                            "Unknown request error".to_string()
                        }
                    } else {
                        "Failed to get request target".to_string()
                    }
                } else {
                    "No event target".to_string()
                };
                let _ = reject.call1(&JsValue::UNDEFINED, &JsValue::from_str(&error_msg));
            }) as Box<dyn FnMut(Event)>);

            request.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            error_closure.forget();
        })
    }

    pub fn register_listener_unique<F>(&self, id: &str, callback: F) -> bool 
    where 
        F: Fn(&str, Option<&JsValue>) + Send + Sync + 'static,
    {
        match EVENT_LISTENERS.lock() {
            Ok(mut listeners) => {
                if listeners.contains_key(id) {
                    false // Event listener already exists
                } else {
                    listeners.insert(id.to_string(), Box::new(callback));
                    true // Event listener registered successfully
                }
            },
            Err(_) => false, // Mutex poisoned, but we don't panic
        }
    }

    fn emit_event(key: &str, value: Option<&JsValue>) {
        if let Ok(listeners) = EVENT_LISTENERS.lock() {
            for (_id, callback) in listeners.iter() {
                callback(key, value);
            }
        }
        // If mutex is poisoned, we silently continue - no panics
    }
}