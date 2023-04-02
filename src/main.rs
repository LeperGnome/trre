//
//  number of leaf nodes should be dynamic
//  if i can display all the child nodes, then i should do that
//
//  lev. 0    lev. 1    lev. 2
// --------------------------------
//                  |- another.json
//          |- baz -|- sample.txt
//          |
// cur_dir -|       |- one.c
//          |       |- two.c
//          |- bar -|- three.c
//                  |- four.c
// --------------------------------
//
//  if i can't, then i should shrink the number of them, so all of their parents are visible
//
//  lev. 0    lev. 1    lev. 2      lev. 3
// -------------------------------------------
//                     |- another.json
//          |- baz ----|- sample.txt
//          |
//          |          |- one.c
//          |          |- two.c
// cur_dir -|- bar ----|- three.c
//          |          |- four.c
//          |          |- ...
//          |                            ┏━ world.rs
//          |                            ┣━ hello.rs
//          |- foo ━━━━┳━ foo2 ----------╋━ aa.rs
//          |- i.py    ┗━ more.xml       ┣━ bb.rs
//                                       ┣━ cc.rs
//                                       ┗━ dd.o
// -------------------------------------------
//
//  if i can't even display all the nodes at lev. 1, then i should shrink those, like this:
//
//  lev. 0    lev. 1    lev. 2
// --------------------------------
//          |- ...
//          |- baz -|- ...
// cur_dir -|- bar -|- ...
//          |- foo -|- ...
//          |- ...
// --------------------------------
//
// But in that case, my application is not really usefull...
// I need to be able to prioritize certain directories
//
//  lev. 0    lev. 1    lev. 2
// --------------------------------
//         ...
//          |       |- another.json
//          |- baz -|- sample.txt
//          |
//          |       |- one.c
//          |       |- two.c
// cur_dir -|- bar -|- three.c
//          |       |- four.c
//         ...      |- ...
//
// --------------------------------
//
//  Or maybe add the ability to open a couple of them in a different pane
//
//  lev. 0    lev. 1    lev. 2      lev. 0    lev. 1    lev. 2
// -------------------------------|-------------------------------
//          |- ...                |          |- one.c
//          |- baz -...           | .../bar -|- two.c
// cur_dir -|- bar -...           |          |- ...
//          |- foo - ...          |
//          |- foobar - ...       |          |-
//          |- ...                | .../foo -|-
// -------------------------------|-------------------------------
//
// Or may be basic tree structure is fine
// Adding horizontal lines really made sense
//
// -------------------------------------------
// cur_dir
//  |-------------------------|
//  |- baz                    |
//  |   |---------------|     |
//  |   |- another.json |     |
//  |   |- sample.txt   |     |
//  |   ┗---------------|     |
//  |- bar                    |
//  |   |----------|          |  // the probem with those lines is that
//  |   |- one.c   |          |     parent need to know size of its children (n generations)
//  |   |- two.c   |          |     May be width can be fixed?
//  |   |- three.c |          |
//  |   |- four.c  |          |
//  |   ┗- ... ----|          | <- those indicate, that there's more down
//  |- foo                    |
//  |   |-----------------|   |
//  |   |- foo2           |   |
//  |   |   |- ... -----| |   | <- and those indicate, that there's more up
//  |   |   |- world.rs | |   | <- pres 'k' until you hit top, then '...' at top will change to '----'
//  |   |   |- hello.rs | |   |
//  |   |   |- aa.rs    | |   |
//  |   |   ┗-----------| |   | <- this will become '...', when you scroll up
//  |   |-i.py            |   |
//  ┗   ┗-----------------|   |
//  ┗- ... -------------------| <- what if I go down there...?
//
//  I can see huuuuuuuuuge problems rendering this (----.-------)
//  ..
//
//  And without horizontal lines for comparison:
//
//  lev. 0    lev. 1    lev. 2      lev. 3
// -------------------------------------------
//  .
//  |- baz
//  |  |- another.json
//  |  ┗- sample.txt
//  |
//  |- bar
//  |  |- one.c
//  |  |- two.c
//  |  |- three.c
//  |  |- four.c
//  |  ┗- ...   <- thos indicate, that there's more,
//  |
//  ┗- foo
//     |- foo2
//     |  |- world.rs
//     |  |- hello.rs
//     |  |- aa.rs
//     |  ┗- ...
//     |
//     ┗-i.py
// ------------------------------------------
//
// I need clearly defined levels, so i can hjkl here

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    fs, io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
};

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn add(&mut self, item: T) {
        self.items.push(item);
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

struct ListItemInfo {
    name: String,
    path: String,
    depth: usize,
    is_last: bool,
    is_dir: bool,
}

impl ListItemInfo {
    fn indented_name(&self) -> String {
        let indent = (0..self.depth).map(|_| "|  ").collect::<String>();
        format!("{}|- {}", indent, self.name)
    }
}

struct App {
    items: StatefulList<ListItemInfo>,
}

impl App {
    fn new() -> Self {
        Self {
            items: StatefulList::with_items(vec![]),
        }
    }

    fn read_dir(&mut self, root: &str, depth: usize, max_depth: usize) {
        let mut paths = fs::read_dir(root).unwrap().peekable(); // TODO
        while let Some(tree_el) = paths.next() {
            let tree_el = tree_el.unwrap();
            let name = tree_el.file_name().into_string().unwrap();
            let path = tree_el.path();
            let path = path.to_str().unwrap(); // TODO

            match tree_el.file_type() {
                Ok(t) => {
                    self.items.add(ListItemInfo {
                        name,
                        depth,
                        path: path.into(),
                        is_dir: t.is_dir(),
                        is_last: !paths.peek().is_some(),
                    });
                    if depth != max_depth && t.is_dir() {
                        self.read_dir(path, depth + 1, max_depth)
                    }
                }
                Err(_) => continue,
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);

    // TODO: Get directory from args
    let mut app = App::new();
    app.read_dir("./", 0, 2);

    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Left => app.items.unselect(),
                    KeyCode::Down | KeyCode::Char('j') => app.items.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.items.previous(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // TODO: Do I need this?
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100), Constraint::Percentage(100)].as_ref())
        .split(f.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|el| {
            let lines: Vec<Spans>;
            let c: Color;
            if el.is_dir {
                c = Color::Green;
                lines = vec![Spans::from(format!("{}/", el.indented_name()))];
            } else {
                c = Color::White;
                lines = vec![Spans::from(el.indented_name())];
            }
            ListItem::new(lines).style(Style::default().fg(c))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::BOTTOM).title("Trre"))
        .highlight_style(
            Style::default()
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    // We can now render the item list
    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
}
