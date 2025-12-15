use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LockScreenProps {
    pub visible: bool,
    pub boot_complete: bool,
    pub on_login: Callback<()>,
}

#[function_component(LockScreen)]
pub fn lock_screen(props: &LockScreenProps) -> Html {
    let fading_out = use_state(|| false);
    let hidden = use_state(|| false);

    // Focus the login button when boot completes
    {
        let boot_complete = props.boot_complete;
        use_effect_with(boot_complete, move |boot_complete| {
            if *boot_complete {
                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        &wasm_bindgen::closure::Closure::once_into_js(move || {
                            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                                if let Some(btn) = document.get_element_by_id("lock-screen-login") {
                                    if let Some(html_btn) = btn.dyn_ref::<web_sys::HtmlElement>() {
                                        let _ = html_btn.focus();
                                    }
                                }
                            }
                        })
                        .unchecked_ref(),
                        100,
                    );
                }
            }
            || ()
        });
    }

    let on_click = {
        let fading_out = fading_out.clone();
        let hidden = hidden.clone();
        let on_login = props.on_login.clone();

        Callback::from(move |_| {
            fading_out.set(true);

            let hidden = hidden.clone();
            let on_login = on_login.clone();

            gloo_timers::callback::Timeout::new(1000, move || {
                hidden.set(true);
                on_login.emit(());
            })
            .forget();
        })
    };

    if !props.visible || *hidden {
        return html! {};
    }

    let class = if *fading_out {
        "lock-screen fade-out"
    } else {
        "lock-screen"
    };

    html! {
        <div class={class}>
            <div class="profile-section">
                <div class="profile-picture"></div>
            </div>
            <div class="welcome-msg">
                <p>{"Hello World"}</p>
            </div>
            <div class="login-section">
                <button
                    class="login-btn"
                    id="lock-screen-login"
                    type="button"
                    autofocus=true
                    onclick={on_click}
                >
                    {"Enter"}
                </button>
            </div>
        </div>
    }
}
