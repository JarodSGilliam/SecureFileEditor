use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::*;
use crossterm::{event, terminal};

use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

pub mod file_io;
pub mod insertion_point;
pub mod key_handler;
pub mod language;
pub mod page;
pub mod screen;

use file_io::FileIO;
use page::*;
use screen::*;

//use device_query::{DeviceQuery, DeviceState, Keycode};

// Configurations
static AUTOSAVE: bool = false;
static AUTOSAVEEVERYNOPERATIONS: usize = 1000;

fn main() {

    let _hl_instance = (HighLight::Normal, HighLight::Number, HighLight::Search);

    // SETUP
    //introduce Tidy_Up instance so that raw mode is disabled at end of main
    let _tidy_up = TidyUp;
    let opened_file_path = FileIO::get_file_path(std::env::args());
    let mut extension: String = String::from("");
    let args: Vec<String> = std::env::args().collect(); //get command-line args
    let mut passed_arg: String = String::new();
    if args.len() >= 2 {
        //get passed file argument for saving purposes
        passed_arg = args[1].clone();
    }

    match opened_file_path.clone() {
        Some(string) => {
            extension = get_extension(string);
        }
        None => {}
    }

    let mut save_as_warned = false;

    // Setup
    match crossterm::terminal::enable_raw_mode() {
        Ok(_a) => {}
        Err(e) => eprint!("{}", e),
    };
    //Creates the screen on which everything is displayed
    let mut screen: Screen = Screen::new(opened_file_path.clone(), extension);
    // Counts the number of operations that have been executed since the last autosave or file opening
    let mut operations: usize = 0;
    // Creates a stack of screens
    // Creates the screen for interacting with the file
    screen.push(Page::new_with_contents(
        PageType::Text,
        FileIO::get_file_contents(&opened_file_path),
    ));
    screen.reset_prompt();

    let mut indices: Vec<usize>; // = Vec::new(); //list of indices where find text occurs
    let mut coordinates: Vec<(usize, usize)> = Vec::new(); //list of x,y pairs for the cursor after find
    let mut point = 0; //used to traverse found instances

    // render the context
    // PROGRAM RUNNING
    loop {
        // Displays the contents of the top screen
        match screen.refresh_screen() {
            Ok(_) => {}
            Err(e) => eprint!("{}", e),
        };

        // Watches for key commands
        if let Event::Key(event) =
            event::read().unwrap_or(Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)))
        {
            match event {
                //exit program
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => break,
                //save file
                KeyEvent {
                    code: KeyCode::Char('s'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    let pathname: String = String::from(match &opened_file_path {
                        Some(t) => t.as_str(),
                        None => "",
                    });
                    if !Path::new(pathname.as_str()).exists() && passed_arg.len() < 1 {
                        //empty cmd-line arg
                        trigger_saveas(&mut screen);
                    } else if !Path::new(pathname.as_str()).exists() {
                        //cmd-line arg refers to new file
                        screen.file_name = Some(passed_arg.clone());
                        screen.modified = false;
                        screen.reset_prompt();
                        let file_text: &String = &screen.active().contents;
                        match FileIO::overwrite_to_file(&passed_arg, file_text) {
                            Ok(_) => {}
                            Err(e) => eprint!("Failed to save because of error {}", e),
                        }
                    } else {
                        //else save as usual
                        // screen.active_mut().set_prompt(String::from("Saved!"));
                        let new_text: &String = &screen.active().contents;
                        match FileIO::overwrite_to_file(&pathname, new_text) {
                            Ok(_) => {}
                            Err(e) => eprint!("Failed to save because of error {}", e),
                        };
                        screen.modified = false;
                        if screen.find_mode() {
                            screen.active_mut().set_prompt(String::from(""));
                        }
                        screen.mode = Mode::Normal;
                        // break
                    }
                } //if-else for triggering SaveAs when user passes no cmd-line args

                //save file as [name]
                KeyEvent {
                    code: KeyCode::Char('s'),
                    modifiers: event::KeyModifiers::ALT,
                } => {
                    if screen.page_stack.len() == 1 {
                        screen.add(PageType::SaveAs);
                        screen.active_mut().set_prompt(String::from("Save As:"));
                    }

                    screen.mode = Mode::Normal;
                }

                KeyEvent {
                    //move to next occurrence
                    code: KeyCode::Right,
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if (screen.find_mode())
                        && coordinates.len() > 0
                        && (point < coordinates.len() - 1)
                    {
                        point += 1;
                        screen.key_handler.ip.x = coordinates[point].0;
                        screen.key_handler.ip.y = coordinates[point].1;
                    }
                }

                KeyEvent {
                    //move to previous occurrence
                    code: KeyCode::Left,
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if (screen.find_mode()) && coordinates.len() > 0 && (point > 0) {
                        point -= 1;
                        screen.key_handler.ip.x = coordinates[point].0;
                        screen.key_handler.ip.y = coordinates[point].1;
                    }
                }

                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    let pathname: String = String::from(match &opened_file_path {
                        Some(t) => t.as_str(),
                        None => "",
                    });
                    screen.add_info_page(String::from(FileIO::get_metadata(&pathname)));
                }

                KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    screen.add(PageType::Command);
                    screen.mode = Mode::Normal;
                }

                // Events that move the cursor
                KeyEvent {
                    code:
                        direction @ (KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Home
                        | KeyCode::End),
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    if screen.active().display_type != PageType::Info {
                        screen.move_ip(direction);
                    }
                },

                // Events that change the text
                KeyEvent {
                    code:
                        input
                        @ (KeyCode::Char(..) | KeyCode::Tab | KeyCode::Backspace | KeyCode::Delete),
                    modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
                } => {
                    if screen.active().display_type != PageType::Info {
                        screen.modified = true;
                        screen.insertion(input);
                    }
                },

                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    match screen.active().display_type {
                        PageType::Text => {
                            if screen.find_mode() {
                                screen.mode = Mode::Normal;
                                continue;
                            }
                            screen.insertion(KeyCode::Enter);
                            continue;
                        }
                        PageType::SaveAs => {
                            screen.mode = Mode::SaveAs(screen.active().contents.clone());
                            match screen.search_text() {
                                Some(string) => {
                                    if string.eq("") == false {
                                        let pathname = string.clone();
                                        let new_text: &String = &screen.text_page().contents;

                                        if !Path::new(pathname.as_str()).exists() | save_as_warned {
                                            //if the specified filename does not already exist
                                            match FileIO::overwrite_to_file(&pathname, new_text) {
                                                Ok(_) => {
                                                    screen.file_name = Some(pathname.clone());
                                                    screen.reset_prompt();
                                                    screen.modified = false;
                                                    screen.pop();
                                                    save_as_warned = false;
                                                    println!("{}", get_extension(pathname.clone()));
                                                    screen.color_struct = Screen::get_color_struct(
                                                        get_extension(pathname.clone()),
                                                    );
                                                    println!("{:?}", screen.color_struct.language);
                                                }
                                                Err(e) => eprint!(
                                                    "Failed to save as new file due to error {}",
                                                    e
                                                ),
                                            }
                                        } else {
                                            screen.active_mut().set_prompt(String::from("Warning: File Already Exists, Press Enter to Overwrite or choose new file name"));
                                            save_as_warned = true;
                                        }
                                    } else {
                                        screen.pop();
                                        save_as_warned = false;
                                    }
                                }

                                None => {
                                    //if user did not enter text to save the file under
                                    screen.pop();
                                    save_as_warned = false;
                                }
                            }
                        }

                        PageType::Command => {
                            screen.mode = Mode::Command(screen.active().contents.clone());
                            match screen.search_text() {
                                Some(string) => {
                                    let toggle = String::from("Toggle Highlight");
                                    let find = String::from("Find");
                                    let info = String::from("File Info");

                                    if string.to_lowercase().eq(&toggle.to_lowercase()) {
                                        //toggle
                                        screen.pop();
                                        screen.color_struct.toggle_status();
                                    } else if string.to_lowercase().eq(&find.to_lowercase()) {
                                        //find
                                        screen.pop();
                                        trigger_find(&mut screen);
                                    } else if string.to_lowercase().eq(&info.to_lowercase()) {
                                        //file info
                                        screen.pop();
                                        trigger_file_info(&mut screen, &opened_file_path);
                                    } else if string.to_lowercase().eq("save") || string.to_lowercase().eq("save as") {
                                        screen.pop();
                                        trigger_saveas(&mut screen);
                                    } else if string.to_lowercase().eq("replace") {
                                        screen.pop();
                                        trigger_replace(&mut screen);
                                    } else{
                                        screen.pop();
                                    }
                                }

                                None => {}
                            }
                        }

                        PageType::Find => {
                            screen.mode = Mode::Find(screen.active().contents.clone());

                            // screen.mode = Mode::Find(the_text_that_is_being_searched_for);
                            match screen.search_text() {
                                None => {
                                    screen.pop();
                                }
                                Some(str) => {
                                    if str.eq("") == false {
                                        let number_found = screen
                                            .text_page()
                                            .contents
                                            .matches(&screen.search_text().unwrap())
                                            .count();
                                        if number_found > 1 {
                                            screen.text_page_mut().set_prompt(format!(
                                            "Found {} matches: (Ctrl + Left for previous, Ctrl + Right for next, ESC to exit find mode)",
                                            number_found
                                        ));
                                        } else if number_found == 1 {
                                            screen.text_page_mut().set_prompt(format!(
                                                "Found 1 match: (ESC to exit find mode)",
                                            ));
                                        } else {
                                            screen.text_page_mut().set_prompt(format!(
                                            "Found no matches: (Try searching for something else, ESC to exit find mode)",
                                        ));
                                        }

                                        screen.pop();

                                        //Find & Move Cursor operation below

                                        indices = get_indices(
                                            &screen.text_page().contents,
                                            &screen.search_text().unwrap(),
                                            number_found,
                                        ); //list of indices where find text occurs
                                        coordinates =
                                            get_xs_and_ys(indices, &screen.active().contents); //list of (x, y) pairs for moving the cursor

                                        let (res1, res2) = find_text(
                                            screen.text_page(),
                                            &screen.search_text().unwrap(),
                                        );
                                        match res1 {
                                            Some(_t) => {
                                                //if res1 is not a None, then at least one occurrence was found
                                                screen.key_handler.ip.x = res1.unwrap();
                                                screen.key_handler.ip.y = res2.unwrap();
                                            }
                                            None => {}
                                        }
                                        //continue;
                                    } //if search text not empty
                                }
                            } //match if search text is empty or not
                        } //match PageType::Find
                        PageType::ReplaceP1 => {
                            screen.mode = Mode::Replace(match screen.page_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            });
                            screen.pop();
                            screen.add(PageType::ReplaceP2);
                            screen
                                .active_mut()
                                .set_prompt(String::from("Replace P2:\nReplace:"));
                            println!("{}", screen.search_text().unwrap());
                            if screen.find_mode() {
                                screen.reset_prompt();
                            }
                            continue;
                        }
                        PageType::ReplaceP2 => {
                            let to_replace = match screen.page_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            };
                            let temp007 = match &screen.mode {
                                Mode::Normal => break,
                                Mode::Find(_) => break,
                                Mode::Replace(t) => t,
                                Mode::SaveAs(_t) => break,
                                Mode::Command(_t) => break,
                            }
                            .clone();
                            screen.text_page_mut().contents = screen
                                .text_page_mut()
                                .contents
                                .replace(temp007.as_str(), to_replace.as_str());
                            screen.pop();
                            screen
                                .text_page_mut()
                                .set_prompt(String::from("Replaced here:"));
                            screen.mode = Mode::Replace(to_replace.clone());
                            if screen.find_mode() {
                                screen.reset_prompt();
                            }
                            continue;
                        }
                        _ => {}
                    }
                }

                KeyEvent {
                    code: KeyCode::Char('h'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if screen.active().display_type != PageType::Info {
                        screen.add_help_page();
                    }
                    if screen.find_mode() {
                        screen.reset_prompt();
                    }
                    screen.mode = Mode::Normal;
                }

                // Triggers find screen
                KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if screen.page_stack.len() == 1 {
                        // find_display
                        screen.add(PageType::Find);
                        screen.active_mut().set_prompt(String::from("Text to find:"));
                    }
                    if screen.find_mode() {
                        screen.reset_prompt();
                    }
                    screen.mode = Mode::Normal;
                }

                // Triggers replace screen
                KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if screen.page_stack.len() == 1 {
                        screen.add(PageType::ReplaceP1);
                        screen
                            .active_mut()
                            .set_prompt(String::from("Replace P1:\nFind:"));
                    }
                    if screen.find_mode() {
                        screen.reset_prompt();
                    }
                    screen.mode = Mode::Normal;
                }

                KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    if screen.page_stack.len() > 1 {
                        screen.pop();
                    } else {
                        if screen.find_mode() || screen.mode.to_str() == "replace" {
                            screen.reset_prompt();
                            screen.mode = Mode::Normal;
                            continue;
                        }
                        if screen.active().display_type != PageType::Info {
                            screen.add_help_page();
                        }
                    }
                }
                _ => {}
            }
        }

        // Autosave system so the user does not lose a lot of progress
        if operations <= AUTOSAVEEVERYNOPERATIONS {
            operations += 1;
        } else {
            operations = 0;
            if AUTOSAVE {
                FileIO::auto_save(&opened_file_path, &screen.text_page().contents);
            }
        }

        //render to user save question
    }
    // EXIT
}

