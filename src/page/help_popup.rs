use crossterm::event::KeyCode;
use ratatui::{layout::Rect, widgets::Clear};
use tracing::info;

use crate::{
    actions::{ActionSender, LayerManageAction},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Page, WidgetExt};

pub(crate) struct HelpPopup {
    help_msg: HelpMsg,
    longest_entry_size: u16,

    tx: ActionSender,
}

impl HelpPopup {
    pub fn new(tx: ActionSender, msg: HelpMsg) -> Self {
        // FIXME use real size
        let longest = 40;
        Self {
            help_msg: msg,
            longest_entry_size: longest,
            tx,
        }
    }
}

impl EventLoopParticipant for HelpPopup {
    fn handle_events(&self, event: crate::tui::Event) -> color_eyre::eyre::Result<()> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.tx.send(LayerManageAction::PopPage);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn update(&mut self, action: crate::actions::Action) {
        // TODO handle list actions
        ()
    }
}

impl Page for HelpPopup {}

impl WidgetExt for HelpPopup {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        info!("Rendering help popup, {:?}", area);
        let show_area = Rect {
            // FIXME deal with subtract overflow
            x: (area.width - self.longest_entry_size - 2) / 2,
            y: 0,
            width: self.longest_entry_size + 2,
            height: area.height * 4 / 5,
        };
        let bottom_help_area = Rect {
            x: 0,
            y: area.height - 3,
            width: area.width,
            height: 3,
        };

        frame.render_widget(Clear, bottom_help_area);
        HelpPopup::get_self_help_msg().render(frame, bottom_help_area);

        frame.render_widget(Clear, show_area);
        // TODO use list
        self.help_msg.render(frame, show_area);
    }
}

impl HelpPopup {
    pub fn get_self_help_msg() -> HelpMsg {
        let help_msg = vec![
            HelpEntry::new('j', "Move Down"),
            HelpEntry::new('k', "Move Up"),
            HelpEntry::new(KeyCode::Esc, "Close help"),
        ];
        help_msg.into()
    }
}
