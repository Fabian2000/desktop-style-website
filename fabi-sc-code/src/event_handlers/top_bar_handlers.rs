use std::cell::RefCell;

use gloo_timers::callback::Timeout;
use wasm_bindgen::{prelude::*, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{console, window, Document, Element, Event, HtmlInputElement};

use crate::database::indexed_db::IndexedDb;

thread_local! {
    static TIMEOUT: RefCell<Option<Timeout>> = RefCell::new(None);
}

pub fn toggle_wifi_audio_popup() {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };

    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };

    let Ok(Some(wifi_audio_popup)) = document.query_selector(".wifi-speaker-popup") else {
        console::error_1(&JsValue::from_str("An error happened while trying to get the wifi-audio popup."));
        return;
    };

    if wifi_audio_popup.class_list().contains("open") {
        _ = wifi_audio_popup.class_list().remove_1("open");
        TIMEOUT.with_borrow_mut(|c| {
            let timeout = Timeout::new(250, move || {
                _ = wifi_audio_popup.class_list().add_1("display-none");
                _ = wifi_audio_popup.class_list().remove_1("display-block");
            });
            *c = Some(timeout);
        });
    }
    else {
        TIMEOUT.with(|c| {
            if let Some(timeout) = c.borrow_mut().take() {
                timeout.cancel();
            }
        });

        _ = wifi_audio_popup.class_list().remove_1("display-none");
        _ = wifi_audio_popup.class_list().add_1("display-block");
        
        _ = window.request_animation_frame(Closure::once_into_js(move || {
            _ = wifi_audio_popup.class_list().add_1("open");
        }).unchecked_ref());
    }
}

pub fn hide_wifi_audio_popup(e: Event) {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };
    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };
    let Ok(Some(wifi_audio_popup)) = document.query_selector(".wifi-speaker-popup") else {
        console::error_1(&JsValue::from_str("An error happened while trying to get the wifi-audio popup."));
        return;
    };
    let Ok(Some(open_btn)) = document.query_selector("#top-bar-wifi-audio-btn") else {
        console::error_1(&JsValue::from_str("An error happened while trying to get the wifi-audio popup."));
        return;
    };

    let path = e.composed_path();
    for i in 0..path.length() {
        if let Some(el) = path.get(i).dyn_ref::<Element>() {
            if wifi_audio_popup.contains(Some(el)) {
                return;
            }
            if open_btn.contains(Some(el)) {
                return;
            }
        }
    }

    if wifi_audio_popup.class_list().contains("open") {
        _ = wifi_audio_popup.class_list().remove_1("open");
        hide_wifi_disconnect(&document);

        TIMEOUT.with_borrow_mut(|c| {
            let timeout = Timeout::new(250, move || {
                _ = wifi_audio_popup.class_list().add_1("display-none");
                _ = wifi_audio_popup.class_list().remove_1("display-block");
            });
            *c = Some(timeout);
        });
    }
}

pub fn show_wifi_disconnect() {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };
    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };
    
    if let Ok(Some(wifi_extended_menu)) = document.query_selector("#wifi-extended-menu")
    {
        _ = wifi_extended_menu.class_list().add_1("display-none");
    }
    else {
        console::error_1(&JsValue::from_str("Unable to get the wifi extended menu button."));
    }

    if let Ok(Some(wifi_disconnect)) = document.query_selector("#wifi-disconnect")
    {
        _ = wifi_disconnect.class_list().remove_1("display-none");
    }
    else {
        console::error_1(&JsValue::from_str("Unable to get the wifi disconnect button."));
    }
}

fn hide_wifi_disconnect(document: &Document) {
    if let Ok(Some(wifi_extended_menu)) = document.query_selector("#wifi-extended-menu")
    {
        _ = wifi_extended_menu.class_list().remove_1("display-none");
    }
    else {
        console::error_1(&JsValue::from_str("Unable to get the wifi extended menu button."));
    }

    if let Ok(Some(wifi_disconnect)) = document.query_selector("#wifi-disconnect")
    {
        _ = wifi_disconnect.class_list().add_1("display-none");
    }
    else {
        console::error_1(&JsValue::from_str("Unable to get the wifi disconnect button."));
    }
}

