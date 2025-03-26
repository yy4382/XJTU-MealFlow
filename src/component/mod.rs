pub(crate) mod input;

use color_eyre::eyre::Result;
use ratatui::{Frame, layout::Rect};

use crate::{actions::Action, app::RootState};

pub(crate) trait Component {
    #[allow(dead_code)]
    fn get_id(&self) -> u64;

    fn handle_events(&self, event: &crate::tui::Event, app: &RootState) -> Result<()>;

    fn update(&mut self, action: &Action, app: &RootState) -> Result<()>;

    fn draw(&self, frame: &mut Frame, area: &Rect, app: &RootState);
}
