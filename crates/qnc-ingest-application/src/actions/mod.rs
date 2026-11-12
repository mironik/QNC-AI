//! One file per action of the form. `dispatch` only says which action runs.

mod bases;
mod dir_cancel;
mod dir_confirm;
mod dir_open;
mod dir_roots;
mod dir_up;
mod import_selected;
mod preview_focus;
mod reload;
mod set_ai_mining;
mod set_archive;
mod set_clip_filter;
mod source_kind;
mod story;

pub use story::StoryAnswer;
