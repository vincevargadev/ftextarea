//! ftextarea - A simple full-screen textarea with localStorage persistence
//!
//! This crate provides the WebAssembly logic for the ftextarea web app.
//! It handles:
//! - Loading and saving text content to localStorage
//! - Debounced auto-save on input changes
//! - Multi-tab synchronization via storage events
//! - Modal dialog management

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{
    console, Document, Element, HtmlDialogElement, HtmlTextAreaElement, Window,
};

// ============================================================================
// Constants
// ============================================================================

/// Key used to store the textarea content in localStorage
const STORAGE_KEY: &str = "ftextarea_content";

/// Debounce delay in milliseconds for auto-save
const DEBOUNCE_MS: i32 = 250;

// ============================================================================
// Storage Trait & Implementation
// ============================================================================

/// Trait for text storage operations.
///
/// This abstraction allows for different storage backends and makes
/// the code testable by enabling mock implementations.
pub trait TextStorage {
    /// Loads the stored content, returning an empty string if none exists.
    fn load(&self) -> String;

    /// Saves content to storage.
    fn save(&self, content: &str);
}

/// Browser localStorage implementation of TextStorage.
///
/// Wraps the Web Storage API with a cleaner, more idiomatic Rust interface.
pub struct LocalStorage {
    storage: web_sys::Storage,
    key: &'static str,
}

impl LocalStorage {
    /// Creates a new LocalStorage instance for the given key.
    ///
    /// Returns `None` if localStorage is unavailable (e.g., private browsing mode).
    pub fn new(key: &'static str) -> Option<Self> {
        let window = web_sys::window()?;
        let storage = window.local_storage().ok().flatten()?;
        Some(Self { storage, key })
    }
}

impl TextStorage for LocalStorage {
    fn load(&self) -> String {
        self.storage
            .get_item(self.key)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn save(&self, content: &str) {
        let _ = self.storage.set_item(self.key, content);
    }
}

// ============================================================================
// DOM Helpers
// ============================================================================

/// Retrieves the Window object.
fn window() -> Window {
    web_sys::window().expect("no global window exists")
}

/// Retrieves the Document object.
fn document() -> Document {
    window().document().expect("no document on window")
}

/// Gets an element by ID and casts it to the specified type.
fn get_element<T: JsCast>(id: &str) -> Option<T> {
    document()
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<T>().ok())
}

// ============================================================================
// Debounce Timer
// ============================================================================

/// A simple debounce timer that delays function execution.
///
/// Uses `Rc<RefCell<...>>` to allow the closure to be called multiple times
/// while maintaining mutable state (the timeout handle).
struct Debouncer {
    timeout_handle: Rc<RefCell<Option<i32>>>,
}

impl Debouncer {
    fn new() -> Self {
        Self {
            timeout_handle: Rc::new(RefCell::new(None)),
        }
    }

    /// Schedules a function to run after the debounce delay.
    ///
    /// If called again before the delay expires, the previous timer is cancelled.
    fn schedule<F>(&self, delay_ms: i32, callback: F)
    where
        F: FnOnce() + 'static,
    {
        // Cancel any existing timer
        self.cancel();

        let handle_ref = self.timeout_handle.clone();

        // Create a closure that will be called after the timeout
        let closure = Closure::once(Box::new(move || {
            callback();
            // Clear the handle after execution
            *handle_ref.borrow_mut() = None;
        }) as Box<dyn FnOnce()>);

        // Schedule the timeout
        let handle = window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                delay_ms,
            )
            .expect("failed to set timeout");

        *self.timeout_handle.borrow_mut() = Some(handle);

        // Prevent the closure from being dropped (which would invalidate the callback)
        closure.forget();
    }

    /// Cancels any pending debounced function.
    fn cancel(&self) {
        if let Some(handle) = self.timeout_handle.borrow_mut().take() {
            window().clear_timeout_with_handle(handle);
        }
    }
}

// ============================================================================
// Application Logic
// ============================================================================

/// Sets up an event listener on an element.
///
/// This is a helper to reduce boilerplate when attaching multiple event handlers.
fn add_event_listener<T, F>(target: &T, event_type: &str, handler: F)
where
    T: AsRef<web_sys::EventTarget>,
    F: FnMut(web_sys::Event) + 'static,
{
    let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
    target
        .as_ref()
        .add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())
        .expect("failed to add event listener");
    closure.forget();
}

