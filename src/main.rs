use async_std::channel;
use async_std::sync::Mutex;
use crossterm::event::{self, Event, KeyCode};
use futures::channel::mpsc;
use futures::stream::StreamExt as _;
use futures::Stream;
use iced::advanced::subscription::EventStream;
use iced::event::Status;
use iced::{
    executor, widget::Container, widget::Text, Application, Color, Command, Element, Font, Length,
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

struct TerminalApp {
    receiver: Arc<Mutex<mpsc::Receiver<String>>>,
    output: String,
}

struct Theme {
    background_color: Color,
    text_color: Color,
}

fn theme() -> Theme {
    Theme {
        background_color: Color::BLACK,
        text_color: Color::WHITE,
    }
}

impl Application for TerminalApp {
    type Executor = executor::Default;
    type Message = String;
    type Theme = iced::Theme;
    type Flags = Arc<Mutex<mpsc::Receiver<String>>>;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                receiver: flags,
                output: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rocket Term")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        self.output.push_str(&message);
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        let text = Text::new(&self.output);
        let container = Container::new(text)
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(400.0));
        container.into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::Subscription::from_recipe(TerminalSubscription {
            receiver: self.receiver.clone(),
        })
    }
}

struct TerminalSubscription {
    receiver: Arc<Mutex<mpsc::Receiver<String>>>,
}

impl iced::advanced::subscription::Recipe for TerminalSubscription {
    type Output = String;

    fn hash(&self, state: &mut iced::advanced::Hasher) {
        use std::hash::Hash;
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _: core::pin::Pin<
            std::boxed::Box<
                (dyn Stream<Item = (iced::Event, Status)> + std::marker::Send + 'static),
            >,
        >,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        let receiver = self.receiver.clone();
        async_stream::stream! {
            loop {
                async_std::task::sleep(Duration::from_millis(100)).await;
                let message = match receiver.lock().await.try_next() {
                    Ok(Some(msg)) => {
                        println!("Received Message: {}", msg);
                        msg
                    },
                    _ => continue,
                };

                yield message;
            }
        }
        .boxed()
    }
}

fn main() -> iced::Result {
    let (mut tx, rx) = mpsc::channel(10);
    let rx = Arc::new(Mutex::new(rx));

    // Terminal logic in a separate thread
    thread::spawn(move || loop {
        if let Ok(Event::Key(key_event)) = event::read() {
            match key_event.code {
                KeyCode::Char(c) => {
                    println!("Sending Message: {}", c);
                    let _ = tx.try_send(c.to_string());
                }
                KeyCode::Enter => {
                    let _ = tx.try_send(String::from("\n"));
                }
                KeyCode::Esc => break,
                _ => (),
            }
        }
    });

    let window_settings = iced::window::Settings {
        size: (800, 600),
        ..Default::default()
    };

    let terminal_settings = iced::Settings {
        id: None,
        window: window_settings,
        flags: rx,
        default_font: Default::default(),
        default_text_size: 1.0,
        antialiasing: false,
        exit_on_close_request: false,
    };

    TerminalApp::run(terminal_settings)
}