/*
 *  This function is called when the user enters the Find command
 *  from the Command Line screen. It essentially does the same thing as the
 *  KeyCode::Char('f') code in main's match statement, just in function form
 *  so it can be easily called from the command line.
 */
fn trigger_find(scr: &mut Screen) {
    if scr.page_stack.len() == 1 {
        scr.add(PageType::Find);
        scr.active_mut().set_prompt(String::from("Text to Find"));
    }

    if scr.find_mode() {
        scr.reset_prompt();
    }
    scr.mode = Mode::Normal;
}

/*
 *  This function is similar to the trigger_find function. It is called when
 *  the user enters the File Info command from the Command Line screen. It essentially does
 *  the same thing as the KeyCode::Char('d') code in main's match statement, just in
 *  function form so it can be easily called from the command line.
 */
fn trigger_file_info(scr: &mut Screen, path: &Option<String>) {
    let pathname: String = String::from(match path {
        Some(t) => t.as_str(),
        None => "",
    });

    scr.add_info_page(String::from(FileIO::get_metadata(&pathname)));
}

/*
 *  This function is called from main if the user is trying to save a new file
 *  that does not yet exist in the system. Its functionality is similar to the
 *  KeyCode::Char('s') code in main's match statement.
 */

fn trigger_saveas(scr: &mut Screen) {
    if scr.page_stack.len() == 1 {
        scr.add(PageType::SaveAs);
        scr.active_mut().set_prompt(String::from("Save As"));
    }

    scr.modified = false;
    scr.mode = Mode::Normal;
}

