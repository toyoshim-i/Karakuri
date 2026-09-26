//! Prompt bay module: embedded agent terminal and CLI selection.

mod cli;
mod head;
mod paint;
mod state;

pub use cli::{is_executable_on_path, CliPreset, CliSelection};
pub use head::{
    prompt_ask, prompt_menu_rect, prompt_pill, PromptAsk, MENU_CARD_W, MENU_ITEM_COUNT,
    MENU_ITEM_H, MENU_PAD_X, MENU_PAD_Y, PROMPT_TITLE,
};
pub use paint::{prompt_head_into, prompt_into, prompt_menu_into};
pub use state::PromptState;
