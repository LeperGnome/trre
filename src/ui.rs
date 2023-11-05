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
    offset: usize,
    need_rerender: bool,
}

const MAX_CHILD_RENDERED: usize = 6;

impl AppState {
    pub fn new_from_fs(path: &str) -> Self {
        Self {
            loc: VecDeque::new(),
            root: DirInfo::new_from_fs(path),
            offset: 0,
            need_rerender: true,
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
            "|   ".repeat(depth.saturating_sub(1)),
            line
        )),
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
    highlight_current: bool,
    mut lines_left: usize,
    mut lines_left_to_skip: usize,
) -> io::Result<(usize, usize)> {
    if let Children::Some(chs) = chs {
        let top_punc_line: &str;
        if chs.selected >= MAX_CHILD_RENDERED {
            top_punc_line = "\\______ ... ___";
        } else {
            top_punc_line = "\\______________";
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
            if idx == chs.selected
                && highlight_current  // i'm on a valid path
                && matches!(local_loc, None)
            // i'm in a leaf
            {
                queue!(w, style::SetBackgroundColor(style::Color::DarkGrey))?;
            }
            if lines_left == 0 {
                return Ok((0, lines_left_to_skip));
            }

            if lines_left_to_skip > 0 {
                lines_left_to_skip -= 1;
            } else {
                lines_left = lines_left.saturating_sub(1);
            }
            match **ch {
                Node::Dir(ref dir) => {
                    if lines_left_to_skip == 0 {
                        // TODO this is getting ugly
                        queue!(
                            w,
                            style::SetForegroundColor(style::Color::DarkGrey),
                            style::Print(format!("{}", "|   ".repeat(depth))),
                            style::SetForegroundColor(style::Color::Magenta),
                            style::Print(format!("{}/", dir.name)),
                            cursor::MoveToNextLine(1),
                            style::ResetColor,
                        )?;
                    }
                    let highlight_next;
                    if let Some(l) = local_loc {
                        highlight_next = l == idx;
                    } else {
                        highlight_next = false;
                    }
                    (lines_left, lines_left_to_skip) = render_children(
                        w,
                        &dir.children,
                        loc.clone(),
                        depth + 1,
                        highlight_next,
                        lines_left,
                        lines_left_to_skip,
                    )?;
                }
                Node::File(ref f) => {
                    if lines_left_to_skip == 0 {
                        // TODO this is getting ugly
                        queue!(
                            w,
                            style::SetForegroundColor(style::Color::DarkGrey),
                            style::Print(format!("{}", "|   ".repeat(depth))),
                            style::SetForegroundColor(style::Color::White),
                            style::Print(format!("{}", f.name)),
                            cursor::MoveToNextLine(1),
                        )?;
                    }
                }
            }
            queue!(w, style::ResetColor)?;
        }
        let bottom_punc_line: &str;
        if chs.list.len() > MAX_CHILD_RENDERED && chs.selected != chs.list.len() - 1 {
            bottom_punc_line = "\\______ ... ___";
        } else {
            bottom_punc_line = "\\______________";
        }
        if depth != 0 {
            draw_punc(w, depth, bottom_punc_line)?;
        }
    }
    Ok((lines_left, lines_left_to_skip))
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
    if !app.need_rerender {
        return Ok(());
    }

    queue!(
        w,
        style::ResetColor,
        terminal::Clear(terminal::ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 0)
    )?;

    render_top_bar(app, w)?;

    render_children(
        w,
        &app.root.children,
        app.loc.clone(),
        0,
        true,
        terminal::size().unwrap().1 as usize - 5,
        app.offset,
    )?;

    // queue!(
    //     w,
    //     cursor::MoveToNextLine(1),
    //     cursor::MoveToNextLine(1),
    //     style::Print(format!("loc: {:?}", &app.loc))
    // )?;

    w.flush()?;
    Ok(())
}

pub fn run_app<W>(mut app: AppState, w: &mut W, tick_rate: Duration) -> io::Result<()>
where
    W: io::Write,
{
    let mut last_tick = Instant::now();
    loop {
        if app.need_rerender {
            render(&app, w)?;
            app.need_rerender = false;
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.need_rerender = true; // TODO: will this always work the way I want it?
                if let Err(_) = process_key(&mut app, key) {
                    return Ok(());
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // TODO: Do I need this?
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
                    // app.offset = app.offset.saturating_add(1);
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(cur) = app.root.get_selected_by_locatoin(app.loc.clone()) {
                if cur > 0 {
                    app.root.set_selected_by_location(cur - 1, app.loc.clone());
                    // app.offset = app.offset.saturating_sub(1);
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