/*
 *  This function is called when the user enters the Replace command
 *  from the Command Line screen. It essentially does the same thing as the
 *  KeyCode::Char('r') code in main's match statement, just in function form
 *  so it can be easily called from the command line.
 */
fn trigger_replace(screen: &mut Screen) {
    if screen.page_stack.len() == 1 {
        screen.add(PageType::ReplaceP1);
        screen
            .active_mut()
            .set_prompt(String::from("Replace P1:\nFind:"));
    }
    if screen.find_mode() {
        screen.reset_prompt();
    }
    screen.mode = Mode::Normal;
}

/*
    Struct for disabling raw mode on program exit (when instance is dropped)
*/
struct TidyUp;
impl Drop for TidyUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Unable to disable raw mode terminal");
        Screen::clear_screen().expect("Error");
    }
}

/*
    This is a test function to see if the Regex crate is really necessary.
    It uses the String::find function to see if the user's search text is
    present in the main Display's contents field. If so, it calls the
    get_newx_newy function to determine the new ip_x and ip_y values for
    moving the cursor. Otherwise, it will return a tuple of None's.
*/

fn find_text(disp: &Page, text: &String) -> (Option<usize>, Option<usize>) {
    //println!("{}", disp.contents);
    match disp.contents.find(text) {
        Some(t) => {
            //println!("{}", t);
            let (new_x, new_y) = get_newx_newy(&disp.contents, t);
            //println!("{}, {}", new_x, new_y);
            return (Some(new_x), Some(new_y));
        }
        None => {
            //println!("Not Found");
            return (None, None);
        }
    }
}

