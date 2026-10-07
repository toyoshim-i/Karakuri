use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{XrReferenceSpace, XrSession};
use winit::event::{ElementState, MouseButton};

/// Normalized pointer event generated from WebXR controller raycast on the 30" console quad.
#[derive(Debug, Clone)]
pub enum WebXrPointerAction {
    CursorMoved {
        x: f64,
        y: f64,
    },
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    MouseWheel {
        delta_y: f32,
    },
}

/// Stereo camera transformation for one eye.
#[derive(Clone, Copy, Debug)]
pub struct StereoEye {
    pub view: [f32; 16],
    pub proj: [f32; 16],
    pub eye: [f32; 3],
}

/// Per-frame stereo poses for left and right eyes.
#[derive(Clone, Copy, Debug)]
pub struct StereoPose {
    pub left: StereoEye,
    pub right: StereoEye,
    /// One eye's share of the XR layer's framebuffer, in pixels.
    pub eye_size: (u32, u32),
}

/// Shared state for WebXR spatial HUD session.
#[derive(Default)]
pub struct WebXrState {
    pub is_supported: bool,
    pub is_active: bool,
    pub session: Option<XrSession>,
    pub _ref_space: Option<XrReferenceSpace>,
    pub(crate) pending_pointer_events: Rc<RefCell<Vec<WebXrPointerAction>>>,
    pub(crate) current_stereo_pose: Rc<RefCell<Option<StereoPose>>>,
    /// The head pose the XR canvas's current picture was drawn for; the XR
    /// layer reprojects it to the pose of the frame it is shown in.
    pub(crate) rendered_stereo_pose: Rc<RefCell<Option<StereoPose>>>,
}

impl WebXrState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ends active WebXR session if currently running.
    pub fn end_session(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session.end();
        }
        self.is_active = false;
    }

    /// Drains pending controller pointer events to inject into winit/egui.
    pub fn drain_pointer_events(&self) -> Vec<WebXrPointerAction> {
        self.pending_pointer_events.borrow_mut().drain(..).collect()
    }

    /// Returns the latest stereo pose if available.
    pub fn stereo_pose(&self) -> Option<StereoPose> {
        *self.current_stereo_pose.borrow()
    }

    /// Records the pose the XR canvas's picture was just drawn for.
    pub fn set_rendered_stereo_pose(&self, pose: Option<StereoPose>) {
        *self.rendered_stereo_pose.borrow_mut() = pose;
    }
}
