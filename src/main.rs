use std::io::{self, Write};
use crossterm::{execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen}};
use std::{thread, time::Duration};

fn main() -> io::Result<()> {
    execute!(io::stdout(), EnterAlternateScreen)?; // enter "alternate screen mode" of terminal

    // Do anything on the alternate screen
    let mut out = io::stdout();
    write!(out, "Hello World!");
    out.flush();
    thread::sleep(Duration::from_secs(3));

    execute!(io::stdout(), LeaveAlternateScreen) // exit "alternate screen mode" of terminal
}
