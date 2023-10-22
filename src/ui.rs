use crate::nodes::*;
use std::collections::VecDeque;

use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    queue, style, terminal,
};

pub struct AppState {
    loc: Location,
    root: DirInfo,
    offset: usize,
    need_rerender: bool,
}

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

fn render_children<W: io::Write>(
    w: &mut W,
    chs: &Children,
    mut loc: Location,
    depth: usize,
    highlight_current: bool,
    mut lines_left: usize,
    mut lines_left_to_skip: usize,
) -> io::Result<(usize, usize)> {
    queue!(w, style::ResetColor)?;
    if let Children::Some(chs) = chs {
        let cur_loc = loc.pop_front();
        for (idx, ch) in chs.list.iter().enumerate() {
            if idx == chs.selected
                && highlight_current  // i'm on a valid path
                && matches!(cur_loc, None)
            // i'm in a leaf
            {
                queue!(w, style::SetBackgroundColor(style::Color::DarkGrey),)?;
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
                            style::SetForegroundColor(style::Color::Magenta),
                            style::Print(format!("{}{}/", "    ".repeat(depth), dir.name)),
                            cursor::MoveToNextLine(1),
                        )?;
                    }
                    let highlight_next;
                    if let Some(l) = cur_loc {
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
                            style::Print(format!("{}{}", "    ".repeat(depth), f.name)),
                            cursor::MoveToNextLine(1),
                        )?;
                    }
                }
            }
            queue!(w, style::ResetColor)?;
        }
    }
    Ok((lines_left, lines_left_to_skip))
}

fn get_object_repr<O: FsObject>(obj: &O) -> String {
    return format!("> {}\n\n\r", obj.fullpath());
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

    if let Some(node) = app.root.get_node_by_location(app.loc.clone()) {
        match **node {
            Node::Dir(ref d) => queue!(w, style::Print(get_object_repr(d)))?,
            Node::File(ref f) => queue!(w, style::Print(get_object_repr(f)))?,
        };
    }

    render_children(
        w,
        &app.root.children,
        app.loc.clone(),
        0,
        true,
        terminal::size().unwrap().1 as usize - 5,
        app.offset,
    )?;

    queue!(w, style::Print(format!("\n\r\n\rloc: {:?}", &app.loc)))?;

    w.flush()?;
    Ok(())
}

//
// dir1/
// dir2/
//     somef1
//     somef2
//     somedir/
//         moref1
//         moredir/
// file1
// file2
//

fn need_offset_increase() -> bool {
    return true;
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

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Left | KeyCode::Char('h') => {
                        _ = app.loc.pop_back();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if let Some(node) = app.root.get_node_by_location_mut(app.loc.clone()) {
                            match **node {
                                Node::Dir(ref mut d) => {
                                    d.read_children();
                                    if let Some(deep_current) =
                                        app.root.get_current(app.loc.clone())
                                    {
                                        app.loc.push_back(deep_current);
                                    }
                                }
                                Node::File(_) => (), // TODO ?
                            }
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let chn = app.root.get_children_len_by_location(app.loc.clone());
                        if let Some(cur) = app.root.get_current(app.loc.clone()) {
                            if cur < chn.saturating_sub(1) {
                                app.root.set_current(cur + 1, app.loc.clone());
                                // app.offset = app.offset.saturating_add(1);
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(cur) = app.root.get_current(app.loc.clone()) {
                            if cur > 0 {
                                app.root.set_current(cur - 1, app.loc.clone());
                                // app.offset = app.offset.saturating_sub(1);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(node) = app.root.get_node_by_location_mut(app.loc.clone()) {
                            match **node {
                                Node::Dir(ref mut d) => d.collapse_or_expand(),
                                Node::File(_) => (), // TODO: should there be a message? cant collapse
                                                     // file
                            }
                        };
                    }
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
