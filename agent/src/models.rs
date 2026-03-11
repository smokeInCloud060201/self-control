use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ControlEvent {
    #[serde(rename = "start_capture")]
    StartCapture,
    #[serde(rename = "stop_capture")]
    StopCapture,
    MouseMove { x: f32, y: f32 },
    MouseDown { button: String },
    MouseUp { button: String },
    KeyDown { key: String },
    KeyUp { key: String },
    #[serde(rename = "switch_display")]
    SwitchDisplay { index: usize },
    #[serde(rename = "paste_text")]
    PasteText { text: String },
    #[serde(rename = "resolution_update")]
    ResolutionUpdate { width: usize, height: usize },
}
