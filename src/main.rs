use std::io::{self, Write};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    style::Print, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode}};
use std::time::{Duration, Instant};

const WIDTH: u16 = 40;
const HEIGHT: u16 = 40;

fn main() -> io::Result<()> {
    // get terminal size if cols less than 40 or rows less than 20 don't start game
    let terminal_size = crossterm::terminal::size().unwrap();
    let (terminal_cols, terminal_rows) = terminal_size;
    if terminal_cols < (WIDTH + 2)*2 || terminal_rows < HEIGHT+2 {
        eprintln!("Terminal too small must have at least:\n{} columns\n{} rows", (WIDTH+2)*2, HEIGHT+2); // error message
        std::process::exit(1); // exit program
    }

    execute!(io::stdout(), EnterAlternateScreen, Hide)?; // enter "alternate screen mode" of terminal
    enable_raw_mode()?; // keys are delivered instantly, no Enter needed, no echo

    let mut out = io::stdout(); // out is literally what is displayed on screen

    let x = (terminal_cols - (WIDTH + 2) * 2) / 2;
    let y = (terminal_rows - (HEIGHT+2))/2;

    // create border
    for i in 0..WIDTH+2 {
        execute!(out, MoveTo(x + i*2, y), Print("██"))?; // top border
        execute!(out, MoveTo(x + i*2, y + (HEIGHT+1)), Print("██"))?; // bottom border
    }
    for i in 0..HEIGHT+2 {
        execute!(out, MoveTo(x, y + i), Print("██"))?; // left border
        execute!(out, MoveTo(x + (WIDTH+1)*2, y + i), Print("██"))?; // right border
    }

    let (col, row) = screen_position(x, y, WIDTH/2, HEIGHT-10);
    execute!(out, MoveTo(col, row), Print("██"))?;
    out.flush()?; // text goes into a waiting area until displayed to terminal flush instantly sends it

    // ====================
    // START GAME
    // ====================
    let result = run_game(&mut out, x, y);

    // =====================
    // GAME OVER CLEAN UP
    // =====================
    let _ = disable_raw_mode(); // must run or the terminal stays broken
    execute!(io::stdout(), LeaveAlternateScreen, Show); // exit "alternate screen mode" of terminal
    result
}

fn run_game (screen: &mut io::Stdout, x: u16, y: u16) -> io::Result<()> {
    // =====================
    // VARIABLES
    // =====================
    let game_over = false;
    let (start_x, start_y) = screen_position(x, y, WIDTH/2, HEIGHT-10);
    let mut head = (start_x as i16, start_y as i16); // (col, row), i16 so it can do math with direction
    let mut direction = (0i16, -1i16); // dx, dy
    let game_tick = Duration::from_millis(167); // standard game tick for original snake game

    // =====================
    // GAME LOOP
    // =====================
    while !game_over {
        // draw state

        // wait 1 gametick for an input
        let tick_start = Instant::now();
        //let mut pressed: Option<KeyCode> = None; // last arrow key pressed during this tick

        loop {
            let remaining = game_tick.saturating_sub(tick_start.elapsed());
            if remaining.is_zero() {
                break; // tick over
            }
            // sleeps until a key event arrives or the remaining time runs out
            if event::poll(remaining)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Up    => direction = (0, -1),
                            KeyCode::Down  => direction = (0, 1),
                            KeyCode::Left  => direction = (-1, 0),
                            KeyCode::Right => direction = (1, 0),
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
                    }
                }
            }
        }
        // =======================================
        // update direction, then snake position
        // =======================================

        // update head (columns are 2 chars wide, so x moves by 2)
        head.0 += direction.0 * 2;
        head.1 += direction.1;

        execute!(screen, MoveTo(head.0 as u16, head.1 as u16), Print("██"))?;
        // =======================================
        // check collisions for if game is over
        // =======================================
    }
    Ok(())
}

// helper function convert to screen position
fn screen_position (origin_x :u16, origin_y :u16, x :u16, y :u16) -> (u16, u16) { // (col, row)
    return (origin_x+((x+1)*2),origin_y+(y+1));
}
