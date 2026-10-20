//! Terminal I/O: raw mode, the alternate screen, the blocking key loop, and
//! the panic hook (D-TERM). This is the only module that touches the real
//! terminal. It is single-threaded: keys are read with a blocking
//! `stdin().keys()`. Known limits: a resize shows on the next key press, and
//! an external SIGTERM or SIGINT leaves the terminal in raw mode.

use super::app::{App, Effect};
use super::views;
use ratatui::Terminal;
use ratatui::backend::TermionBackend;
use ratatui::termion::cursor;
use ratatui::termion::input::TermRead;
use ratatui::termion::raw::IntoRawMode;
use ratatui::termion::screen::{IntoAlternateScreen, ToMainScreen};
use std::io::Write;

/// The bytes that leave the alternate screen and show the cursor.
pub fn restore_bytes() -> Vec<u8> {
    format!("{ToMainScreen}{}", cursor::Show).into_bytes()
}

/// Write the restore bytes to the terminal, then run the saved hook, so the
/// panic message is printed on the main screen where the user sees it. The
/// `RawTerminal` and `AlternateScreen` drop guards restore the terminal
/// settings while the stack unwinds (the workspace never sets
/// `panic = "abort"`; a test guards that).
pub fn install_panic_hook() {
    let saved = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bytes = restore_bytes();
        let mut out = std::io::stdout();
        let _ = out.write_all(&bytes);
        let _ = out.flush();
        let mut err = std::io::stderr();
        let _ = err.write_all(&bytes);
        let _ = err.flush();
        saved(info);
    }));
}

/// Run the picker in the real terminal until the user quits.
pub fn run_terminal(app: &mut App) -> anyhow::Result<()> {
    install_panic_hook();
    let screen = std::io::stdout().into_raw_mode()?.into_alternate_screen()?;
    let mut terminal = Terminal::new(TermionBackend::new(screen))?;
    // The static loading frame, then the synchronous catalog load (M6-8).
    terminal.draw(|f| views::draw(f, app))?;
    app.load()?;
    terminal.draw(|f| views::draw(f, app))?;
    for key in std::io::stdin().keys() {
        let key = key?;
        match app.handle_key(key) {
            Effect::Quit => break,
            Effect::Run => {
                terminal.draw(|f| views::draw(f, app))?;
                app.run_pending();
            }
            Effect::Continue => {}
        }
        terminal.draw(|f| views::draw(f, app))?;
    }
    terminal.show_cursor()?;
    Ok(())
}
