//! Embedding surface alongside the original Synth page and its protocol assets.
use crate::shell::WindowInput;
use serde_json::Value;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub struct HostPage {
    pub markup: &'static str,
    pub stylesheet: &'static str,
    pub script: &'static str,
}

#[derive(Clone, Debug)]
pub struct HostView {
    pub document: Value,
    pub synth_visible: bool,
    pub text_entry: bool,
}

#[derive(Clone)]
pub struct HostPresentation {
    pub page: HostPage,
    pub projection: Rc<dyn Fn() -> HostView>,
    pub input: Rc<dyn Fn(WindowInput) -> bool>,
    pub event: Rc<dyn Fn(Value)>,
}
impl std::fmt::Debug for HostPresentation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostPresentation")
            .field("page", &"embedded")
            .finish_non_exhaustive()
    }
}
