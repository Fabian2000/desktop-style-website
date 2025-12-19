//! Session Management Module
//!
//! Handles multi-tab detection using BroadcastChannel.
//! When a new tab opens, it takes over the session and old tabs
//! show a "connection lost" screen.
//!
//! This prevents data corruption from multiple tabs trying to
//! use the same VFS cache/IndexedDB simultaneously.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Channel name for session management
const CHANNEL_NAME: &str = "fabiscos-session";

/// Message types
const MSG_TAKEOVER: &str = "SESSION_TAKEOVER";

/// Unique session ID for this tab
fn generate_session_id() -> String {
    let timestamp = js_sys::Date::now() as u64;
    let random = (js_sys::Math::random() * 1_000_000.0) as u64;
    format!("{}-{}", timestamp, random)
}

/// Session state
struct SessionState {
    /// Unique ID for this session/tab
    session_id: String,
    /// Whether this session is active (not taken over)
    is_active: bool,
    /// Whether we've been taken over (for polling by Yew)
    was_taken_over: bool,
    /// The BroadcastChannel instance
    channel: Option<web_sys::BroadcastChannel>,
    /// Closure for message handler (must be kept alive)
    _message_handler: Option<Closure<dyn Fn(web_sys::MessageEvent)>>,
}

thread_local! {
    static SESSION: RefCell<SessionState> = RefCell::new(SessionState {
        session_id: String::new(),
        is_active: false,
        was_taken_over: false,
        channel: None,
        _message_handler: None,
    });
}

/// Initialize session management
/// This broadcasts a takeover message to any existing tabs.
/// Call `check_and_clear_takeover()` to check if this tab was taken over.
pub fn init_session() {
    SESSION.with(|state| {
        let mut state = state.borrow_mut();

        // Generate unique session ID
        state.session_id = generate_session_id();
        state.is_active = true;
        state.was_taken_over = false;

        // Create BroadcastChannel
        let channel = match web_sys::BroadcastChannel::new(CHANNEL_NAME) {
            Ok(ch) => ch,
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("[Session] BroadcastChannel not supported: {:?}", e).into(),
                );
                return; // Proceed without multi-tab support
            }
        };

        // Set up message handler
        let session_id = state.session_id.clone();
        let handler = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            handle_message(event, &session_id);
        }) as Box<dyn Fn(_)>);

        channel.set_onmessage(Some(handler.as_ref().unchecked_ref()));

        // Broadcast takeover message to existing tabs
        let takeover_msg = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &takeover_msg,
            &JsValue::from_str("type"),
            &JsValue::from_str(MSG_TAKEOVER),
        );
        let _ = js_sys::Reflect::set(
            &takeover_msg,
            &JsValue::from_str("sessionId"),
            &JsValue::from_str(&state.session_id),
        );

        if let Err(e) = channel.post_message(&takeover_msg) {
            web_sys::console::warn_1(&format!("[Session] Failed to post message: {:?}", e).into());
        }

        web_sys::console::log_1(
            &format!("[Session] Initialized with ID: {}", state.session_id).into(),
        );

        state.channel = Some(channel);
        state._message_handler = Some(handler);
    });
}

/// Handle incoming BroadcastChannel message
fn handle_message(event: web_sys::MessageEvent, our_session_id: &str) {
    let data = event.data();

    // Try to get message type
    let msg_type = js_sys::Reflect::get(&data, &JsValue::from_str("type"))
        .ok()
        .and_then(|v| v.as_string());

    let sender_id = js_sys::Reflect::get(&data, &JsValue::from_str("sessionId"))
        .ok()
        .and_then(|v| v.as_string());

    if let Some(MSG_TAKEOVER) = msg_type.as_deref() {
        // Another tab is taking over
        if let Some(sender) = sender_id {
            if sender != our_session_id {
                web_sys::console::log_1(
                    &format!("[Session] Received takeover from: {}", sender).into(),
                );
                // We're being replaced - mark as taken over
                mark_taken_over();
            }
        }
    }
}

/// Mark this session as taken over
fn mark_taken_over() {
    SESSION.with(|state| {
        let mut state = state.borrow_mut();

        if !state.is_active {
            return; // Already disconnected
        }

        state.is_active = false;
        state.was_taken_over = true;

        web_sys::console::log_1(&"[Session] This tab has been taken over by another tab".into());

        // Close the channel - we're done
        if let Some(channel) = state.channel.take() {
            channel.close();
        }
    });
}

/// Check if this session was taken over by another tab.
/// This is intended to be polled by Yew components.
/// Returns true once when taken over, then returns false (to prevent multiple triggers).
pub fn check_and_clear_takeover() -> bool {
    SESSION.with(|state| {
        let mut state = state.borrow_mut();
        if state.was_taken_over {
            state.was_taken_over = false; // Clear the flag
            true
        } else {
            false
        }
    })
}

/// Check if the session is still active
#[allow(dead_code)]
pub fn is_session_active() -> bool {
    SESSION.with(|state| state.borrow().is_active)
}

/// Clean up session (call on page unload)
#[allow(dead_code)]
pub fn cleanup_session() {
    SESSION.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(channel) = state.channel.take() {
            channel.close();
        }
        state.is_active = false;
    });
}
