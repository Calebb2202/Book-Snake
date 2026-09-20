use std::io::{self, Write};
use std::collections::VecDeque;
use rand::RngExt; // rand 0.10: provides random_range
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    style::{Print, SetForegroundColor, ResetColor, Color}, execute, queue,
    terminal::{EnterAlternateScreen, Clear, ClearType, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode}};
use std::time::{Duration, Instant};

const WIDTH: u16 = 30;
const HEIGHT: u16 = 30;

struct Question {
    prompt: String,
    answers: [String; 4],
    answer_index: usize
}

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

    // ====================
    // START GAME
    // ====================
    let result = run_game(&mut out, x, y);

    // =====================
    // GAME OVER CLEAN UP
    // =====================
    let _ = disable_raw_mode(); // must run or the terminal stays broken
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show); // exit "alternate screen mode" of terminal
    result
}

fn run_game (screen: &mut io::Stdout, x: u16, y: u16) -> io::Result<()> {
    // =====================
    // VARIABLES
    // =====================
    let mut game_over = false;

    // snake is stored in GRID coordinates (0..WIDTH, 0..HEIGHT), not screen coordinates
    // front = head, back = tail, snake.len() = length
    let mut snake: VecDeque<(i16, i16)> = VecDeque::new();
    snake.push_back((WIDTH as i16 / 2, HEIGHT as i16 - 10));
    let mut grow = false; // set to true when an apple is eaten, snake keeps its tail for one tick

    let mut apple = spawn_apple(&snake);

    let mut direction = (0i16, -1i16); // dx, dy // this is the direction moved on the LAST tick
    let mut next_direction = direction; // direction requested by input, applied at the end of the tick
    let game_tick = Duration::from_millis(120); // standard game tick for original snake game

    draw_board(screen, x, y, &snake, apple)?;

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
                        let requested = match key.code {
                            KeyCode::Up    => (0, -1),
                            KeyCode::Down  => (0, 1),
                            KeyCode::Left  => (-1, 0),
                            KeyCode::Right => (1, 0),
                            KeyCode::Char('q') => return Ok(()),
                            _ => next_direction,
                        };
                        // compared against `direction` (last MOVED direction), so pressing
                        // two keys within one tick can't reverse the snake into itself
                        if requested != (-direction.0, -direction.1) {
                            next_direction = requested;
                        }
                    }
                }
            }
        }
        // =======================================
        // update direction, then snake position
        // =======================================
        direction = next_direction; // lock in this tick's direction

        let head = snake[0];
        let new_head = (head.0 + direction.0, head.1 + direction.1); // x moves by 1 grid cell (screen_position handles the width of 2)

        // =======================================
        // check collisions for if game is over
        // =======================================
        let hit_wall = new_head.0 < 0 || new_head.0 >= WIDTH as i16
                    || new_head.1 < 0 || new_head.1 >= HEIGHT as i16;

        // the tail cell moves away this tick (unless growing), so don't count it as a hit
        let body_len = if grow { snake.len() } else { snake.len() - 1 };
        let hit_self = snake.iter().take(body_len).any(|&segment| segment == new_head);

        if hit_wall || hit_self {
            game_over = true;
            continue; // skip drawing, the while condition ends the loop
        }

        let ate_apple = new_head == apple;
        if ate_apple {
            grow = true; // the tail block below sees this and keeps the tail
        }

        // erase the tail first (before drawing the head, in case the head moves into the old tail cell)
        if grow {
            grow = false; // keep the tail this tick, snake gets 1 longer
        } else {
            let tail = snake.pop_back().unwrap();
            let (col, row) = screen_position(x, y, tail.0 as u16, tail.1 as u16);
            queue!(screen, MoveTo(col, row), Print("  "))?; // erase old tail
        }

        snake.push_front(new_head);
        let (col, row) = screen_position(x, y, new_head.0 as u16, new_head.1 as u16);
        queue!(screen, MoveTo(col, row), Print("██"))?; // draw new head

        if ate_apple {
            apple = spawn_apple(&snake); // snake already contains new_head here
            let question = get_question();
            ask_question(screen, &question, x, y)?;
            execute!(screen, Clear(ClearType::All))?;
            draw_board(screen, x, y, &snake, apple)?;
        }

        screen.flush()?; // send the whole frame at once
    }

    let (col, row) = screen_position(x, y, WIDTH/2 - 6, HEIGHT/2);
    execute!(screen, MoveTo(col, row), Print("Game over! Press any key"))?;
    while event::poll(Duration::ZERO)? { event::read()?; } // throw away keys buffered before death
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                break;
            }
        }
    }

    Ok(())
}

// draws the border, the whole snake, and the apple from the current game state
fn draw_board(screen: &mut io::Stdout, x: u16, y: u16, snake: &VecDeque<(i16, i16)>, apple: (i16, i16)) -> io::Result<()> {
    // create border (moved from main)
    for i in 0..WIDTH+2 {
        queue!(screen, MoveTo(x + i*2, y), Print("██"))?; // top border
        queue!(screen, MoveTo(x + i*2, y + (HEIGHT+1)), Print("██"))?; // bottom border
    }
    for i in 0..HEIGHT+2 {
        queue!(screen, MoveTo(x, y + i), Print("██"))?; // left border
        queue!(screen, MoveTo(x + (WIDTH+1)*2, y + i), Print("██"))?; // right border
    }

    // snake
    for &(sx, sy) in snake.iter() {
        let (col, row) = screen_position(x, y, sx as u16, sy as u16);
        queue!(screen, MoveTo(col, row), Print("██"))?;
    }

    // apple
    draw_apple(screen, x, y, apple)?;

    screen.flush()?; // text goes into a waiting area until displayed to terminal flush instantly sends it
    Ok(())
}

// makes apple red color
fn draw_apple(screen: &mut io::Stdout, x: u16, y: u16, apple: (i16, i16)) -> io::Result<()> {
    let (col, row) = screen_position(x, y, apple.0 as u16, apple.1 as u16);
    queue!(
        screen,
        MoveTo(col, row),
        SetForegroundColor(Color::Red),
        Print("██"),
        ResetColor // without this, everything printed afterwards stays red
    )?;
    Ok(())
}

// spawns apple at random position
fn spawn_apple(snake: &VecDeque<(i16, i16)>) -> (i16, i16) {
    loop {
        let pos = (
            rand::rng().random_range(0..WIDTH as i16),
            rand::rng().random_range(0..HEIGHT as i16),
        );
        if !snake.contains(&pos) {
            return pos; // retry if it landed on the snake
        }
    }
}

// helper function convert to screen position
fn screen_position (origin_x :u16, origin_y :u16, x :u16, y :u16) -> (u16, u16) { // (col, row)
    return (origin_x+((x+1)*2),origin_y+(y+1));
}

fn ask_question(screen: &mut io::Stdout, question: &Question, x: u16, y: u16) -> io::Result<()> {
    execute!(screen, Clear(ClearType::All))?;
    execute!(screen, MoveTo(x, y), Print(&question.prompt))?;

    while event::poll(Duration::ZERO)? { event::read()?; }
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press { break; }
        }
    }
    Ok(())
}

fn get_question () -> Question {
    Question {prompt: "The answer is C?".to_string(), answers: ["not the answer".to_string(), "not the answer".to_string(), "ANSWER".to_string(), "not the answer".to_string()], answer_index: 2}
}
