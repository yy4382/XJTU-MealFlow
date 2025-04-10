use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::layer_manager::EventHandlingStatus,
    page::WidgetExt,
    tui::Event,
    utils::{
        help_msg::{HelpEntry, HelpMsg},
        key_events::KeyEvent,
    },
};

#[derive(Clone, Debug)]
/// A input Component
///
/// Set the focus state: send a [`InputComp::get_switch_mode_action()`] Action
///
/// Get value: parse an action with [`InputComp::parse_submit_action`]
pub(crate) struct InputComp {
    input: Input,
    pub mode: InputMode,

    title: Option<String>,

    auto_submit: bool,
    control_keys: InputCompCtrlKeys,
}

#[derive(Default, Clone, Debug)]
pub(crate) enum InputMode {
    #[default]
    Idle,
    Focused,
    Inputting,
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

impl InputComp {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            mode: InputMode::default(),
            title: Default::default(),
            auto_submit: false,
            control_keys: Default::default(),
        }
    }

    pub fn init_text<T: Into<String>>(self, text: T) -> Self {
        Self {
            input: Input::new(text.into()),
            ..self
        }
    }

    pub fn title<T: Into<String>>(self, title: T) -> Self {
        Self {
            title: Some(title.into()),
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn enter_keys(mut self, enter_keys: Vec<KeyEvent>) -> Self {
        self.control_keys.enter_keys = enter_keys;
        self
    }
    #[allow(dead_code)]
    pub fn submit_keys(mut self, submit_keys: Vec<KeyEvent>) -> Self {
        self.control_keys.submit_keys = submit_keys;
        self
    }
    #[allow(dead_code)]
    pub fn exit_keys(mut self, exit_keys: Vec<KeyEvent>) -> Self {
        self.control_keys.exit_keys = exit_keys;
        self
    }

    pub fn auto_submit(self, b: bool) -> Self {
        Self {
            auto_submit: b,
            ..self
        }
    }

    pub fn get_help_msg(&self) -> HelpMsg {
        let mut msg = HelpMsg::default();
        match self.mode {
            InputMode::Idle => {}
            InputMode::Focused => {
                msg.push(HelpEntry::new(
                    self.control_keys.enter_keys[0].clone(),
                    "Start input",
                ));
            }
            InputMode::Inputting => {
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
            }
        };
        msg
    }
}

impl InputComp {
    pub fn handle_events(
        &mut self,
        event: &crate::tui::Event,
    ) -> (EventHandlingStatus, Option<String>) {
        let mut status = EventHandlingStatus::default();
        let mut output_string: Option<String> = None;
        match self.mode {
            InputMode::Idle => (),
            InputMode::Focused => {
                if let Event::Key(key) = event {
                    if self.control_keys.enter_keys.contains(&(*key).into()) {
                        self.mode = InputMode::Inputting;
                        status.consumed();
                    }
                }
            }
            InputMode::Inputting => match event {
                Event::Key(key) => {
                    if self.control_keys.submit_keys.contains(&(*key).into()) {
                        output_string = Some(self.input.value().to_string());
                        self.mode = InputMode::Focused;
                        status.consumed();
                    } else if self.control_keys.exit_keys.contains(&(*key).into()) {
                        self.input.reset();
                        self.mode = InputMode::Focused;
                        status.consumed();
                    } else {
                        self.input.handle_event(&crossterm::event::Event::Key(*key));
                        if self.auto_submit {
                            output_string = Some(self.input.value().to_string());
                        }
                        status.consumed();
                    }
                }
                Event::Paste(s) => {
                    s.chars().for_each(|c| {
                        self.input.handle(tui_input::InputRequest::InsertChar(c));
                    });
                    if self.auto_submit {
                        output_string = Some(self.input.value().to_string());
                    }
                    status.consumed();
                }
                _ => (),
            },
        };
        (status, output_string)
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }
    pub fn is_inputting(&self) -> bool {
        matches!(self.mode, InputMode::Inputting)
    }
}

impl WidgetExt for InputComp {
    fn render(&mut self, frame: &mut Frame, area: ratatui::prelude::Rect) {
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style = match self.mode {
            InputMode::Focused => Color::Cyan.into(),
            InputMode::Inputting => Color::Yellow.into(),
            InputMode::Idle => Style::default(),
        };

        let input_widget = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(
                match &self.title {
                    Some(title) => Block::default().title(title.as_str()),
                    None => Block::default(),
                }
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            );
        frame.render_widget(input_widget, area);

        if matches!(self.mode, InputMode::Focused) && self.is_inputting() {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }
}

#[cfg(test)]
pub mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn get_input(auto_submit: bool) -> InputComp {
        InputComp::new()
            .auto_submit(auto_submit)
            .title("Input Test")
    }

    impl InputComp {
        fn handle_seq(&mut self, seq: Vec<Event>) -> Option<String> {
            let mut output = None;
            seq.into_iter().for_each(|e| {
                let status = self.handle_events(&e);
                if let Some(s) = status.1 {
                    output = Some(s);
                }
            });
            output
        }
    }

    #[test]
    fn input_mode_change() {
        let mut input = get_input(false);
        assert!(matches!(input.mode, InputMode::Idle));
        input.set_mode(InputMode::Focused);
        input.handle_events(&KeyCode::Enter.into());
        assert!(matches!(input.mode, InputMode::Inputting));
        input.handle_events(&KeyCode::Enter.into());
        assert!(matches!(input.mode, InputMode::Focused));
    }

    #[test]
    fn test_input() {
        let mut input = get_input(false);
        input.set_mode(InputMode::Focused);

        let seq: Vec<Event> = vec![
            KeyCode::Enter.into(),
            'a'.into(),
            'b'.into(),
            KeyCode::Enter.into(),
        ];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("ab".to_string()));

        let seq = [
            KeyCode::Enter.into(),
            KeyCode::Left.into(),
            'c'.into(),
            KeyCode::Enter.into(),
        ];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("acb".to_string()));
    }

    #[test]
    fn test_input_auto_submit() {
        let mut input = get_input(true);
        input.set_mode(InputMode::Focused);

        let seq: Vec<Event> = vec![KeyCode::Enter.into(), 'a'.into(), 'b'.into()];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("ab".to_string()));

        let seq = [KeyCode::Left.into(), 'c'.into()];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("acb".to_string()));
    }

    #[test]
    fn test_input_paste() {
        let mut input = get_input(false);
        input.set_mode(InputMode::Focused);

        let seq = [
            KeyCode::Enter.into(),
            'a'.into(),
            'b'.into(),
            KeyCode::Enter.into(),
        ];

        assert_eq!(input.handle_seq(seq.to_vec()), Some("ab".to_string()));

        let seq = [
            KeyCode::Enter.into(),
            KeyCode::Left.into(),
            Event::Paste("ccc".into()),
            KeyCode::Enter.into(),
        ];
        // cSpell:ignore acccb
        assert_eq!(input.handle_seq(seq.to_vec()), Some("acccb".to_string()));
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
        let mut input = get_input(false);

        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|frame| input.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
        assert_eq!(get_buffer_color(&terminal), Color::Reset);

        input.set_mode(InputMode::Focused);
        terminal.draw(|f| input.render(f, f.area())).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Cyan);

        let seq = [KeyCode::Enter.into(), 'a'.into(), 'b'.into()];
        input.handle_seq(seq.to_vec());

        terminal.draw(|f| input.render(f, f.area())).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Yellow);

        input.handle_events(&KeyCode::Enter.into());
        terminal.draw(|f| input.render(f, f.area())).unwrap();
        assert_snapshot!(terminal.backend());
        assert_eq!(get_buffer_color(&terminal), Color::Cyan);
    }

    #[test]
    fn test_help_msg() {
        let mut input = get_input(false);
        assert_eq!(input.get_help_msg().to_string(), "");

        input.set_mode(InputMode::Focused);
        assert_eq!(input.get_help_msg().to_string(), "Start input: enter");
        input.set_mode(InputMode::Inputting);
        assert_eq!(
            input.get_help_msg().to_string(),
            "quit input: esc | submit input: enter"
        );
    }

    #[test]
    fn test_help_msg_auto_commit() {
        let mut input = get_input(true);
        assert_eq!(input.get_help_msg().to_string(), "");

        input.set_mode(InputMode::Focused);
        assert_eq!(input.get_help_msg().to_string(), "Start input: enter");
        input.set_mode(InputMode::Inputting);
        assert_eq!(input.get_help_msg().to_string(), "quit input: enter");
    }
}
