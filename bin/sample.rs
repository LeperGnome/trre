use std::collections::VecDeque;

use std::{
    error::Error,
    fs, io,
    time::{Duration, Instant},
};

use crossterm::{
    cursor,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, queue, style, terminal,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

#[derive(Debug)]
enum Node {
    Dir(DirInfo),
    File(FileInfo),
}

#[derive(Debug)]
struct ChildrenState {
    list: Vec<Box<Node>>,
    current: usize,
}

impl From<Vec<Box<Node>>> for ChildrenState {
    fn from(v: Vec<Box<Node>>) -> Self {
        Self {
            list: v,
            current: 0,
        }
    }
}

#[derive(Debug)]
enum Children {
    Some(ChildrenState),
    None,
    Unread,
}

#[derive(Debug)]
struct DirInfo {
    children: Children,
    fullpath: String,
    name: String,
}

#[derive(Debug)]
struct FileInfo {
    fullpath: String,
    name: String,
}

impl DirInfo {
    fn read_from_fs(&mut self) {
        let mut children = vec![];
        let mut paths = fs::read_dir(&self.fullpath).unwrap(); // TODO
        while let Some(child) = paths.next() {
            let child = child.unwrap(); // TODO
            let name = child.file_name().into_string().unwrap(); // TODO
            let cpath = child.path();
            let cpath = cpath.to_str().unwrap(); // TODO

            match child.file_type() {
                Ok(t) if t.is_dir() => {
                    children.push(Box::new(Node::Dir(DirInfo {
                        name,
                        fullpath: cpath.into(),
                        children: Children::Unread,
                    })));
                }
                Ok(t) if t.is_file() => {
                    children.push(Box::new(Node::File(FileInfo {
                        name,
                        fullpath: cpath.into(),
                    })));
                }
                Ok(_) | Err(_) => continue,
            }
        }
        self.children = Children::Some(ChildrenState::from(children));
    }

    fn collapse(&mut self) {
        self.children = Children::Unread;
    }

    fn new_from_fs(path: &str) -> Self {
        let mut s = Self {
            fullpath: path.into(),
            name: path.into(),
            children: Children::None,
        };
        s.read_from_fs();
        return s;
    }

    fn read_children(&mut self) {
        if matches!(self.children, Children::Unread) {
            self.read_from_fs();
        }
    }

    fn print_children(&mut self) {
        self.read_children();
        match &self.children {
            Children::Some(chs) => {
                for ch in &chs.list {
                    match **ch {
                        Node::Dir(ref v) => println!("Dir: {}\r", v.name),
                        Node::File(ref v) => println!("File: {}\r", v.name),
                    }
                }
            }
            Children::None | Children::Unread => (),
        };
    }

    fn print_by_locatoin(&mut self, mut loc: Location) {
        // println!("currently in {:?}", &self);
        self.read_children();
        if let Some(l) = loc.pop_front() {
            match &mut self.children {
                Children::Some(chs) => {
                    if let Some(child) = chs.list.get_mut(l) {
                        // println!("got child: {:?}", child);
                        match **child {
                            Node::Dir(ref mut dir) => dir.print_by_locatoin(loc),
                            _ => panic!("this is a file"), // TODO
                        }
                    } else {
                        panic!("no children found");
                    }
                }
                Children::None | Children::Unread => (),
            };
        } else {
            self.print_children();
        }
    }

    fn get_child_by_location(&mut self, mut loc: Location) -> &mut Box<Node> {
        // TODO can error
        self.read_children();
        match self.children {
            Children::Some(ref mut chs) => {
                if let Some(l) = loc.pop_front() {
                    match **chs.list.get_mut(l).unwrap() {
                        Node::Dir(ref mut d) => d.get_child_by_location(loc),
                        _ => panic!("Not a directory"),
                    }
                } else {
                    return &mut chs.list[chs.current]; // TODO: if let get_mut?
                }
            }
            Children::None | Children::Unread => panic!("no such child"), // TODO
        }
    }

    fn get_current(&mut self, mut loc: Location) -> Option<usize> {
        self.read_children();
        match &mut self.children {
            Children::Some(chs) => {
                if let Some(l) = loc.pop_front() {
                    if let Some(child) = chs.list.get_mut(l) {
                        match **child {
                            Node::Dir(ref mut dir) => dir.get_current(loc),
                            _ => panic!("this is a file"), // TODO
                        }
                    } else {
                        panic!("child with idx {} not found", l);
                    }
                } else {
                    Some(chs.current)
                }
            }
            Children::None | Children::Unread => None,
        }
    }

    fn set_current(&mut self, to: usize, mut loc: Location) {
        // TODO: from location
        self.read_children();
        match &mut self.children {
            Children::Some(ref mut chs) => {
                if let Some(l) = loc.pop_front() {
                    if let Some(child) = chs.list.get_mut(l) {
                        match **child {
                            Node::Dir(ref mut dir) => dir.set_current(to, loc),
                            _ => panic!("this is a file"), // TODO
                        }
                    }
                } else {
                    chs.current = to
                }
            }
            Children::None | Children::Unread => (),
        }
    }
}

type Location = VecDeque<usize>;

struct AppState {
    loc: Location,
    root: DirInfo,
    need_rerender: bool,
}

impl AppState {
    fn new_from_fs(path: &str) -> Self {
        Self {
            loc: VecDeque::new(),
            root: DirInfo::new_from_fs(path),
            need_rerender: true,
        }
    }
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
        terminal::Clear(ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 0)
    )?;

    fn render_children<W: io::Write>(
        w: &mut W,
        chs: &Children,
        mut loc: Location,
        depth: usize,
        highlight_current: bool,
    ) -> io::Result<()> {
        queue!(w, style::ResetColor)?;
        if let Children::Some(chs) = chs {
            let cur_loc = loc.pop_front();
            for (idx, ch) in chs.list.iter().enumerate() {
                if idx == chs.current
                    && highlight_current // i'm on a valid path
                    && matches!(cur_loc, None) // i'm in a leaf
                {
                    queue!(w, style::SetBackgroundColor(style::Color::DarkGrey),)?;
                }
                match **ch {
                    Node::Dir(ref dir) => {
                        queue!(
                            w,
                            style::SetForegroundColor(style::Color::Magenta),
                            style::Print(format!("{}{}/", "    ".repeat(depth), dir.name)),
                            cursor::MoveToNextLine(1),
                        )?;
                        let highlight_next;
                        if let Some(l) = cur_loc {
                            highlight_next = l == idx;
                        } else {
                            highlight_next = false;
                        }
                        render_children(w, &dir.children, loc.clone(), depth + 1, highlight_next)?;
                    }
                    Node::File(ref f) => {
                        queue!(
                            w,
                            style::Print(format!("{}{}", "    ".repeat(depth), f.name)),
                            cursor::MoveToNextLine(1),
                        )?;
                    }
                }
                queue!(w, style::ResetColor)?;
            }
        }
        Ok(())
    }

    render_children(w, &app.root.children, app.loc.clone(), 0, true)?;

    w.flush()?;
    Ok(())
}

