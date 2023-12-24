use std::cmp::Ordering;
use std::fs;

pub struct Node {
    pub idx: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub fullpath: String,
    pub name: String,
    pub is_dir: bool,
}
impl Node {
    pub fn previous_child(&self, idx: usize) -> Option<usize> {
        if let Some(current_pos) = self.children.iter().position(|&x| x == idx) {
            return self.children.get(current_pos.saturating_sub(1)).copied();
        }
        return None;
    }
    pub fn next_child(&self, idx: usize) -> Option<usize> {
        if let Some(current_pos) = self.children.iter().position(|&x| x == idx) {
            return self.children.get(current_pos + 1).copied();
        }
        return None;
    }
}

pub struct ArenaTree {
    pub arena: Vec<Node>,
    pub current: usize,
}

impl ArenaTree {
    pub fn new(path: &str) -> Self {
        return Self {
            current: 0,
            arena: vec![Node {
                idx: 0,
                parent: None,
                is_dir: true,
                children: vec![],
                fullpath: path.into(),
                name: path.into(),
            }],
        };
    }

    pub fn get_current(&self) -> &Node {
        return &self.arena[self.current];
    }

    pub fn get(&self, idx: usize) -> &Node {
        return &self.arena[idx];
    }

    pub fn remove_children(&mut self, idx: usize) {
        while let Some(rm_idx) = self.arena[idx].children.pop() {
            self.arena.remove(rm_idx);
        }
    }

    pub fn read_children(&mut self, idx: usize) -> Result<(), String> {
        if let None = self.arena.get(idx) {
            return Err("Node not found in arena".into());
        }
        if !self.arena[idx].is_dir {
            return Err("Selected node is a file".into());
        }
        let mut children_uninit = fs::read_dir(&self.arena[idx].fullpath)
            .unwrap()
            .map(|child| {
                let child = child.unwrap();
                let is_dir: bool;
                match child.file_type() {
                    Ok(t) if t.is_dir() => is_dir = true,
                    _ => is_dir = false,
                }
                Node {
                    idx: 0, // not setting proper id here, because elements are unsorted
                    parent: Some(idx),
                    children: vec![],
                    fullpath: child.path().to_str().unwrap().into(),
                    is_dir,
                    name: child.file_name().into_string().unwrap(),
                }
            })
            .collect::<Vec<Node>>();

        children_uninit.sort_by(|a, b| {
            if a.is_dir ^ b.is_dir {
                if a.is_dir {
                    return Ordering::Less;
                }
                return Ordering::Greater;
            }
            a.name.cmp(&b.name)
        });

        for (chidx, child) in children_uninit.iter_mut().enumerate() {
            let new_idx = self.arena.len() + chidx;
            self.arena[idx].children.push(new_idx);
            child.idx = new_idx;
        }

        self.arena.append(&mut children_uninit);
        Ok(())
    }
}
