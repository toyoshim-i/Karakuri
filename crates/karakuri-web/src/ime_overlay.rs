//! DOM-based IME input overlay providing native Japanese composition and
//! keystroke dispatching for the WebAssembly terminal.

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;
use winit::event_loop::EventLoopProxy;

use karakuri_console::egui::Rect;
use karakuri_console::view::prompt::{extract_selected_text, PromptState};

pub struct ImeOverlay {
    element: HtmlTextAreaElement,
    prompt_state: PromptState,
    _closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
}

impl ImeOverlay {
    /// Creates and mounts a transparent DOM `<textarea>` overlay configured for IME input.
    pub fn new(prompt_state: PromptState, proxy: EventLoopProxy<()>) -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;
        let body = document
            .body()
            .ok_or_else(|| JsValue::from_str("no document.body"))?;

        let textarea = document
            .create_element("textarea")?
            .dyn_into::<HtmlTextAreaElement>()?;

        textarea.set_id("karakuri-ime-overlay");
        textarea.set_attribute("autocomplete", "off")?;
        textarea.set_attribute("autocorrect", "off")?;
        textarea.set_attribute("autocapitalize", "off")?;
        textarea.set_attribute("spellcheck", "false")?;
        textarea.set_attribute("tabindex", "0")?;

        let style = textarea.style();
        style.set_property("position", "fixed")?;
        style.set_property("left", "0px")?;
        style.set_property("top", "0px")?;
        style.set_property("width", "10px")?;
        style.set_property("height", "14px")?;
        style.set_property("opacity", "0")?;
        style.set_property("z-index", "1000")?;
        style.set_property("background", "transparent")?;
        style.set_property("color", "transparent")?;
        style.set_property("caret-color", "transparent")?;
        style.set_property("border", "none")?;
        style.set_property("outline", "none")?;
        style.set_property("resize", "none")?;
        style.set_property("overflow", "hidden")?;
        style.set_property("font-family", "monospace")?;
        style.set_property("font-size", "10px")?;
        style.set_property("line-height", "12px")?;
        style.set_property("padding", "0")?;
        style.set_property("margin", "0")?;
        style.set_property("pointer-events", "none")?;
        style.set_property("user-select", "none")?;
        style.set_property("-webkit-user-select", "none")?;

        body.append_child(&textarea)?;

        let is_composing = Rc::new(Cell::new(false));
        let mut closures = Vec::new();

