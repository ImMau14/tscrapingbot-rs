pub mod image;
pub use image::{analyze_image, message_has_photo};

pub mod analize;
pub use analize::{run_main_model, run_reasoning_step};