/*
    This function aims to find the new ip_x and ip_y values
    for the cursor after finding the user's search text.

    Algorithm is as follows:
        Using position as the total # of characters the cursor must advance
        to find the new location, we iterate over each line of the file contents,
        paying special attention to each line's length. After each line, we increment
        the y_val, which will become the new ip_y value. The line's length in relation to
        the position parameter determines if that line contains the new location or not.
        Once we find the line that does contain the new position, we iterate over that line's
        characters, adding up the total number of spaces traversed, until we reach an amount
        equal to the position parameter, at which point we update x_val, which will become the new
        ip_x value. Finally, we return a tuple containing both x_val and y_val.
*/

fn get_newx_newy(contents: &String, position: usize) -> (usize, usize) {
    let v: Vec<&str> = contents.split("\n").collect(); //collect lines of contents
    let mut x_val = 0;
    let mut y_val = 0;
    let mut total = 0;
    'outer: for line in &v {
        if (line.len()) + total < position {
            //if position not on this line
            //println!("len: {}", line.len());
            total = total + line.len() + 1;
            y_val += 1;
            //println!("total: {}", total);
        } else if (line.len()) + total == position {
            //if position at end of this line
            //println!("here");
            total = total + line.len();
            x_val = line.len();
        } else if (line.len() + total) > position {
            //if position somewhere in this line
            //println!("final len: {}", line.len());
            let mut i = 0;

            for c in line.chars() {
                //println!("iterating on {}", c);
                if (total + i) == position {
                    // let s=disp.row_contents.get(y_val).unwrap();
                    let t = line.unicode_truncate(line[..i].width());
                    x_val = t.1;
                    break 'outer;
                }
                i += c.len_utf8();
            }
        }
    }
    (x_val, y_val)
}