pub fn wifi_disconnect() {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };
    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };
    
    if let Ok(Some(body)) = document.query_selector("body") {
        let all_children = (0..body.children().length())
            .filter_map(| i | body.children().item(i))
            .filter(| item | item.id() != "offline-screen")
            .collect::<Vec<Element>>();
        
        for child in all_children {
            _ = child.class_list().add_1("display-none");
        }
    }
    else {
        console::error_1(&JsValue::from_str("Unable to get the html body."));
    }

    if let Ok(Some(wifi_disconnect)) = document.query_selector("#offline-screen")
    {
        _ = wifi_disconnect.class_list().remove_1("display-none");
    }
    else {
        console::error_1(&JsValue::from_str("Unable to get the offline-screen."));
    }
}

pub fn save_volume_slider() {
    let Ok(window) = window().ok_or("Unable to get the window.") else {
        console::error_1(&JsValue::from_str("Unable to get the window."));
        return;
    };
    let Ok(document) = window.document().ok_or("Unable to get the document.") else {
        console::error_1(&JsValue::from_str("Unable to get the document."));
        return;
    };
    
    if let Ok(Some(slider)) = document.query_selector("#volume-slider") {
        let Ok(slider) = slider.dyn_into::<HtmlInputElement>() else {
            console::error_1(&JsValue::from_str("Unable to convert the slider to input."));
            return;
        };

        let value = slider.value();
        let Ok(value_i32) = value.parse::<i32>() else {
            console::error_1(&JsValue::from_str("Unable to parse slider value string to int."));
            return;
        };

        spawn_local(async move {
            let Ok(db) = IndexedDb::open("settings", "system_settings").await else {
                console::error_1(&JsValue::from_str("Unable to open database."));
                return;
            };

            db.register_listener_unique("top-bar-volume", |key, value| {
                if key == "volume" {
                    update_volume_slider(value);
                }
            });

            _ = db.set_item("volume", &JsValue::from(value_i32));
        });
    }
}

fn update_volume_slider(value: Option<JsValue>) {
	let Some(window) = web_sys::window() else {
		console::error_1(&JsValue::from_str("Unable to get window"));
		return;
	};
	let Some(document) = window.document() else {
		console::error_1(&JsValue::from_str("Unable to get document"));
		return;
	};
	let Ok(Some(slider)) = document.query_selector("#volume-slider") else {
		console::error_1(&JsValue::from_str("Unable to find #volume-slider"));
		return;
	};
	let Ok(slider) = slider.dyn_into::<HtmlInputElement>() else {
		console::error_1(&JsValue::from_str("Unable to cast element to HtmlInputElement"));
		return;
	};

	if let Some(value) = value {
        if let Some(value) = value.as_f64() {
            let value = value.to_string();
		    slider.set_value(&value);
        }
	}
}

pub fn live_volume_slider() {
    let Some(window) = web_sys::window() else {
		console::error_1(&JsValue::from_str("Unable to get window"));
		return;
	};
	let Some(document) = window.document() else {
		console::error_1(&JsValue::from_str("Unable to get document"));
		return;
	};
	let Ok(Some(slider)) = document.query_selector("#volume-slider") else {
		console::error_1(&JsValue::from_str("Unable to get volume slider"));
		return;
	};
    let Ok(slider) = slider.dyn_into::<HtmlInputElement>() else {
		console::error_1(&JsValue::from_str("Unable to cast element to HtmlInputElement"));
		return;
	};
	let Ok(Some(display)) = document.query_selector("#volume-display") else {
		console::error_1(&JsValue::from_str("Unable to get volume display"));
		return;
	};

    display.set_text_content(Some(&format!("{} %", slider.value())));
}

pub fn init_volume_slider() {
    let Some(window) = web_sys::window() else {
		console::error_1(&JsValue::from_str("Unable to get window"));
		return;
	};
	let Some(document) = window.document() else {
		console::error_1(&JsValue::from_str("Unable to get document"));
		return;
	};
	let Ok(Some(slider)) = document.query_selector("#volume-slider") else {
		console::error_1(&JsValue::from_str("Unable to get volume slider"));
		return;
	};
    let Ok(slider) = slider.dyn_into::<HtmlInputElement>() else {
		console::error_1(&JsValue::from_str("Unable to cast element to HtmlInputElement"));
		return;
	};

    spawn_local(async move {
        let Ok(db) = IndexedDb::open("settings", "system_settings").await else {
            console::error_1(&JsValue::from_str("Unable to open database."));
            return;
        };

        if db.has_item("volume").await {
            let Ok(value) = db.get_item("volume").await else {
                console::error_1(&JsValue::from_str("Unable to get the database value for volume display."));
                return;
            };

            if let Some(value) = value.as_f64() {
                slider.set_value(&value.to_string());
            }
        }
    });
}