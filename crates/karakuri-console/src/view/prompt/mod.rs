//! Prompt bay module: embedded agent terminal and CLI selection.

mod cli;
mod head;
mod paint;
mod session;
mod state;

pub use cli::{is_executable_on_path, resolve_executable, CliPreset, CliSelection};
pub use head::{
    prompt_ask, prompt_item_rect, prompt_menu_rect, prompt_pill, PromptAsk, MENU_CARD_W, MENU_COLS,
    MENU_COL_GAP, MENU_COL_W, MENU_ITEM_COUNT, MENU_ITEM_H, MENU_PAD_X, MENU_PAD_Y,
    MENU_ROWS_PER_COL, PROMPT_TITLE,
};
pub use paint::{prompt_head_into, prompt_into, prompt_menu_into, PROMPT_FONT_SIZE};
pub use session::{Scrollback, SessionManager, SessionStatus, TerminalSession};
pub use state::PromptState;
