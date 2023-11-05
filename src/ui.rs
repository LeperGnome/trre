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

const MAX_CHILD_RENDERED: usize = 6;
const PADDING: &str = "│  ";
const TOP_PUNC_LINE_MORE: &str = "├────── ... ──────";
const TOP_PUNC_LINE_DONE: &str = "├─────────────────";
const BOTTOM_PUNC_LINE_MORE: &str = "└────── ... ──────";
const BOTTOM_PUNC_LINE_DONE: &str = "└─────────────────";

impl AppState {
    pub fn new_from_fs(path: &str) -> Self {
        Self {
            loc: VecDeque::new(),
            root: DirInfo::new_from_fs(path),
        }
    }
}

fn draw_punc<W: io::Write>(w: &mut W, depth: usize, line: &str) -> io::Result<()> {
    queue!(
        w,
        style::ResetColor,
        style::SetForegroundColor(style::Color::DarkGrey),
        style::Print(format!(
            "{}{}",
            PADDING.repeat(depth.saturating_sub(1)),
            line
        )),
        cursor::MoveToNextLine(1),
        style::ResetColor,
    )?;
    Ok(())
}

fn draw_node<W: io::Write>(w: &mut W, depth: usize, node: &Node) -> io::Result<()> {
    let name: String;
    let fgcolor: style::Color;
    match node {
        Node::Dir(ref dir) => {
            name = format!("{}/", dir.name);
            fgcolor = style::Color::Magenta;
        }
        Node::File(ref f) => {
            name = f.name.clone();
            fgcolor = style::Color::White;
        }
    };
    queue!(
        w,
        style::SetForegroundColor(style::Color::DarkGrey),
        style::Print(format!("{}", PADDING.repeat(depth))),
        style::SetForegroundColor(fgcolor),
        style::Print(name),
        cursor::MoveToNextLine(1),
        style::ResetColor,
    )?;
    Ok(())
}

fn render_children<W: io::Write>(
    w: &mut W,
    chs: &Children,
    mut loc: Location,
    depth: usize,
    in_selected_branch: bool,
) -> io::Result<()> {
    if let Children::Some(chs) = chs {
        let top_punc_line: &str;
        if chs.selected >= MAX_CHILD_RENDERED {
            top_punc_line = TOP_PUNC_LINE_MORE;
        } else {
            top_punc_line = TOP_PUNC_LINE_DONE;
        }
        if depth != 0 {
            draw_punc(w, depth, top_punc_line)?;
        }
        let local_loc = loc.pop_front();

        let mut skip_n: usize = 0;
        if chs.selected >= MAX_CHILD_RENDERED {
            skip_n = chs.selected - MAX_CHILD_RENDERED + 1;
        }
        for (idx, ch) in chs
            .list
            .iter()
            .enumerate()
            .skip(skip_n)
            .take(MAX_CHILD_RENDERED)
        {
            if idx == chs.selected && in_selected_branch && matches!(local_loc, None) {
                // i'm a selected node!
                queue!(w, style::SetBackgroundColor(style::Color::DarkGrey))?;
            }

            draw_node(w, depth, ch)?;

            if let Node::Dir(ref dir) = **ch {
                let highlight_next;
                if let Some(l) = local_loc {
                    highlight_next = l == idx;
                } else {
                    highlight_next = false;
                }
                render_children(w, &dir.children, loc.clone(), depth + 1, highlight_next)?;
            }
            queue!(w, style::ResetColor)?;
        }
        let bottom_punc_line: &str;
        if chs.list.len() > MAX_CHILD_RENDERED && chs.selected != chs.list.len() - 1 {
            bottom_punc_line = BOTTOM_PUNC_LINE_MORE;
        } else {
            bottom_punc_line = BOTTOM_PUNC_LINE_DONE;
        }
        if depth != 0 {
            draw_punc(w, depth, bottom_punc_line)?;
        }
    }
    Ok(())
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
        queue!(w, cursor::MoveToNextLine(1), cursor::MoveToNextLine(1))?;
    }
    Ok(())
}

fn render<W>(app: &AppState, w: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    queue!(
        w,
        style::ResetColor,
        terminal::Clear(terminal::ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 0)
    )?;

    render_top_bar(app, w)?;
    render_children(w, &app.root.children, app.loc.clone(), 0, true)?;
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
