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

mod nodes;
mod ui;

use std::{error::Error, io, time::Duration};

use crossterm::{
    cursor, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() -> Result<(), Box<dyn Error>> {
    //
    // TODO:
    // 1. need to refactor Location. is this really a good idea to have it?
    //
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::MoveTo(0, 0),)?;

    let tick_rate = Duration::from_millis(100);

    let state = ui::AppState::new_from_fs("./");
    let res = ui::run_app(state, &mut stdout, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
