use js_sys::Promise;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub fn request_to_future<R>(req: &R) -> JsFuture
where
	R: JsCast + Clone + 'static,
{
	let promise = Promise::new(&mut |resolve, reject| {
		let success = {
			let resolve = resolve.clone();
			Closure::once(move |event: web_sys::Event| {
				if let Some(target) = event.target() {
					let _ = resolve.call1(&JsValue::NULL, &target);
				}
			})
		};
		req.clone()
			.unchecked_ref::<web_sys::EventTarget>()
			.add_event_listener_with_callback("success", success.as_ref().unchecked_ref())
			.ok();
		success.forget();

		let error = {
			let reject = reject.clone();
			Closure::once(move |event: web_sys::Event| {
				let _ = reject.call1(&JsValue::NULL, &event.into());
			})
		};
		req.clone()
			.unchecked_ref::<web_sys::EventTarget>()
			.add_event_listener_with_callback("error", error.as_ref().unchecked_ref())
			.ok();
		error.forget();
	});
	JsFuture::from(promise)
}