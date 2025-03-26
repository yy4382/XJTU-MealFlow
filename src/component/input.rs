use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    actions::{Action, CompAction},
    tui::Event,
};

#[derive(Clone, Debug)]
/// A input Component
///
/// Set the focus state: send a [`InputComp::get_switch_mode_action()`] Action
///
/// Get value: parse an action with [`InputComp::parse_submit_action`]
pub(crate) struct InputComp {
    id: u64,
    input: Input,
    mode: InputMode,

    title: String,

    auto_submit: bool,
    control_keys: InputCompCtrlKeys,
}

#[derive(Default, Clone, Debug)]
pub(crate) enum InputMode {
    #[default]
    Idle,
    Focused,
}

#[derive(Clone, Debug)]
pub(crate) struct InputCompCtrlKeys {
    enter_keys: Vec<KeyEvent>,
    submit_keys: Vec<KeyEvent>,
    exit_keys: Vec<KeyEvent>,
}

impl Default for InputCompCtrlKeys {
    fn default() -> Self {
        Self {
            enter_keys: vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)],
            submit_keys: vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)],
            exit_keys: vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)],
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InputAction {
    SwitchMode(InputMode),
    HandleKey(KeyEvent),
    HandlePaste(String),
    Exit(),
    SubmitExit(String),

    /// the event owner should pay attention to
    Submit(String),
}

impl InputComp {
    pub fn new<T: Into<String>, K: Into<String>>(
        id: u64,
        from: Option<T>,
        title: K,
        ctrl_keys: Option<InputCompCtrlKeys>,
    ) -> Self {
        Self {
            id,
            input: if let Some(from) = from {
                Input::from(from.into())
            } else {
                Input::default()
            },
            mode: InputMode::default(),
            title: title.into(),
            auto_submit: false,
            control_keys: ctrl_keys.unwrap_or_default(),
        }
    }

    pub fn get_switch_mode_action(&self, mode: InputMode) -> Action {
        Action::Comp((CompAction::Input(InputAction::SwitchMode(mode)), self.id))
    }

    pub fn parse_submit_action(&self, action: &Action) -> Option<String> {
        if let Some(input_action) = self.unwrap_action(action) {
            match input_action {
                InputAction::Submit(s) => Some(s),
                _ => None,
            }
        } else {
            None
        }
    }

    fn get_action(&self, action: InputAction) -> Action {
        Action::Comp((CompAction::Input(action), self.id))
    }

    fn unwrap_action(&self, action: &Action) -> Option<InputAction> {
        if let Action::Comp((comp_action, id)) = action {
            if *id != self.id {
                return None;
            };
            if let CompAction::Input(action) = comp_action {
                Some(action.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl super::Component for InputComp {
    fn get_id(&self) -> u64 {
        self.id
    }

    fn handle_events(&self, event: &crate::tui::Event, app: &crate::app::RootState) -> Result<()> {
        match self.mode {
            InputMode::Idle => (),
            InputMode::Focused => {
                if app.input_mode() {
                    match event {
                        Event::Key(key) => {
                            if self.control_keys.submit_keys.contains(key) {
                                app.send_action(self.get_action(InputAction::SubmitExit(
                                    self.input.value().to_string(),
                                )))
                            } else if self.control_keys.exit_keys.contains(key) {
                                app.send_action(self.get_action(InputAction::Exit()))
                            } else {
                                app.send_action(self.get_action(InputAction::HandleKey(*key)))
                            }
                        }
                        Event::Paste(s) => {
                            app.send_action(self.get_action(InputAction::HandlePaste(s.clone())))
                        }
                        _ => (),
                    }
                } else {
                    match event {
                        Event::Key(key) => {
                            if self.control_keys.enter_keys.contains(&key) {
                                app.send_action(Action::SwitchInputMode(true))
                            }
                        }
                        _ => (),
                    }
                }
            }
        };
        Ok(())
    }

    fn update(
        &mut self,
        action: &crate::actions::Action,
        app: &crate::app::RootState,
    ) -> Result<()> {
        let Some(action) = self.unwrap_action(action) else {
            return Ok(());
        };

        match action {
            InputAction::SwitchMode(input_mode) => {
                self.mode = input_mode;
                Ok(())
            }
            InputAction::HandleKey(key_event) => {
                self.input
                    .handle_event(&crossterm::event::Event::Key(key_event));
                if self.auto_submit {
                    app.send_action(
                        self.get_action(InputAction::Submit(self.input.value().to_string())),
                    )
                }
                Ok(())
            }
            InputAction::HandlePaste(string) => {
                string.chars().for_each(|c| {
                    self.input.handle(tui_input::InputRequest::InsertChar(c));
                });
                if self.auto_submit {
                    app.send_action(
                        self.get_action(InputAction::Submit(self.input.value().to_string())),
                    )
                }
                Ok(())
            }
            InputAction::SubmitExit(string) => {
                app.send_action(self.get_action(InputAction::Submit(string)));
                app.send_action(self.get_action(InputAction::Exit()));
                Ok(())
            }
            InputAction::Exit() => {
                app.send_action(Action::SwitchInputMode(false));
                Ok(())
            }
            InputAction::Submit(_) => Ok(()),
        }
    }

    fn draw(&self, frame: &mut Frame, area: &ratatui::prelude::Rect, app: &crate::app::RootState) {
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style = match self.mode {
            InputMode::Focused => {
                if app.input_mode() {
                    Color::Yellow.into()
                } else {
                    Color::Cyan.into()
                }
            }
            InputMode::Idle => Style::default(),
        };

        let input_widget = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(self.title.as_str()),
            );
        frame.render_widget(input_widget, *area);

        if matches!(self.mode, InputMode::Focused) && app.input_mode() {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }
}

#[cfg(test)]
impl InputComp {
    pub fn get_mode(&self) -> InputMode {
        self.mode.clone()
    }
}
