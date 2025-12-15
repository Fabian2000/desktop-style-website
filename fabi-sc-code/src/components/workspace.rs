use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct WorkspaceProps {
    pub visible: bool,
}

#[function_component(Workspace)]
pub fn workspace(props: &WorkspaceProps) -> Html {
    if !props.visible {
        return html! {};
    }

    html! {
        <div class="workspace">
        </div>
    }
}