        // 1. compositionstart
        {
            let is_comp = is_composing.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
                is_comp.set(true);
            });
            textarea.add_event_listener_with_callback(
                "compositionstart",
                cb.as_ref().unchecked_ref(),
            )?;
            closures.push(cb);
        }

        // 2. compositionend
        {
            let is_comp = is_composing.clone();
            let state = prompt_state.clone();
            let elem_clone = textarea.clone();
            let px = proxy.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
                is_comp.set(false);
                let text = ev
                    .dyn_into::<web_sys::CompositionEvent>()
                    .ok()
                    .and_then(|e| e.data())
                    .unwrap_or_default();
                elem_clone.set_value("");

                if !text.is_empty() {
                    if let Some(session) = state.active_session() {
                        let _ = session.send_bytes(text.as_bytes());
                        let _ = px.send_event(());
                    }
                }
            });
            textarea
                .add_event_listener_with_callback("compositionend", cb.as_ref().unchecked_ref())?;
            closures.push(cb);
        }

        // 3. input
        {
            let is_comp = is_composing.clone();
            let state = prompt_state.clone();
            let elem_clone = textarea.clone();
            let px = proxy.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
                if !is_comp.get() {
                    let val = elem_clone.value();
                    elem_clone.set_value("");
                    if !val.is_empty() {
                        if let Some(session) = state.active_session() {
                            let _ = session.send_bytes(val.as_bytes());
                            let _ = px.send_event(());
                        }
                    }
                }
            });
            textarea.add_event_listener_with_callback("input", cb.as_ref().unchecked_ref())?;
            closures.push(cb);
        }

        // 4. keydown
        {
            let is_comp = is_composing.clone();
            let state = prompt_state.clone();
            let elem_clone = textarea.clone();
            let px = proxy.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
                if is_comp.get() {
                    return;
                }
                if let Ok(kb_ev) = ev.clone().dyn_into::<web_sys::KeyboardEvent>() {
                    let key = kb_ev.key();
                    let ctrl = kb_ev.ctrl_key();
                    let cmd = kb_ev.meta_key();

                    let mut handled = false;
                    let mut bytes_to_send: Option<&[u8]> = None;

                    match key.as_str() {
                        "Enter" => {
                            bytes_to_send = Some(b"\r");
                            handled = true;
                        }
                        "Backspace" => {
                            bytes_to_send = Some(b"\x7f");
                            handled = true;
                        }
                        "Tab" => {
                            bytes_to_send = Some(b"\t");
                            handled = true;
                        }
                        "Escape" => {
                            bytes_to_send = Some(b"\x1b");
                            handled = true;
                        }
                        "ArrowUp" => {
                            bytes_to_send = Some(b"\x1b[A");
                            handled = true;
                        }
                        "ArrowDown" => {
                            bytes_to_send = Some(b"\x1b[B");
                            handled = true;
                        }
                        "ArrowRight" => {
                            bytes_to_send = Some(b"\x1b[C");
                            handled = true;
                        }
                        "ArrowLeft" => {
                            bytes_to_send = Some(b"\x1b[D");
                            handled = true;
                        }
                        "c" | "C" if cmd || (ctrl && state.selection_range().is_some()) => {
                            if let Some(session) = state.active_session() {
                                let rows = session.rows();
                                let text = if let Some((start, end)) = state.selection_range() {
                                    extract_selected_text(&rows, start, end)
                                } else if state.is_captured() {
                                    let max_r = rows.len().saturating_sub(1);
                                    let max_c = rows.last().map(|r| r.len()).unwrap_or(0);
                                    extract_selected_text(&rows, (0, 0), (max_r, max_c))
                                } else {
                                    String::new()
                                };
                                if !text.is_empty() {
                                    if let Some(win) = web_sys::window() {
                                        let _ = win.navigator().clipboard().write_text(&text);
                                    }
                                }
                            }
                            handled = true;
                        }
                        "c" | "C" if ctrl => {
                            bytes_to_send = Some(b"\x03");
                            handled = true;
                        }
                        "l" | "L" if ctrl => {
                            bytes_to_send = Some(b"\x0c");
                            handled = true;
                        }
                        "d" | "D" if ctrl => {
                            bytes_to_send = Some(b"\x04");
                            handled = true;
                        }
                        "u" | "U" if ctrl => {
                            bytes_to_send = Some(b"\x15");
                            handled = true;
                        }
                        "w" | "W" if ctrl => {
                            bytes_to_send = Some(b"\x17");
                            handled = true;
                        }
                        "a" | "A" if ctrl || cmd => {
                            if let Some(session) = state.active_session() {
                                let rows = session.rows();
                                let max_r = rows.len().saturating_sub(1);
                                let max_c = rows.last().map(|r| r.len()).unwrap_or(0);
                                state.set_selection_range(Some(((0, 0), (max_r, max_c))));
                                handled = true;
                            }
                        }
                        _ => {}
                    }

                    if handled {
                        kb_ev.prevent_default();
                        ev.stop_propagation();
                        elem_clone.set_value("");

                        if let Some(bytes) = bytes_to_send {
                            if let Some(session) = state.active_session() {
                                let _ = session.send_bytes(bytes);
                                let _ = px.send_event(());
                            }
                        }
                    }
                }
            });
            textarea.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref())?;
            closures.push(cb);
        }

        // 5. copy (handles native ⌘C, browser Edit -> Copy, and context menu Copy)
        {
            let state = prompt_state.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
                if let Some(session) = state.active_session() {
                    let rows = session.rows();
                    let text = if let Some((start, end)) = state.selection_range() {
                        extract_selected_text(&rows, start, end)
                    } else if state.is_captured() {
                        let max_r = rows.len().saturating_sub(1);
                        let max_c = rows.last().map(|r| r.len()).unwrap_or(0);
                        extract_selected_text(&rows, (0, 0), (max_r, max_c))
                    } else {
                        String::new()
                    };

                    if !text.is_empty() {
                        if let Ok(clip_ev) = ev.clone().dyn_into::<web_sys::ClipboardEvent>() {
                            if let Some(dt) = clip_ev.clipboard_data() {
                                let _ = dt.set_data("text/plain", &text);
                                ev.prevent_default();
                            }
                        }
                        if let Some(win) = web_sys::window() {
                            let _ = win.navigator().clipboard().write_text(&text);
                        }
                    }
                }
            });
            textarea.add_event_listener_with_callback("copy", cb.as_ref().unchecked_ref())?;
            window.add_event_listener_with_callback("copy", cb.as_ref().unchecked_ref())?;
            closures.push(cb);
        }

        Ok(Self {
            element: textarea,
            prompt_state,
            _closures: closures,
        })
    }

    /// Synchronizes the position of the DOM textarea overlay to the active terminal cursor,
    /// and manages focus state based on whether the Prompt bay is capturing keyboard input.
    pub fn sync(&self, bay_rect: Option<Rect>, _scale: f64) {
        let is_captured = self.prompt_state.is_captured();
        if !is_captured {
            let _ = self.element.blur();
            return;
        }

        let Some(rect) = bay_rect else {
            return;
        };

        // Determine cursor position in pixels
        let (cursor_row, cursor_col) = self
            .prompt_state
            .active_session()
            .map(|s| s.cursor())
            .unwrap_or((0, 0));

        let char_w = 6.0;
        let row_h = 12.0;
        let head_h = 24.0;

        let cursor_x = rect.min.x as f64 + 4.0 + (cursor_col as f64 * char_w);
        let cursor_y = rect.min.y as f64 + head_h + (cursor_row as f64 * row_h);

        let style = self.element.style();
        let _ = style.set_property("left", &format!("{:.1}px", cursor_x));
        let _ = style.set_property("top", &format!("{:.1}px", cursor_y));

        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                let is_already_active = doc
                    .active_element()
                    .map(|el| el == *self.element.as_ref())
                    .unwrap_or(false);
                if !is_already_active {
                    let opts = web_sys::FocusOptions::new();
                    opts.set_prevent_scroll(true);
                    let _ = self.element.focus_with_options(&opts);
                }
            }
            win.scroll_to_with_x_and_y(0.0, 0.0);
        }
    }
}
