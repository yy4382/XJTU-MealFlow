pub(crate) mod input;

use color_eyre::eyre::Result;

use crate::{actions::Action, page::WidgetExt};

pub(crate) trait Component: WidgetExt {
    #[allow(dead_code)]
    fn get_id(&self) -> u64;

    fn handle_events(&self, event: &crate::tui::Event) -> Result<()>;

    fn update(&mut self, action: &Action) -> Result<()>;
}
