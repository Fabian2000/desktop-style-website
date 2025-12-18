use gloo_timers::callback::Timeout;
use wasm_bindgen::JsCast;
use yew::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum ShutdownPhase {
    FadeOut,    // Fade desktop to black with spinner
    Black,      // Hold black screen
    Done,       // Shutdown complete - trigger callback
}

#[derive(Properties, PartialEq)]
pub struct ShutdownScreenProps {
    pub visible: bool,
    pub is_restart: bool,
    pub on_shutdown_complete: Callback<()>,  // Called when shutdown animation is done
}

#[function_component(ShutdownScreen)]
pub fn shutdown_screen(props: &ShutdownScreenProps) -> Html {
    let phase = use_state(|| ShutdownPhase::FadeOut);

    // Phase progression - triggered when visible becomes true
    {
        let phase = phase.clone();
        let is_restart = props.is_restart;
        let visible = props.visible;
        let on_shutdown_complete = props.on_shutdown_complete.clone();

        use_effect_with((visible, is_restart), move |(visible, is_restart)| {
            // Reset phase when becoming invisible
            if !*visible {
                phase.set(ShutdownPhase::FadeOut);
                return;
            }

            let is_restart = *is_restart;
            let phase_clone = phase.clone();
            let on_shutdown_complete = on_shutdown_complete.clone();

            // Start the shutdown sequence - fade to black over 1.5s
            let timeout1 = Timeout::new(1500, move || {
                phase_clone.set(ShutdownPhase::Black);

                let phase_clone2 = phase_clone.clone();
                let on_shutdown_complete = on_shutdown_complete.clone();

                // After 2s black screen
                let timeout2 = Timeout::new(2000, move || {
                    if is_restart {
                        // Restart: trigger page reload
                        if let Some(window) = web_sys::window() {
                            if let Some(location) = window.location().dyn_ref::<web_sys::Location>() {
                                let _ = location.reload();
                            }
                        }
                    } else {
                        // Shutdown: signal completion to show OfflineScreen
                        phase_clone2.set(ShutdownPhase::Done);
                        on_shutdown_complete.emit(());
                    }
                });
                timeout2.forget();
            });
            timeout1.forget();
        });
    }

    if !props.visible {
        return html! {};
    }

    // Don't render anything in Done phase - OfflineScreen takes over
    if *phase == ShutdownPhase::Done {
        return html! {};
    }

    let phase_class = match *phase {
        ShutdownPhase::FadeOut => "shutdown-screen fade-out",
        ShutdownPhase::Black => "shutdown-screen black",
        ShutdownPhase::Done => "shutdown-screen",
    };

    html! {
        <div class={phase_class}>
            <div class="shutdown-overlay"></div>

            if matches!(*phase, ShutdownPhase::FadeOut) {
                <div class="shutdown-content">
                    <div class="shutdown-spinner"></div>
                    <p class="shutdown-text">
                        { if props.is_restart { "Restarting..." } else { "Shutting down..." } }
                    </p>
                </div>
            }
        </div>
    }
}
