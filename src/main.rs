use std::io::{self, Write};
use std::collections::VecDeque;
use rand::RngExt; // rand 0.10: provides random_range
use serde::Deserialize; // lets serde turn the json file into Question structs
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind},
    style::{Print, SetForegroundColor, ResetColor, Color, SetAttribute, Attribute}, execute, queue,
    terminal::{EnterAlternateScreen, Clear, ClearType, LeaveAlternateScreen, enable_raw_mode, disable_raw_mode}};
use std::time::{Duration, Instant};

const WIDTH: u16 = 30;
const HEIGHT: u16 = 30;
const LABELS: [&str; 4] = ["A", "B", "C", "D"]; // letters shown next to the answers

#[derive(Deserialize)] // the field names in here must match the names in the json file
struct Question {
    prompt: String,
    answers: [String; 4],
    answer_index: usize // 0=A 1=B 2=C 3=D
}

fn main() -> io::Result<()> {
    // get terminal size if cols less than 40 or rows less than 20 don't start game
    let terminal_size = crossterm::terminal::size().unwrap();
    let (terminal_cols, terminal_rows) = terminal_size;
    if terminal_cols < (WIDTH + 2)*2 || terminal_rows < HEIGHT+2 {
        eprintln!("Terminal too small must have at least:\n{} columns\n{} rows", (WIDTH+2)*2, HEIGHT+2); // error message
        std::process::exit(1); // exit program
    }

    // json file with the questions, a different file can be given as the first argument: cargo run -- my_questions.json
    let questions_path = std::env::args().nth(1).unwrap_or("questions.json".to_string());
    let questions = match load_questions(&questions_path) {
        Ok(questions) => questions,
        Err(e) => {
            eprintln!("Could not load questions from {}: {}", questions_path, e); // error message
            std::process::exit(1); // exit program
        }
    };

    execute!(io::stdout(), EnterAlternateScreen, Hide)?; // enter "alternate screen mode" of terminal
    enable_raw_mode()?; // keys are delivered instantly, no Enter needed, no echo

    let mut out = io::stdout(); // out is literally what is displayed on screen

    let x = (terminal_cols - (WIDTH + 2) * 2) / 2;
    let y = (terminal_rows - (HEIGHT+2))/2;

    // ====================
    // START GAME
    // ====================
    let result = run_game(&mut out, x, y, &questions);

    // =====================
    // GAME OVER CLEAN UP
    // =====================
    let _ = disable_raw_mode(); // must run or the terminal stays broken
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show); // exit "alternate screen mode" of terminal
    result
}

fn run_game (screen: &mut io::Stdout, x: u16, y: u16, questions: &[Question]) -> io::Result<()> {
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

    let mut questions_asked: u32 = 0; // how many questions have been shown so far
    let mut questions_correct: u32 = 0; // how many of those were answered right

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
            let question = get_question(questions); // random question from the json file
            match ask_question(screen, question, x, y)? {
                Some(correct) => {
                    questions_asked += 1;
                    // a wrong answer currently does nothing except not count, change what happens here if you want a penalty
                    if correct {
                        questions_correct += 1;
                    }
                }
                None => return Ok(()), // player pressed q during the question
            }
            execute!(screen, Clear(ClearType::All))?;
            draw_board(screen, x, y, &snake, apple)?;
        }

        screen.flush()?; // send the whole frame at once
    }

    let (col, row) = screen_position(x, y, WIDTH/2 - 6, HEIGHT/2);
    execute!(screen, MoveTo(col, row), Print("Game over! Press any key"))?;
    let (col, row) = screen_position(x, y, WIDTH/2 - 6, HEIGHT/2 + 1);
    execute!(screen, MoveTo(col, row), Print(format!("Questions correct: {}/{}", questions_correct, questions_asked)))?; // score
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

// shows the question, lets the player pick an answer with up/down + enter, then shows if it was right
// returns Some(true) if correct, Some(false) if wrong, None if the player pressed q to quit
fn ask_question(screen: &mut io::Stdout, question: &Question, x: u16, y: u16) -> io::Result<Option<bool>> {
    let mut selected: usize = 0; // which answer is highlighted, 0=A 1=B 2=C 3=D

    while event::poll(Duration::ZERO)? { event::read()?; } // throw away keys buffered while playing so a stray arrow key doesn't pick an answer

    loop {
        draw_question(screen, question, selected, x, y)?;

        // event::read() waits until a key is pressed, this is what pauses the game
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Up => selected = (selected + 3) % 4, // +3 instead of -1 so A wraps around to D (usize can't go below 0)
                    KeyCode::Down => selected = (selected + 1) % 4,
                    KeyCode::Enter => break, // answer locked in
                    KeyCode::Char('q') => return Ok(None),
                    _ => {}
                }
            }
        }
    }

    let correct = selected == question.answer_index;
    draw_result(screen, question, correct, x, y)?;
    wait_for_key()?;

    Ok(Some(correct))
}

