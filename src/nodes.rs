use std::{collections::VecDeque, fs};

pub type Location = VecDeque<usize>;

#[derive(Debug)]
pub enum Node {
    Dir(DirInfo),
    File(FileInfo),
}
impl Node {
    pub fn get_full_path(&self) -> String {
        match self {
            Node::Dir(dirinfo) => dirinfo.fullpath.clone(),
            Node::File(finfo) => finfo.fullpath.clone(),
        }
    }
}

#[derive(Debug)]
pub struct ChildrenState {
    pub list: Vec<Box<Node>>,
    pub selected: usize,
}

impl From<Vec<Box<Node>>> for ChildrenState {
    fn from(v: Vec<Box<Node>>) -> Self {
        Self {
            list: v,
            selected: 0,
        }
    }
}

#[derive(Debug)]
pub enum Children {
    Some(ChildrenState),
    None,
    Unread,
}

#[derive(Debug)]
pub struct DirInfo {
    pub children: Children,
    pub fullpath: String,
    pub name: String,
}

#[derive(Debug)]
pub struct FileInfo {
    pub fullpath: String,
    pub name: String,
}

impl DirInfo {
    pub fn new_from_fs(path: &str) -> Self {
        let mut s = Self {
            fullpath: path.into(),
            name: path.into(),
            children: Children::None,
        };
        s.read_from_fs();
        return s;
    }

    pub fn read_from_fs(&mut self) {
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
        if children.len() > 0 {
            self.children = Children::Some(ChildrenState::from(children));
        } else {
            self.children = Children::None;
        }
    }

    pub fn collapse_or_expand(&mut self) {
        match self.children {
            Children::Unread => self.read_children(),
            Children::Some(_) => self.children = Children::Unread,
            Children::None => (),
        }
    }

    pub fn read_children(&mut self) {
        if matches!(self.children, Children::Unread) {
            self.read_from_fs();
        }
    }

    pub fn get_selected_node_by_location_mut(
        &mut self,
        mut loc: Location,
    ) -> Option<&mut Box<Node>> {
        match self.children {
            Children::Some(ref mut chs) => {
                if let Some(l) = loc.pop_front() {
                    match **chs.list.get_mut(l).unwrap() {
                        Node::Dir(ref mut d) => d.get_selected_node_by_location_mut(loc),
                        _ => panic!("Not a directory"),
                    }
                } else {
                    return chs.list.get_mut(chs.selected);
                }
            }
            Children::None | Children::Unread => None,
        }
    }

    pub fn get_dir_by_location_mut(&mut self, mut loc: Location) -> &mut DirInfo {
        if let Some(l) = loc.pop_front() {
            match self.children {
                Children::Some(ref mut chs) => match **chs.list.get_mut(l).unwrap() {
                    Node::Dir(ref mut d) => d.get_dir_by_location_mut(loc),
                    _ => panic!("Not a directory"),
                },
                Children::None | Children::Unread => panic!("No children with non-empty location"),
            }
        } else {
            return self;
        }
    }

    pub fn get_selected_node_by_location(&self, mut loc: Location) -> Option<&Box<Node>> {
        match self.children {
            Children::Some(ref chs) => {
                if let Some(l) = loc.pop_front() {
                    match **chs.list.get(l).unwrap() {
                        Node::Dir(ref d) => d.get_selected_node_by_location(loc),
                        _ => panic!("Not a directory"),
                    }
                } else {
                    return chs.list.get(chs.selected);
                }
            }
            Children::None | Children::Unread => None,
        }
    }

    pub fn get_selected_by_locatoin(&self, mut loc: Location) -> Option<usize> {
        match &self.children {
            Children::Some(chs) => {
                if let Some(l) = loc.pop_front() {
                    if let Some(child) = chs.list.get(l) {
                        match **child {
                            Node::Dir(ref dir) => dir.get_selected_by_locatoin(loc),
                            _ => panic!("this is a file"), // TODO
                        }
                    } else {
                        panic!("child with idx {} not found", l);
                    }
                } else {
                    Some(chs.selected)
                }
            }
            Children::None | Children::Unread => None,
        }
    }

    pub fn set_selected_by_location(&mut self, to: usize, mut loc: Location) {
        match &mut self.children {
            Children::Some(ref mut chs) => {
                if let Some(l) = loc.pop_front() {
                    if let Some(child) = chs.list.get_mut(l) {
                        match **child {
                            Node::Dir(ref mut dir) => dir.set_selected_by_location(to, loc),
                            _ => panic!("this is a file"), // TODO
                        }
                    }
                } else {
                    chs.selected = to
                }
            }
            Children::None | Children::Unread => (),
        }
    }

    pub fn get_children_len_by_location(&self, mut loc: Location) -> usize {
        match &self.children {
            Children::Some(chs) => {
                if let Some(l) = loc.pop_front() {
                    if let Some(ch) = chs.list.get(l) {
                        match **ch {
                            Node::Dir(ref d) => d.get_children_len_by_location(loc),
                            Node::File(_) => 0,
                        }
                    } else {
                        0
                    }
                } else {
                    chs.list.len()
                }
            }
            Children::Unread | Children::None => 0,
        }
    }
}