fn run_app<W>(mut app: AppState, w: &mut W, tick_rate: Duration) -> io::Result<()>
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
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Left | KeyCode::Char('h') => {
                        _ = app.loc.pop_back();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        match **app.root.get_child_by_location(app.loc.clone()) {
                            Node::Dir(ref mut d) => {
                                d.read_children();
                                if let Some(deep_current) = app.root.get_current(app.loc.clone()) {
                                    app.loc.push_back(deep_current);
                                }
                            }
                            Node::File(_) => (), // TODO ?
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(cur) = app.root.get_current(app.loc.clone()) {
                            // TODO limit
                            app.root.set_current(cur + 1, app.loc.clone());
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(cur) = app.root.get_current(app.loc.clone()) {
                            if cur > 0 {
                                app.root.set_current(cur - 1, app.loc.clone());
                            }
                        }
                    }
                    KeyCode::Enter => {
                        match **app.root.get_child_by_location(app.loc.clone()) {
                            Node::Dir(ref mut d) => d.collapse(),
                            Node::File(_) => (), // TODO: should there be a message? cant collapse
                                                 // file
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

fn main() -> Result<(), Box<dyn Error>> {
    //
    // TODO:
    // 1. need to refactor Location. is this really a good idea to have it?
    //
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::MoveTo(0, 0),)?;

    let tick_rate = Duration::from_millis(500);

    let state = AppState::new_from_fs("./");
    let res = run_app(state, &mut stdout, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
