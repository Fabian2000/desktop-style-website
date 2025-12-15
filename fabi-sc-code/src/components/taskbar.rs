use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TaskbarProps {
    pub visible: bool,
}

#[function_component(Taskbar)]
pub fn taskbar(props: &TaskbarProps) -> Html {
    if !props.visible {
        return html! {};
    }

    html! {
        <div class="taskbar">
            <button class=""></button>
        </div>
    }
}
