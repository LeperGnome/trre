mod arenatree;
mod nodes;
mod ui;

use std::{env::args, error::Error, io, time::Duration};

use crossterm::{
    cursor, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() -> Result<(), Box<dyn Error>> {
    let dir: String;

    match args().collect::<Vec<String>>().get(1) {
        Some(s) => dir = s.clone(),
        None => dir = String::from("."),
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::MoveTo(0, 0),)?;

    let tick_rate = Duration::from_millis(100);

    let state = ui::AppState::new_from_fs(&dir);
    let res = ui::run_app(state, &mut stdout, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
