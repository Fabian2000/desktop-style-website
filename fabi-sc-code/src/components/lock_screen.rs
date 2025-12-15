use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LockScreenProps {
    pub visible: bool,
    pub on_login: Callback<()>,
}

#[function_component(LockScreen)]
pub fn lock_screen(props: &LockScreenProps) -> Html {
    let fading_out = use_state(|| false);
    let hidden = use_state(|| false);

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
