use crate::nodes::*;
use std::collections::VecDeque;

use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    queue, style, terminal,
};

pub struct AppState {
    loc: Location,
    root: DirInfo,
}

const MAX_CHILD_RENDERED: usize = 7;
const PADDING: &str = "│  ";
const PADDING_MORE_UP: &str = "▲  ";
const PADDING_MORE_DOWN: &str = "▼  ";

impl AppState {
    pub fn new_from_fs(path: &str) -> Self {
        Self {
            loc: VecDeque::new(),
            root: DirInfo::new_from_fs(path),
        }
    }
}

fn draw_node<W: io::Write>(
    w: &mut W,
    depth: usize,
    node: &Node,
    highlight: bool,
    more_up: bool,
    more_down: bool,
) -> io::Result<()> {
    let (name, fgcolor) = match *node {
        Node::Dir(ref dir) => (format!("{}/", dir.name), style::Color::Magenta),
        Node::File(ref f) => (f.name.clone(), style::Color::White),
    };

    let bgcolor = match highlight {
        true => style::Color::DarkGrey,
        false => style::Color::Black,
    };

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
        style::SetForegroundColor(style::Color::Yellow),
        style::Print(padding),
        style::SetForegroundColor(fgcolor),
        style::SetBackgroundColor(bgcolor),
        style::Print(name),
        style::ResetColor,
        terminal::Clear(terminal::ClearType::UntilNewLine),
        cursor::MoveToNextLine(1),
    )?;
    Ok(())
}

fn render_children<W: io::Write>(
    w: &mut W,
    chs: &Children,
    mut loc: Location,
    depth: usize,
    in_selected_branch: bool,
    mut lines_capacity: usize,
    max_children: usize,
) -> io::Result<usize> {
    if let Children::Some(chs) = chs {
        let mut more_up: bool = false;
        let mut more_down: bool = false;
        if chs.selected >= max_children {
            more_up = true;
        }
        if chs.list.len() > max_children && chs.selected != chs.list.len() - 1 {
            more_down = true;
        }
        let local_loc = loc.pop_front();

        let mut skip_n: usize = 0;
        if chs.selected >= max_children {
            skip_n = chs.selected - max_children + 1;
        }
        for (idx, (gid, ch)) in chs
            .list
            .iter()
            .enumerate()
            .skip(skip_n)
            .enumerate()
            .take(max_children)
        {
            if lines_capacity == 0 {
                break;
            }

            let mut should_highlight = false;
            if gid == chs.selected && in_selected_branch && matches!(local_loc, None) {
                // i'm a selected node!
                should_highlight = true;
            }

            draw_node(
                w,
                depth,
                ch,
                should_highlight,
                more_up && idx == 0,
                more_down && idx == max_children - 1,
            )?;

            lines_capacity = lines_capacity.saturating_sub(1);

            if let Node::Dir(ref dir) = **ch {
                let highlight_next;
                if let Some(l) = local_loc {
                    highlight_next = l == idx;
                } else {
                    highlight_next = false;
                }
                lines_capacity = render_children(
                    w,
                    &dir.children,
                    loc.clone(),
                    depth + 1,
                    highlight_next,
                    lines_capacity,
                    lines_capacity.saturating_sub(2).min(MAX_CHILD_RENDERED),
                )?;
            }
            queue!(w, style::ResetColor)?;
        }
    }
    Ok(lines_capacity)
}

fn get_object_repr<O: FsObject>(obj: &O) -> String {
    return format!("> {}", obj.fullpath());
}

fn render_top_bar<W>(app: &AppState, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    if let Some(node) = app.root.get_node_by_location(app.loc.clone()) {
        match **node {
            Node::Dir(ref d) => queue!(w, style::Print(get_object_repr(d)))?,
            Node::File(ref f) => queue!(w, style::Print(get_object_repr(f)))?,
        };
        queue!(
            w,
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1)
        )?;
    }
    Ok(())
}

fn render<W>(app: &AppState, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    queue!(w, cursor::Hide, style::ResetColor, cursor::MoveTo(0, 0))?;

    render_top_bar(app, w)?;
    let lines_left = render_children(
        w,
        &app.root.children,
        app.loc.clone(),
        1,
        true,
        terminal::size().unwrap().1 as usize - 3,
        terminal::size().unwrap().1 as usize - 3,
    )?;
    for _ in 0..=lines_left {
        queue!(
            w,
            style::Print("~"),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
        )?;
    }
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
        KeyCode::Left | KeyCode::Char('h') => {
            _ = app.loc.pop_back();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if let Some(node) = app.root.get_node_by_location_mut(app.loc.clone()) {
                match **node {
                    Node::Dir(ref mut d) => {
                        d.read_children();
                        match d.children {
                            Children::Some(_) => {
                                if let Some(deep_current) =
                                    app.root.get_selected_by_locatoin(app.loc.clone())
                                {
                                    app.loc.push_back(deep_current);
                                }
                            }
                            Children::None | Children::Unread => (),
                        }
                    }
                    Node::File(_) => (), // TODO ?
                }
            };
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let chn = app.root.get_children_len_by_location(app.loc.clone());
            if let Some(cur) = app.root.get_selected_by_locatoin(app.loc.clone()) {
                if cur < chn.saturating_sub(1) {
                    app.root.set_selected_by_location(cur + 1, app.loc.clone());
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(cur) = app.root.get_selected_by_locatoin(app.loc.clone()) {
                if cur > 0 {
                    app.root.set_selected_by_location(cur - 1, app.loc.clone());
                }
            }
        }
        KeyCode::Enter => {
            if let Some(node) = app.root.get_node_by_location_mut(app.loc.clone()) {
                match **node {
                    Node::Dir(ref mut d) => d.collapse_or_expand(),
                    Node::File(_) => (),
                }
            };
        }
        _ => {}
    }
    Ok(())
}
