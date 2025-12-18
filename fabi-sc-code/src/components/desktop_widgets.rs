use gloo_timers::callback::Interval;
use yew::prelude::*;

use crate::utils::{get_local_date, get_local_time_no_sec};

#[function_component(DesktopWidgets)]
pub fn desktop_widgets() -> Html {
    let time = use_state(|| get_local_time_no_sec());
    let date = use_state(|| get_local_date());

    // Update time every second
    {
        let time = time.clone();
        let date = date.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                time.set(get_local_time_no_sec());
                date.set(get_local_date());
            });
            || drop(interval)
        });
    }

    html! {
        <div class="desktop-widgets">
            <div class="clock-widget">
                <div class="clock-time">{&*time}</div>
                <div class="clock-date">{&*date}</div>
            </div>
        </div>
    }
}
