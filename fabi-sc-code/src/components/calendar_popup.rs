use gloo_timers::callback::Interval;
use js_sys::Date;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct CalendarPopupProps {
    pub visible: bool,
    pub open: bool,
}

#[function_component(CalendarPopup)]
pub fn calendar_popup(props: &CalendarPopupProps) -> Html {
    let current_time = use_state(|| get_current_time());
    let current_date = use_state(|| get_current_date());
    let displayed_month = use_state(|| (get_current_year(), get_current_month()));

    // Update time every second
    {
        let current_time = current_time.clone();
        let current_date = current_date.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1000, move || {
                current_time.set(get_current_time());
                current_date.set(get_current_date());
            });
            || drop(interval)
        });
    }

    // Reset to current month when popup opens
    {
        let displayed_month = displayed_month.clone();
        let open = props.open;
        use_effect_with(open, move |open| {
            if *open {
                displayed_month.set((get_current_year(), get_current_month()));
            }
            || ()
        });
    }

    let on_prev_month = {
        let displayed_month = displayed_month.clone();
        Callback::from(move |_: MouseEvent| {
            let (year, month) = *displayed_month;
            if month == 0 {
                displayed_month.set((year - 1, 11));
            } else {
                displayed_month.set((year, month - 1));
            }
        })
    };

    let on_next_month = {
        let displayed_month = displayed_month.clone();
        Callback::from(move |_: MouseEvent| {
            let (year, month) = *displayed_month;
            if month == 11 {
                displayed_month.set((year + 1, 0));
            } else {
                displayed_month.set((year, month + 1));
            }
        })
    };

    if !props.visible {
        return html! {};
    }

    let popup_class = if props.open {
        "calendar-popup open"
    } else {
        "calendar-popup"
    };

    let (display_year, display_month) = *displayed_month;
    let month_name = get_month_name(display_month);
    let calendar_days = generate_calendar_days(display_year, display_month);
    let today = (get_current_year(), get_current_month(), get_current_day());

    html! {
        <div class={popup_class}>
            <div class="calendar-time-display">
                <div class="large-time">{(*current_time).clone()}</div>
                <div class="current-date">{(*current_date).clone()}</div>
            </div>
            <hr />
            <div class="calendar-header">
                <button class="calendar-nav-btn" onclick={on_prev_month}>
                    <i class="fa-solid fa-chevron-left"></i>
                </button>
                <span class="calendar-month-year">{format!("{} {}", month_name, display_year)}</span>
                <button class="calendar-nav-btn" onclick={on_next_month}>
                    <i class="fa-solid fa-chevron-right"></i>
                </button>
            </div>
            <div class="calendar-grid">
                <div class="calendar-weekday">{"Mon"}</div>
                <div class="calendar-weekday">{"Tue"}</div>
                <div class="calendar-weekday">{"Wed"}</div>
                <div class="calendar-weekday">{"Thu"}</div>
                <div class="calendar-weekday">{"Fri"}</div>
                <div class="calendar-weekday">{"Sat"}</div>
                <div class="calendar-weekday">{"Sun"}</div>
                { for calendar_days.iter().map(|day| {
                    let is_today = day.is_current_month
                        && display_year == today.0
                        && display_month == today.1
                        && day.day == today.2;
                    let mut class = String::from("calendar-day");
                    if !day.is_current_month {
                        class.push_str(" other-month");
                    }
                    if is_today {
                        class.push_str(" today");
                    }
                    html! {
                        <div class={class}>{day.day}</div>
                    }
                })}
            </div>
        </div>
    }
}

struct CalendarDay {
    day: u32,
    is_current_month: bool,
}

fn get_current_time() -> String {
    let date = Date::new_0();
    let hours = date.get_hours();
    let minutes = date.get_minutes();
    let seconds = date.get_seconds();
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn get_current_date() -> String {
    let date = Date::new_0();
    let day = date.get_date();
    let month = get_month_name(date.get_month());
    let year = date.get_full_year();
    let weekday = get_weekday_name(date.get_day());
    format!("{}, {}. {} {}", weekday, day, month, year)
}

fn get_current_year() -> i32 {
    Date::new_0().get_full_year() as i32
}

fn get_current_month() -> u32 {
    Date::new_0().get_month()
}

fn get_current_day() -> u32 {
    Date::new_0().get_date()
}

fn get_month_name(month: u32) -> &'static str {
    match month {
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
        _ => "Unknown",
    }
}

fn get_weekday_name(day: u32) -> &'static str {
    match day {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    // Use JS Date to get days in month
    // Day 0 of next month = last day of current month
    let next_month = if month == 11 { 0 } else { month + 1 };
    let next_year = if month == 11 { year + 1 } else { year };
    let date = Date::new_with_year_month_day(next_year as u32, next_month as i32, 0);
    date.get_date()
}

fn first_day_of_month(year: i32, month: u32) -> u32 {
    let date = Date::new_with_year_month_day(year as u32, month as i32, 1);
    let day = date.get_day();
    // Convert Sunday=0 to Monday=0 format
    if day == 0 { 6 } else { day - 1 }
}

fn generate_calendar_days(year: i32, month: u32) -> Vec<CalendarDay> {
    let mut days = Vec::new();

    let days_in_current = days_in_month(year, month);
    let first_day = first_day_of_month(year, month);

    // Previous month days
    let prev_month = if month == 0 { 11 } else { month - 1 };
    let prev_year = if month == 0 { year - 1 } else { year };
    let days_in_prev = days_in_month(prev_year, prev_month);

    for i in 0..first_day {
        days.push(CalendarDay {
            day: days_in_prev - first_day + i + 1,
            is_current_month: false,
        });
    }

    // Current month days
    for day in 1..=days_in_current {
        days.push(CalendarDay {
            day,
            is_current_month: true,
        });
    }

    // Next month days to fill remaining cells (6 rows * 7 = 42)
    let remaining = 42 - days.len();
    for day in 1..=remaining {
        days.push(CalendarDay {
            day: day as u32,
            is_current_month: false,
        });
    }

    days
}
