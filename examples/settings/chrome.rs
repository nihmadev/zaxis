use zaxis::{Context, Frame, TitleBar};

pub enum Action {
    Close,
    Minimize,
    Maximize,
}

pub fn show(context: &mut Context, maximized: bool) -> Option<Action> {
    let response = TitleBar::new("Settings").maximized(maximized).show(context);
    if response.close {
        Some(Action::Close)
    } else if response.minimize {
        Some(Action::Minimize)
    } else if response.maximize {
        Some(Action::Maximize)
    } else {
        None
    }
}

pub fn apply(action: Action, frame: &mut Frame<'_>) {
    match action {
        // Ask like the OS does, so the title-bar button and Alt+F4 share one confirmation.
        Action::Close => frame.request_close(),
        Action::Minimize => frame.window().set_minimized(true),
        Action::Maximize => frame.window().set_maximized(!frame.window().is_maximized()),
    }
}