/// Initializes the ftextarea application.
///
/// This is the main entry point called from JavaScript.
#[wasm_bindgen(start)]
pub fn init() {
    // Set up panic hook for better error messages in the console
    console_error_panic_hook::set_once();

    // Initialize storage
    let storage = Rc::new(
        LocalStorage::new(STORAGE_KEY).expect("localStorage not available")
    );

    // Get DOM elements
    let editor: HtmlTextAreaElement = get_element("editor").expect("editor element not found");
    let info_btn: Element = get_element("info-btn").expect("info-btn element not found");
    let close_btn: Element = get_element("close-modal").expect("close-modal element not found");
    let modal: HtmlDialogElement = get_element("info-modal").expect("info-modal element not found");

    // Load initial content from localStorage
    let initial_content = storage.load();
    editor.set_value(&initial_content);

    console::log_1(&"ftextarea initialized".into());

    // Set up debounced auto-save on input
    let debouncer = Rc::new(Debouncer::new());
    {
        let editor_clone = editor.clone();
        let debouncer_clone = debouncer.clone();
        let storage_clone = storage.clone();

        add_event_listener(&editor, "input", move |_| {
            let editor_ref = editor_clone.clone();
            let storage_ref = storage_clone.clone();
            debouncer_clone.schedule(DEBOUNCE_MS, move || {
                let content = editor_ref.value();
                storage_ref.save(&content);
            });
        });
    }

    // Save immediately when the page becomes hidden (tab switch, minimize, etc.)
    {
        let editor_clone = editor.clone();
        let debouncer_clone = debouncer.clone();
        let storage_clone = storage.clone();

        add_event_listener(&document(), "visibilitychange", move |_| {
            if document().visibility_state() == web_sys::VisibilityState::Hidden {
                // Cancel pending debounce and save immediately
                debouncer_clone.cancel();
                storage_clone.save(&editor_clone.value());
            }
        });
    }

    // Listen for storage events from other tabs
    {
        let editor_clone = editor.clone();

        add_event_listener(&window(), "storage", move |event| {
            let storage_event = event.dyn_into::<web_sys::StorageEvent>().unwrap();
            if storage_event.key().as_deref() == Some(STORAGE_KEY) {
                if let Some(new_value) = storage_event.new_value() {
                    editor_clone.set_value(&new_value);
                }
            }
        });
    }

    // Save when the page is about to unload
    {
        let editor_clone = editor.clone();
        let storage_clone = storage.clone();

        add_event_listener(&window(), "beforeunload", move |_| {
            storage_clone.save(&editor_clone.value());
        });
    }

    // Modal: open on info button click
    {
        let modal_clone = modal.clone();

        add_event_listener(&info_btn, "click", move |_| {
            modal_clone.show_modal().expect("failed to show modal");
        });
    }

    // Modal: close on close button click
    {
        let modal_clone = modal.clone();

        add_event_listener(&close_btn, "click", move |_| {
            modal_clone.close();
        });
    }

    // Modal: close on backdrop click (clicking outside the modal content)
    {
        let modal_clone = modal.clone();

        add_event_listener(&modal, "click", move |event| {
            // The modal element itself is the backdrop; article is the content
            // If the click target is the dialog (not its children), close it
            if let Some(target) = event.target() {
                if let Ok(el) = target.dyn_into::<Element>() {
                    if el.tag_name() == "DIALOG" {
                        modal_clone.close();
                    }
                }
            }
        });
    }

    // Modal: close on Escape key (browsers handle this automatically for <dialog>,
    // but we add it for consistency)
    add_event_listener(&modal, "cancel", move |_| {
        // The cancel event fires when Escape is pressed; dialog closes automatically
    });
}

// ============================================================================
// Panic Hook (for better error messages)
// ============================================================================

mod console_error_panic_hook {
    use std::panic;
    use std::sync::Once;

    static SET_HOOK: Once = Once::new();

    /// Sets a panic hook that logs panic info to the browser console.
    ///
    /// This only sets the hook once, even if called multiple times.
    pub fn set_once() {
        SET_HOOK.call_once(|| {
            panic::set_hook(Box::new(|info| {
                let msg = info.to_string();
                web_sys::console::error_1(&msg.into());
            }));
        });
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock storage for testing purposes.
    struct MockStorage {
        content: RefCell<String>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                content: RefCell::new(String::new()),
            }
        }

        fn with_content(initial: &str) -> Self {
            Self {
                content: RefCell::new(initial.to_string()),
            }
        }
    }

    impl TextStorage for MockStorage {
        fn load(&self) -> String {
            self.content.borrow().clone()
        }

        fn save(&self, content: &str) {
            *self.content.borrow_mut() = content.to_string();
        }
    }

    #[test]
    fn test_mock_storage_load_empty() {
        let storage = MockStorage::new();
        assert_eq!(storage.load(), "");
    }

    #[test]
    fn test_mock_storage_save_and_load() {
        let storage = MockStorage::new();
        storage.save("Hello, world!");
        assert_eq!(storage.load(), "Hello, world!");
    }

    #[test]
    fn test_mock_storage_with_initial_content() {
        let storage = MockStorage::with_content("Initial content");
        assert_eq!(storage.load(), "Initial content");
    }

    #[test]
    fn test_mock_storage_overwrite() {
        let storage = MockStorage::with_content("Old content");
        storage.save("New content");
        assert_eq!(storage.load(), "New content");
    }
}
