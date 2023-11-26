// use crate::nodes::*;
use crate::arenatree::*;
// use std::collections::VecDeque;

use std::{
    io,
    // process::Command,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    queue, style,
    style::Stylize,
    terminal,
};

enum OpType {
    Copy(usize),
    Cut(usize),
}

pub struct AppState {
    tree: ArenaTree,
    op_buff: Option<OpType>,
    bottom_satatus: String,
}

const MAX_CHILD_RENDERED: usize = 7;
const PADDING: &str = "│  ";
const PADDING_MORE_UP: &str = "▲  ";
const PADDING_MORE_DOWN: &str = "▼  ";

impl AppState {
    pub fn new_from_fs(path: &str) -> Self {
        let mut tree = ArenaTree::new(path);
        tree.read_children(0);

        if tree.get(0).children.len() > 0 {
            tree.current = 0;
        }
        Self {
            tree,
            op_buff: None,
            bottom_satatus: String::from("--"),
        }
    }
}

fn render_node<W: io::Write>(
    w: &mut W,
    depth: usize,
    node: &Node,
    highlight: bool,
    more_up: bool,
    more_down: bool,
) -> io::Result<()> {
    let mut name = match node.is_dir {
        true => format!("{}/", node.name).magenta(),
        false => node.name.clone().white(),
    };

    if highlight {
        name = name.black().on_white();
    }

    let padding: String;

    if more_up {
        padding = format!("{}{}", PADDING.repeat(depth - 1), PADDING_MORE_UP)
    } else if more_down {
        padding = format!("{}{}", PADDING.repeat(depth - 1), PADDING_MORE_DOWN)
    } else {
        padding = format!("{}", PADDING.repeat(depth))
    }
    queue!(
        w,
        style::SetForegroundColor(style::Color::Grey),
        style::Print(padding),
        style::Print(name),
        style::ResetColor,
        terminal::Clear(terminal::ClearType::UntilNewLine),
        cursor::MoveToNextLine(1),
    )?;
    Ok(())
}

fn render_top_bar<W>(app: &AppState, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    let node = app.tree.get_current();
    queue!(
        w,
        style::Print(&node.fullpath),
        terminal::Clear(terminal::ClearType::UntilNewLine),
        cursor::MoveToNextLine(1),
        terminal::Clear(terminal::ClearType::UntilNewLine),
        cursor::MoveToNextLine(1)
    )?;
    Ok(())
}

fn render_bottom_bar<W>(app: &AppState, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    let s = match app.op_buff {
        _ => app.bottom_satatus.clone(),
        // TODO
        // Some(ref o) => match o {
        //     OpType::Copy(ref n) => format!("Copying: {}", n.get_full_path()),
        //     OpType::Cut(ref n) => format!("Moving: {}", n.get_full_path()),
        // },
        // None => app.bottom_satatus.clone(),
    };
    queue!(
        w,
        style::Print(s),
        terminal::Clear(terminal::ClearType::UntilNewLine),
        cursor::MoveToNextLine(1),
    )?;
    Ok(())
}

fn render_tree<W>(
    w: &mut W,
    tree: &ArenaTree,
    idx: usize,
    depth: usize,
    mut lines_left: usize,
) -> io::Result<usize>
where
    W: io::Write,
{
    let current = tree.get(idx);
    if lines_left == 0 {
        return Ok(lines_left);
    }
    lines_left -= 1;
    render_node(w, depth, current, tree.current == idx, false, false)?;
    for chidx in tree.get(idx).children.iter() {
        lines_left = render_tree(w, tree, *chidx, depth + 1, lines_left)?;
    }
    return Ok(lines_left);
}

fn render<W>(app: &AppState, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    queue!(w, cursor::Hide, style::ResetColor, cursor::MoveTo(0, 0))?;

    render_top_bar(app, w)?;
    let lines_left = render_tree(w, &app.tree, 0, 0, terminal::size().unwrap().1 as usize - 3)?;
    for _ in 0..=lines_left {
        queue!(
            w,
            style::Print("~"),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
        )?;
    }
    render_bottom_bar(&app, w)?;

    w.flush()?;
    Ok(())
}

pub fn run_app<W>(mut app: AppState, w: &mut W, tick_rate: Duration) -> io::Result<()>
where
    W: io::Write,
{
    let mut last_tick = Instant::now();
    loop {
        render(&app, w)?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let Err(_) = process_key(&mut app, key) {
                    return Ok(());
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn process_key(app: &mut AppState, key: KeyEvent) -> Result<(), ()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Err(()),
        // KeyCode::Char('d') => {
        //     if let Some(node) = app.root.get_selected_node_by_location(app.loc.clone()) {
        //         app.op_buff = Some(OpType::Cut(node.clone()));
        //     }
        // }
        // KeyCode::Char('y') => {
        //     if let Some(node) = app.root.get_selected_node_by_location(app.loc.clone()) {
        //         app.op_buff = Some(OpType::Copy(node.clone()));
        //     }
        // }
        // KeyCode::Char('p') => {
        //     let to_dir = app.root.get_dir_by_location_mut(app.loc.clone());
        //     if let Some(ref op) = app.op_buff {
        //         match op {
        //             OpType::Copy(from) => {
        //                 Command::new("cp")
        //                     .args([&from.get_full_path(), &to_dir.fullpath])
        //                     .output()
        //                     .expect("failed to copy");
        //                 app.bottom_satatus = format!("Copied: {} -> {}", from.get_full_path(), &to_dir.fullpath);
        //                 app.op_buff = None;
        //                 to_dir.refresh();
        //             }
        //             OpType::Cut(from) => {
        //                 Command::new("mv")
        //                     .args([&from.get_full_path(), &to_dir.fullpath])
        //                     .output()
        //                     .expect("failed to copy");
        //                 app.bottom_satatus = format!("Moved: {} -> {}", from.get_full_path(), &to_dir.fullpath);
        //                 app.op_buff = None;
        //                 to_dir.refresh();
        //                 // TODO: refresh 'from' dir
        //             }
        //         }
        //     }
        // }
        KeyCode::Left | KeyCode::Char('h') => {
            if let Some(parent) = app.tree.get(app.tree.current).parent {
                app.tree.current = parent;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if !app.tree.get_current().is_dir {
                return Ok(());
            }
            if app.tree.get_current().children.len() == 0 {
                app.tree.read_children(app.tree.get_current().idx);
            }
            let node = app.tree.get_current();
            if node.children.len() == 0 {
                return Ok(());
            }
            app.tree.current = node.children[0];
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(parent) = app.tree.get_current().parent {
                if let Some(next_child) = app.tree.get(parent).next_child(app.tree.current) {
                    app.tree.current = next_child;
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(parent) = app.tree.get_current().parent {
                if let Some(prev_child) = app.tree.get(parent).previous_child(app.tree.current) {
                    app.tree.current = prev_child;
                }
            }
        }
        KeyCode::Enter => {
            let node = app.tree.get_current();
            if node.children.len() == 0 {
                app.tree.read_children(node.idx);
            } else {
                app.tree.remove_children(node.idx);
            }
        }
        _ => {}
    }
    Ok(())
}
