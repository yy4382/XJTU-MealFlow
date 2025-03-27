use color_eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    actions::{Action, CompAction},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
    utils::key_events::KeyEvent,
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
            enter_keys: vec![KeyCode::Enter.into()],
            submit_keys: vec![KeyCode::Enter.into()],
            exit_keys: vec![KeyCode::Esc.into()],
        }
    }
}

impl InputCompCtrlKeys {
    #[allow(dead_code)]
    pub fn with_enter_keys(mut self, enter_keys: Vec<KeyEvent>) -> Self {
        self.enter_keys = enter_keys;
        self
    }
    #[allow(dead_code)]
    pub fn with_submit_keys(mut self, submit_keys: Vec<KeyEvent>) -> Self {
        self.submit_keys = submit_keys;
        self
    }
    #[allow(dead_code)]
    pub fn with_exit_keys(mut self, exit_keys: Vec<KeyEvent>) -> Self {
        self.exit_keys = exit_keys;
        self
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InputAction {
    SwitchMode(InputMode),
    HandleKey(KeyEvent),
    HandlePaste(String),
    Exit(),
    DirectExit(),
    SubmitExit(String),

    /// the event owner should pay attention to
    Submit(String),
}

impl InputComp {
    pub fn new<T: Into<String>, K: Into<String>>(
        id: u64,
        from: Option<T>,
        title: K,
        ctrl_keys: InputCompCtrlKeys,
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
            control_keys: ctrl_keys,
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

    pub fn set_auto_submit(self, b: bool) -> Self {
        Self {
            auto_submit: b,
            ..self
        }
    }

    pub fn get_help_msg(&self, inputing: bool) -> HelpMsg {
        let mut msg = HelpMsg::default();
        if matches!(self.mode, InputMode::Focused) {
            if inputing {
                if self.auto_submit {
                    msg.push(HelpEntry::new(
                        self.control_keys.submit_keys[0].clone(),
                        "quit input",
                    ));
                } else {
                    msg.push(HelpEntry::new(
                        self.control_keys.exit_keys[0].clone(),
                        "quit input",
                    ));
                    msg.push(HelpEntry::new(
                        self.control_keys.submit_keys[0].clone(),
                        "submit input",
                    ));
                }
            } else {
                msg.push(HelpEntry::new(
                    self.control_keys.enter_keys[0].clone(),
                    "Start input",
                ));
            }
        }
        msg
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
                            if self.control_keys.submit_keys.contains(&(*key).into()) {
                                app.send_action(self.get_action(InputAction::SubmitExit(
                                    self.input.value().to_string(),
                                )))
                            } else if self.control_keys.exit_keys.contains(&(*key).into()) {
                                app.send_action(self.get_action(InputAction::DirectExit()))
                            } else {
                                app.send_action(
                                    self.get_action(InputAction::HandleKey((*key).into())),
                                )
                            }
                        }
                        Event::Paste(s) => {
                            app.send_action(self.get_action(InputAction::HandlePaste(s.clone())))
                        }
                        _ => (),
                    }
                } else if let Event::Key(key) = event {
                    if self.control_keys.enter_keys.contains(&(*key).into()) {
                        app.send_action(Action::SwitchInputMode(true))
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
                    .handle_event(&crossterm::event::Event::Key(key_event.into()));
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
            InputAction::DirectExit() => {
                self.input.reset();
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
    pub fn get_value(&self) -> &str {
        self.input.value()
    }
}

#[cfg(test)]
pub mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        app::RootState,
        component::Component,
        page::Page,
        utils::key_events::test_utils::{get_char_evt, get_key_evt},
    };

    use super::*;
    struct TestInputPage {
        content: String,
        input: InputComp,
    }

    impl TestInputPage {
        fn new(auto_submit: bool) -> Self {
            Self {
                content: Default::default(),
                input: InputComp::new(1, None::<&str>, "Input Test", Default::default())
                    .set_auto_submit(auto_submit),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub(crate) enum TestInputPageAction {
        SetFocus(bool),
    }

    impl Page for TestInputPage {
        fn render(&self, frame: &mut Frame, app: &RootState) {
            self.input.draw(frame, &frame.area(), app);
        }

        fn handle_events(&self, app: &RootState, event: Event) -> Result<()> {
            if !app.input_mode() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Enter => {
                            app.send_action(Action::TestPage(TestInputPageAction::SetFocus(true)));
                        }
                        KeyCode::Esc => {
                            app.send_action(Action::TestPage(TestInputPageAction::SetFocus(false)));
                        }
                        _ => {}
                    }
                }
            };
            self.input.handle_events(&event, app)?;
            Ok(())
        }

        fn update(&mut self, app: &RootState, action: Action) {
            if let Action::TestPage(TestInputPageAction::SetFocus(focus)) = &action {
                app.send_action(self.input.get_switch_mode_action(if *focus {
                    InputMode::Focused
                } else {
                    InputMode::Idle
                }));
            };
            if let Some(text) = self.input.parse_submit_action(&action) {
                self.content = text;
            };
            self.input.update(&action, app).unwrap();
        }

        fn get_name(&self) -> String {
            "Test Input Page".into()
        }
    }

    fn get_test_page(auto_submit: bool) -> (TestInputPage, RootState) {
        let app = RootState::new(None);
        let mut page = TestInputPage::new(auto_submit);
        page.init(&app);
        (page, app)
    }

    #[test]
    fn test_input() {
        let (mut page, mut app) = get_test_page(false);

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Enter),
            get_char_evt('a'),
            get_char_evt('b'),
            get_key_evt(KeyCode::Enter),
        ];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "ab");

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Left),
            get_char_evt('c'),
            get_key_evt(KeyCode::Enter),
        ];
        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "acb")
    }

    #[test]
    fn test_input_auto_submit() {
        let (mut page, mut app) = get_test_page(true);

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Enter),
            get_char_evt('a'),
            get_char_evt('b'),
        ];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "ab");

        let seq = [get_key_evt(KeyCode::Left), get_char_evt('c')];
        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "acb")
    }

    #[test]
    fn test_input_paste() {
        let (mut page, mut app) = get_test_page(false);

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Enter),
            get_char_evt('a'),
            get_char_evt('b'),
            get_key_evt(KeyCode::Enter),
        ];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "ab");

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Left),
            Event::Paste("ccc".into()),
            get_key_evt(KeyCode::Enter),
        ];
        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "acccb")
    }

    #[test]
    fn test_input_paste_auto_submit() {
        let (mut page, mut app) = get_test_page(true);

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Enter),
            get_char_evt('a'),
            get_char_evt('b'),
        ];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "ab");

        let seq = [get_key_evt(KeyCode::Left), Event::Paste("ccc".into())];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "acccb")
    }

    #[test]
    fn test_input_quit() {
        let (mut page, mut app) = get_test_page(false);

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_key_evt(KeyCode::Enter),
            get_char_evt('a'),
            get_char_evt('b'),
            get_key_evt(KeyCode::Esc),
        ];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        assert_eq!(page.content, "");
        assert_eq!(page.input.get_value(), "")
    }

    fn get_buffer_color(t: &Terminal<TestBackend>) -> Color {
        let cell = t
            .backend()
            .buffer()
            .content()
            .iter()
            .find(|&c| c.symbol() == "I")
            .unwrap();

        cell.fg
    }

    #[test]
    fn test_render() {
        let (mut page, mut app) = get_test_page(false);
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal.draw(|frame| page.render(frame, &app)).unwrap();
        assert_snapshot!(terminal.backend());
        assert_eq!(get_buffer_color(&terminal), Color::Reset);

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        terminal.draw(|f| page.render(f, &app)).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Cyan);

        let seq = [
            get_key_evt(KeyCode::Enter),
            get_char_evt('a'),
            get_char_evt('b'),
        ];

        seq.iter()
            .for_each(|e| app.handle_event_and_update(&mut page, e.clone()));

        terminal.draw(|f| page.render(f, &app)).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Yellow);

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        terminal.draw(|f| page.render(f, &app)).unwrap();
        assert_snapshot!(terminal.backend());
        assert_eq!(get_buffer_color(&terminal), Color::Cyan);

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Esc));
        terminal.draw(|f| page.render(f, &app)).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Reset);
    }

    #[test]
    fn test_help_msg() {
        let (mut page, mut app) = get_test_page(false);
        fn get_help_msg(page: &TestInputPage, input: bool) -> String {
            <HelpMsg as Into<String>>::into(page.input.get_help_msg(input))
        }
        assert_eq!(get_help_msg(&page, false), "");
        assert_eq!(get_help_msg(&page, true), "");
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(get_help_msg(&page, false), "Start input: enter");
        assert_eq!(
            get_help_msg(&page, true),
            "quit input: esc | submit input: enter"
        );
    }

    #[test]
    fn test_help_msg_auto_commit() {
        let (mut page, mut app) = get_test_page(true);
        fn get_help_msg(page: &TestInputPage, input: bool) -> String {
            <HelpMsg as Into<String>>::into(page.input.get_help_msg(input))
        }
        assert_eq!(get_help_msg(&page, false), "");
        assert_eq!(get_help_msg(&page, true), "");
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(get_help_msg(&page, false), "Start input: enter");
        assert_eq!(get_help_msg(&page, true), "quit input: enter");
    }
}