/*
    This function is intended to get all the indices of a piece of text that
    the user wants to find. It will return a vector of usizes, representing all the
    indices where the found text appears. This is very important for the find function
    to be able to move between the found instances.
*/

fn get_indices(contents: &String, text: &String, count: usize) -> Vec<usize> {
    let mut new_str = contents.clone();
    let mut res_vec = Vec::new();
    let mut c: char = '\0';
    while res_vec.len() < count {
        match new_str.find(text) {
            Some(t) => {
                //println!("new_str: {}", new_str);
                if res_vec.len() > 0 {
                    let temp = res_vec[res_vec.len() - 1];
                    res_vec.push(t + temp + c.len_utf8());
                    //println!("pushing_ {}", t + temp + 1);
                } else {
                    res_vec.push(t);
                    //println!("pushing: {}", t);
                }
                //res_vec.push(t);
                new_str = new_str.split_at(t).1.to_string();
                c = new_str.remove(0);
                //println!("new_str_: {}", new_str);
            }
            None => {}
        }
    }

    res_vec
}

/*
    This funciton is designed to build a list of tuples,
    each containing an (x, y) value that the user can
    traverse with ip_x and ip_y to move the cursor between
    instances of the found text. It relies on the get_newx_newy() function
    to accomplish this.

    It takes in a vector of usize (indices) and a String reference. It then iterates
    through each usize in the vector, calling the get_newx_newy function on the usize and
    the string ref, then pushes the returned tuple into a new vector. It returns this new vector
    at the end of execution.
*/

