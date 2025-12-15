use gloo_timers::callback::Interval;
use js_sys::Date;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::TouchEvent;
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
    let current_time = use_state(|| get_current_time());
    let current_date = use_state(|| get_current_date());
    let is_mobile = use_state(|| is_portrait_mode());

    // Swipe state for mobile
    let touch_start_y = use_state(|| 0.0f64);
    let swipe_offset = use_state(|| 0.0f64);
    let is_swiping = use_state(|| false);

    // Update time every second
    {
        let current_time = current_time.clone();
        let current_date = current_date.clone();
        let is_mobile = is_mobile.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                current_time.set(get_current_time());
                current_date.set(get_current_date());
                is_mobile.set(is_portrait_mode());
            });
            || drop(interval)
        });
    }

    // Unlock function
    let do_unlock = {
        let fading_out = fading_out.clone();
        let hidden = hidden.clone();
        let on_login = props.on_login.clone();

        Callback::from(move |_: ()| {
            if *fading_out {
                return;
            }
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

    // Click anywhere to unlock (desktop)
    let on_click = {
        let do_unlock = do_unlock.clone();
        let is_mobile = is_mobile.clone();
        Callback::from(move |_: MouseEvent| {
            if !*is_mobile {
                do_unlock.emit(());
            }
        })
    };

    // Keyboard listener for any key press to unlock
    {
        let do_unlock = do_unlock.clone();
        let visible = props.visible;
        let fading_out_clone = fading_out.clone();
        let hidden_clone = hidden.clone();

        use_effect_with((visible, *fading_out_clone, *hidden_clone), move |(visible, fading, hidden)| {
            let closure: Rc<RefCell<Option<Closure<dyn Fn(web_sys::KeyboardEvent)>>>> =
                Rc::new(RefCell::new(None));
            let closure_clone = closure.clone();

            if *visible && !*fading && !*hidden {
                let do_unlock = do_unlock.clone();
                let keydown_closure = Closure::wrap(Box::new(move |_e: web_sys::KeyboardEvent| {
                    do_unlock.emit(());
                }) as Box<dyn Fn(web_sys::KeyboardEvent)>);

                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        let _ = document.add_event_listener_with_callback(
                            "keydown",
                            keydown_closure.as_ref().unchecked_ref(),
                        );
                    }
                }

                *closure_clone.borrow_mut() = Some(keydown_closure);
            }

            move || {
                if let Some(closure) = closure.borrow_mut().take() {
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        let _ = document.remove_event_listener_with_callback(
                            "keydown",
                            closure.as_ref().unchecked_ref(),
                        );
                    }
                }
            }
        });
    }

    // Touch handlers for swipe-to-unlock (mobile)
    let on_touch_start = {
        let touch_start_y = touch_start_y.clone();
        let is_swiping = is_swiping.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.touches().get(0) {
                touch_start_y.set(touch.client_y() as f64);
                is_swiping.set(true);
            }
        })
    };

    let on_touch_move = {
        let touch_start_y = touch_start_y.clone();
        let swipe_offset = swipe_offset.clone();
        let is_swiping = is_swiping.clone();
        Callback::from(move |e: TouchEvent| {
            if !*is_swiping {
                return;
            }
            if let Some(touch) = e.touches().get(0) {
                let current_y = touch.client_y() as f64;
                let delta = *touch_start_y - current_y;
                // Only allow upward swipe (positive delta)
                if delta > 0.0 {
                    swipe_offset.set(delta.min(200.0));
                }
            }
        })
    };

    let on_touch_end = {
        let swipe_offset = swipe_offset.clone();
        let is_swiping = is_swiping.clone();
        let do_unlock = do_unlock.clone();
        Callback::from(move |_: TouchEvent| {
            is_swiping.set(false);
            // If swiped more than 100px, unlock
            if *swipe_offset > 100.0 {
                do_unlock.emit(());
            }
            swipe_offset.set(0.0);
        })
    };

    if !props.visible || *hidden {
        return html! {};
    }

    let class = if *fading_out {
        "lock-screen fade-out"
    } else if *is_swiping && *swipe_offset > 0.0 {
        "lock-screen swiping"
    } else {
        "lock-screen"
    };

    // Apply swipe transform
    let swipe_style = if *is_swiping && *swipe_offset > 0.0 {
        let opacity = 1.0 - (*swipe_offset / 200.0);
        let scale = 1.0 + (*swipe_offset / 1000.0);
        format!("opacity: {}; transform: scale({});", opacity, scale)
    } else {
        String::new()
    };

    // Hint text based on device
    let hint_text = if *is_mobile {
        "Swipe up to unlock"
    } else {
        "Press any key or click to unlock"
    };

    html! {
        <div
            class={class}
            style={swipe_style}
            onclick={on_click}
            ontouchstart={on_touch_start}
            ontouchmove={on_touch_move}
            ontouchend={on_touch_end}
        >
            <div class="lock-screen-time">
                <span class="time">{(*current_time).clone()}</span>
                <span class="date">{(*current_date).clone()}</span>
            </div>
            if *is_mobile {
                // Mobile: minimalist - just swipe hint at bottom
                <div class="lock-screen-card">
                    <div></div>
                    <div class="unlock-hint">
                        <i class="fa-solid fa-chevron-up"></i>
                        <span>{"Swipe up to unlock"}</span>
                    </div>
                </div>
            } else {
                // Desktop: full card with profile, welcome, button
                <div class="lock-screen-card">
                    <div class="profile-section">
                        <div class="profile-picture"></div>
                    </div>
                    <div class="welcome-msg">
                        <p>{"Hello World"}</p>
                    </div>
                    <div class="login-section">
                        <button class="login-btn" type="button">
                            {"Enter"}
                        </button>
                    </div>
                </div>
            }
        </div>
    }
}

fn is_portrait_mode() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(orientation: portrait)").ok().flatten())
        .map(|mq| mq.matches())
        .unwrap_or(false)
}

fn get_current_time() -> String {
    let date = Date::new_0();
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    format!("{:02}:{:02}", hours, minutes)
}

fn get_current_date() -> String {
    let date = Date::new_0();
    let day = date.get_date();
    let month = date.get_month();
    let year = date.get_full_year();
    let weekday = date.get_day();

    let weekday_name = match weekday {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "",
    };

    let month_name = match month {
        0 => "January",
        1 => "February",
        2 => "March",
        3 => "April",
        4 => "May",
        5 => "June",
        6 => "July",
        7 => "August",
        8 => "September",
        9 => "October",
        10 => "November",
        11 => "December",
        _ => "",
    };

    format!("{}, {} {}", weekday_name, month_name, day)
}
