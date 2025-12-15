use yew::prelude::*;

#[allow(dead_code)]
#[derive(Properties, PartialEq)]
pub struct BootScreenProps {
    pub visible: bool,
}

#[function_component(BootScreen)]
pub fn boot_screen(props: &BootScreenProps) -> Html {
    // Sync the static boot screen visibility with props
    {
        let visible = props.visible;
        use_effect_with(visible, move |visible| {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(static_boot) = document.get_element_by_id("static-boot-screen") {
                        if *visible {
                            let _ = static_boot.class_list().remove_1("display-none");
                        } else {
                            let _ = static_boot.class_list().add_1("display-none");
                        }
                    }
                }
            }
            || ()
        });
    }

    html! {}
}
