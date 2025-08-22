use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransactionMode};
use std::cell::RefCell;
use std::collections::HashMap;

use crate::database::request_to_future::request_to_future;

thread_local! {
	static LISTENERS: RefCell<HashMap<String, HashMap<String, Box<dyn FnMut(String, Option<JsValue>)>>>> =
		RefCell::new(HashMap::new());
}

pub struct IndexedDb {
	name: String,
	database: IdbDatabase,
	store: String,
}

#[allow(dead_code)]
impl IndexedDb {
	fn ns_key(&self) -> String {
		format!("{}:{}", self.name, self.store)
	}

	pub fn register_listener_unique<F>(&self, id: &str, callback: F)
	where
		F: 'static + FnMut(String, Option<JsValue>),
	{
		let ns = self.ns_key();
		LISTENERS.with(|m| {
			let mut map = m.borrow_mut();
			let entry = map.entry(ns).or_default();
			if !entry.contains_key(id) {
				entry.insert(id.to_string(), Box::new(callback));
			}
		});
	}

	fn dispatch_event(&self, key: &str, value: Option<JsValue>) {
		let ns = self.ns_key();
		let key = key.to_string();
		LISTENERS.with(|m| {
			if let Some(entry) = m.borrow_mut().get_mut(&ns) {
				for (_id, cb) in entry.iter_mut() {
					(cb)(key.clone(), value.clone());
				}
			}
		});
	}

	pub async fn open(name: &str, store: &str) -> Result<IndexedDb, JsValue> {
		let window = web_sys::window().ok_or(JsValue::from_str("no window"))?;
		let indexed_db = window.indexed_db()?.ok_or(JsValue::from_str("no indexeddb"))?;
		let request: IdbOpenDbRequest = indexed_db.open(name)?;
		let store_name = store.to_string();

		let on_upgrade = {
			let store_name = store_name.clone();
			Closure::once(move |event: web_sys::Event| {
				if let Some(target) = event.target() {
					if let Ok(req) = target.dyn_into::<IdbOpenDbRequest>() {
						if let Ok(val) = req.result() {
							if let Ok(db) = val.dyn_into::<IdbDatabase>() {
								let _ = db.create_object_store(&store_name);
							}
						}
					}
				}
			})
		};
		request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
		on_upgrade.forget();

		let event = request_to_future(&request).await?;
		let db: IdbDatabase = event.dyn_into()?;
		Ok(IndexedDb { name: name.to_string(), database: db, store: store_name })
	}

	pub async fn set_item(&self, key: &str, value: &JsValue) -> Result<(), JsValue> {
		let tx = self.database.transaction_with_str_and_mode(&self.store, IdbTransactionMode::Readwrite)?;
		let store = tx.object_store(&self.store)?;
		let request: IdbRequest = store.put_with_key(value, &JsValue::from_str(key))?;
		let _ = request_to_future(&request).await?;
		self.dispatch_event(key, Some(value.clone()));
		Ok(())
	}

	pub async fn get_item(&self, key: &str) -> Result<JsValue, JsValue> {
		let tx = self.database.transaction_with_str_and_mode(&self.store, IdbTransactionMode::Readonly)?;
		let store = tx.object_store(&self.store)?;
		let request: IdbRequest = store.get(&JsValue::from_str(key))?;
		let result = request_to_future(&request).await?;
		if result.is_undefined() {
			Err(JsValue::from_str("Unable to read undefined value."))
		} else {
			Ok(result)
		}
	}

	async fn has_item_internal(&self, key: &str) -> Result<bool, JsValue> {
		let tx = self.database.transaction_with_str_and_mode(&self.store, IdbTransactionMode::Readonly)?;
		let store = tx.object_store(&self.store)?;
		let request: IdbRequest = store.get(&JsValue::from_str(key))?;
		let result = request_to_future(&request).await?;
		Ok(!result.is_undefined())
	}

	pub async fn has_item(&self, key: &str) -> bool {
		if let Ok(result) = self.has_item_internal(key).await {
			return result;
		}

		false
	}

	pub async fn delete_item(&self, key: &str) -> Result<(), JsValue> {
		let tx = self.database.transaction_with_str_and_mode(&self.store, IdbTransactionMode::Readwrite)?;
		let store = tx.object_store(&self.store)?;
		let request: IdbRequest = store.delete(&JsValue::from_str(key))?;
		let _ = request_to_future(&request).await?;
		self.dispatch_event(key, None);
		Ok(())
	}
}