fn get_xs_and_ys(list: Vec<usize>, contents: &String) -> Vec<(usize, usize)> {
    let mut res_vec: Vec<(usize, usize)> = Vec::new();
    for entry in list {
        res_vec.push(get_newx_newy(contents, entry)); //get the x, y coordinates from an index
    }
    //println!("coordinates: {:?}", res_vec);
    res_vec
}

// render the tab
// #[derive(PartialEq)]
// struct RowContent {
//     row_content: String,
//     render: String,
//     highlight: Vec<HighLight>,
// }

// struct EachRowContent {
//     row_content_each: Vec<RowContent>,
// }

// impl RowContent {
//     fn new(row_content: String, render: String, highlight: Vec<HighLight>) -> Self {
//         Self {
//             row_content,
//             render,
//             highlight,
//         }
//     }
// }
// impl EachRowContent {
//     fn new() -> Self {
//         Self {
//             row_content_each: Vec::new(),
//         }
//     }

//     fn render_content(&self, row: &mut RowContent) {
//         let mut ip = 0;
//         let capacity = row
//             .row_content
//             .chars()
//             .fold(0, |acc, next| acc + if next == '\t' { 8 } else { 1 });
//         row.render = String::with_capacity(capacity);
//         row.row_content.chars().for_each(|c| {
//             ip += 1;
//             if c == '\t' {
//                 row.render.push(' ');
//                 while ip % 8 != 0 {
//                     row.render.push(' ');
//                     ip += 1
//                 }
//             } else {
//                 row.render.push(c);
//             }
//         });
//     }

//     pub fn make_render(&self, t: usize) -> &String {
//         &self.row_content_each[t].render
//     }
// }

// highlight the search result
// syntax highlight function
// done in highlighting trait

enum HighLight {
    Normal,
    Number,
    Search,
}



trait ColorContent {
    fn set_color(&self, highlight_type: &HighLight) -> Color;
    fn match_type(&self, page: &Page) -> HighLight;
}

/*
    For the purposes of syntax highlighting with the syntect crate, this function
    is meant to get the command-line argument (file) extension. We need this extension to
    build the correct syntax for that file type.
*/

fn get_extension(full_name: String) -> String {
    let tokens: Vec<&str> = full_name.split(".").collect();
    if tokens.len() != 2 {
        return "".to_string();
    } else {
        return tokens[1].to_string();
    }
}

#[macro_export]
macro_rules! highlight_struct {
    (
        struct $Name:ident;
    ) => {
        struct $Name;

        impl ColorContent for $Name {
            fn set_color(&self, highlight_type: &HighLight) -> Color {
                match highlight_type {
                    HighLight::Normal => Color::Reset,
                    HighLight::Search => Color::Blue,
                }
            }

            fn match_type(&self, page: &Page) -> HighLight {
                let row = page.row_contents;
                let chars = row.chars();
                for c in chars {
                    if c.is_digit(10) {
                        HighLight::Number;
                    } else {
                        HighLight::Normal;
                    }
                }
            }
        }
    };
}
