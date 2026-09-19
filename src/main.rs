use std::io::{self, Write};
use crossterm::{cursor::{Hide, MoveTo, Show}, style::Print, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen}};
use std::{thread, time::Duration};

fn main() -> io::Result<()> {
    execute!(io::stdout(), EnterAlternateScreen, Hide)?; // enter "alternate screen mode" of terminal
    // get terminal size if cols less than 40 or rows less than 20 don't start game
    let size = crossterm::terminal::size().unwrap();
    let (cols, rows) = size;
    if cols < 40 || rows < 20 {
        eprintln!("Terminal too small must have at least:\n40 columns\n20 rows"); // error message
        std::process::exit(1); // exit program
    }

    let x = 20;
    let y = 20;

    let mut out = io::stdout(); // out is literally what is displayed on screen
    execute!(out, MoveTo(x, y), Print("█"))?;
    // write!(out, "rows: {} cols: {}", rows, cols); // sets Hello World! on screen
    out.flush()?; // text goes into a waiting area until displayed to terminal flush instantly sends it

    thread::sleep(Duration::from_secs(10));

    execute!(io::stdout(), LeaveAlternateScreen, Show) // exit "alternate screen mode" of terminal
}
