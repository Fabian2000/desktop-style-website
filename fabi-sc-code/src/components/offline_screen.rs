use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct OfflineScreenProps {
    pub visible: bool,
}

#[function_component(OfflineScreen)]
pub fn offline_screen(props: &OfflineScreenProps) -> Html {
    let on_reload = Callback::from(|_| {
        if let Some(window) = web_sys::window() {
            if let Some(location) = window.location().dyn_ref::<web_sys::Location>() {
                let _ = location.reload();
            }
        }
    });

    let class = if props.visible {
        "offline-screen"
    } else {
        "offline-screen display-none"
    };

    html! {
        <div class={class} id="offline-screen">
            <div class="content">
                <div class="icon"></div>
                <h1>{"This site can't be reached"}</h1>
                <p>{"The remote desktop has been disconnected."}</p>
                <small>{"ERR_REMOTE_DISCONNECT"}</small>
                <button onclick={on_reload}>{"Reload"}</button>
            </div>
        </div>
    }
}
