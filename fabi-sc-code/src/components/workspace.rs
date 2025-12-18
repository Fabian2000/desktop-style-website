use yew::prelude::*;

use super::desktop_widgets::DesktopWidgets;

#[derive(Properties, PartialEq)]
pub struct WorkspaceProps {
    pub visible: bool,
    #[prop_or_default]
    pub children: Html,
}

#[function_component(Workspace)]
pub fn workspace(props: &WorkspaceProps) -> Html {
    if !props.visible {
        return html! {};
    }

    html! {
        <div class="workspace">
            <DesktopWidgets />
            <div class="workspace-content">
                {props.children.clone()}
            </div>
        </div>
    }
}