// draws the question and the four answers, the selected answer is highlighted
fn draw_question(screen: &mut io::Stdout, question: &Question, selected: usize, x: u16, y: u16) -> io::Result<()> {
    let text_x = x + 2; // text starts 2 columns in from the left edge of the game window
    let text_width = ((WIDTH + 2) * 2 - 4) as usize; // same width as the game window minus 2 columns of space on each side
    let mut row = y + 1;

    queue!(screen, Clear(ClearType::All))?; // wipe whatever was on screen before

    // question
    for line in wrap_text(&question.prompt, text_width) {
        queue!(screen, MoveTo(text_x, row), Print(line))?;
        row += 1;
    }
    row += 1; // empty line between the question and the answers

    // answers
    for (i, answer) in question.answers.iter().enumerate() {
        if i == selected {
            queue!(screen, SetAttribute(Attribute::Reverse))?; // swaps text and background color, this is the highlight
        }
        // long answers wrap onto more lines, -3 leaves room for the "A) " at the start
        for (j, line) in wrap_text(answer, text_width - 3).into_iter().enumerate() {
            let prefix = if j == 0 { format!("{}) ", LABELS[i]) } else { "   ".to_string() }; // only the first line gets the letter
            let text = format!("{}{}", prefix, line);
            let padded = format!("{:<width$}", text, width = text_width); // pad with spaces so the highlight is a solid bar
            queue!(screen, MoveTo(text_x, row), Print(padded))?;
            row += 1;
        }
        queue!(screen, SetAttribute(Attribute::NoReverse))?; // without this, everything printed afterwards stays highlighted
        row += 1; // empty line between answers
    }

    queue!(screen, MoveTo(text_x, row), Print("↑/↓ move    Enter select    q quit"))?;

    screen.flush()?; // send the whole frame at once
    Ok(())
}

// shows if the answer was right, and what the right answer was if it wasn't
fn draw_result(screen: &mut io::Stdout, question: &Question, correct: bool, x: u16, y: u16) -> io::Result<()> {
    let text_x = x + 2;
    let text_width = ((WIDTH + 2) * 2 - 4) as usize;
    let mut row = y + 1;

    queue!(screen, Clear(ClearType::All))?;

    if correct {
        queue!(screen, MoveTo(text_x, row), SetForegroundColor(Color::Green), Print("Correct!"), ResetColor)?;
        row += 1;
    } else {
        queue!(screen, MoveTo(text_x, row), SetForegroundColor(Color::Red), Print("Wrong!"), ResetColor)?;
        row += 2;
        queue!(screen, MoveTo(text_x, row), Print("The correct answer was:"))?;
        row += 1;
        let right_answer = format!("{}) {}", LABELS[question.answer_index], question.answers[question.answer_index]);
        for line in wrap_text(&right_answer, text_width) {
            queue!(screen, MoveTo(text_x, row), Print(line))?;
            row += 1;
        }
    }

    row += 1;
    queue!(screen, MoveTo(text_x, row), Print("Press any key to continue"))?;

    screen.flush()?;
    Ok(())
}

// waits until any key is pressed
fn wait_for_key() -> io::Result<()> {
    while event::poll(Duration::ZERO)? { event::read()?; } // throw away keys buffered before this screen showed up
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                break;
            }
        }
    }
    Ok(())
}

// splits text into lines that are at most `width` characters long, only breaks between words
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        // if adding this word (plus a space) goes past the width, start a new line
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new()); // empty text still gets one line so it shows up
    }
    lines
}

// picks a random question from the ones loaded from the json file
fn get_question(questions: &[Question]) -> &Question {
    &questions[rand::rng().random_range(0..questions.len())]
}

// reads the questions from the json file and checks that they are usable
fn load_questions(path: &str) -> io::Result<Vec<Question>> {
    let text = std::fs::read_to_string(path)?; // fails if the file doesn't exist
    let questions: Vec<Question> = serde_json::from_str(&text)?; // fails if the json is badly formatted or a question is missing something

    if questions.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "the file has no questions in it"));
    }
    for (i, question) in questions.iter().enumerate() {
        if question.answer_index >= 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("question {} has an answer_index outside of 0-3", i + 1)));
        }
    }

    Ok(questions)
}
