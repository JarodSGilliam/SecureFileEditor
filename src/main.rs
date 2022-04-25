use std::io::{stdout, BufRead};
use std::path::Path;
use std::{io, thread, time};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::*;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::Result as CResult;
use crossterm::{event, execute, terminal};

use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

use syntect::parsing::SyntaxSet;

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
    // SETUP
    //introduce Tidy_Up instance so that raw mode is disabled at end of main
    let _tidy_up = TidyUp;
    let opened_file_path = FileIO::get_file_path(std::env::args());
    let mut extension: String = String::from("");
    match opened_file_path.clone() {
        Some(string) => {
            extension = get_extension(string);
        }
        None => {}
    }

    let mut save_as_warned = false;

    //println!("extension: {}", extension);
    //let mut builder = SyntaxSetBuilder::new();
    let s_set = SyntaxSet::load_defaults_newlines();
    let syntax = s_set
        .find_syntax_by_extension(extension.as_str())
        .unwrap_or_else(|| s_set.find_syntax_plain_text()); //load plaintext syntax if extension does not yield another valid syntax
    println!("{}", syntax.name);
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

    // let mut the_text_that_is_being_searched_for = String::new();

    let mut indices: Vec<usize>; // = Vec::new(); //list of indices where find text occurs
    let mut coordinates: Vec<(usize, usize)> = Vec::new(); //list of x,y pairs for the cursor after find
    let mut point = 0; //used to traverse found instances

    // render the context
    // let mut row_content = screens_stack.first().unwrap().contents.clone();
    // let mut eachrowcontent: EachRowContent = EachRowContent::new();
    // let mut rowcontent: RowContent = RowContent::new(row_content, String::new(), Vec::new());
    // let mut rendercontent = eachrowcontent.render_content(&mut rowcontent);

    // PROGRAM RUNNING
    loop {
        // Displays the contents of the top screen
        match screen.refresh_screen() {
            //the_text_that_is_being_searched_for.as_str()
            Ok(_) => {}
            Err(e) => eprint!("{}", e),
        };

        // new display function which can match the word it founded and color it
        // match screen.refresh_screen(match screens_stack.last() {
        //     Some(t) => t,
        //     None => break,
        // },& indices,& the_text_that_is_being_searched_for) {
        //     Ok(_) => {}
        //     Err(e) => eprint!("{}", e),
        // };

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
                    screen.active_mut().set_prompt(String::from("Saved!"));
                    let pathname: String = String::from(match &opened_file_path {
                        Some(t) => t.as_str(),
                        None => "",
                    });
                    let new_text: &String = &screen.active().contents;
                    match FileIO::overwrite_to_file(&pathname, new_text) {
                        Ok(_) => {}
                        Err(e) => eprint!("Failed to save because of error {}", e),
                    };
                    if screen.find_mode() {
                        screen.active_mut().set_prompt(String::from(""));
                    }
                    screen.mode = Mode::Normal;
                    // break
                }

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
                    code: KeyCode::Char(' '),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    screen.add(PageType::Command);
                    // screen.add_info_page(String::from(FileIO::get_metadata(&pathname)));
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
                } => screen.move_ip(direction),

                // Events that change the text
                KeyEvent {
                    code:
                        input
                        @ (KeyCode::Char(..) | KeyCode::Tab | KeyCode::Backspace | KeyCode::Delete),
                    modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
                } => screen.insertion(input),

                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    match screen.active().display_type {
                        PageType::Text => {
                            if screen.find_mode() {
                                //fix here
                                /* screens_stack.first_mut().unwrap().contents =
                                 screens_stack.first_mut().unwrap().contents.replace(
                                    format!(
                                        "|{}|",
                                        the_text_that_is_being_searched_for.as_str()
                                    )
                                    .as_str(),
                                    the_text_that_is_being_searched_for.as_str(),
                                ); */
                                screen.mode = Mode::Normal;
                                // the_text_that_is_being_searched_for = ;
                                continue;
                            }
                            screen.insertion(KeyCode::Enter);
                            continue;
                        }
                        PageType::SaveAs => {
                            screen.mode = Mode::SaveAs(screen.active().contents.clone());
                            match screen.search_text() {
                                Some(string) => {
                                    let pathname = string.clone();
                                    let new_text: &String = &screen.text_page().contents;
                                    //println!("new_text: {}", new_text);

                                    if !Path::new(pathname.as_str()).exists() | save_as_warned {
                                        //if the specified filename does not already exist
                                        match FileIO::overwrite_to_file(&pathname, new_text) {
                                            Ok(_) => {
                                                screen.file_name = Some(pathname.clone());
                                                screen.reset_prompt();
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
                                    let toggle_lower = String::from("toggle highlight");
                                    let find = String::from("Find");
                                    let find_lower = String::from("find");
                                    let info = String::from("File Info");
                                    let info_lower = String::from("file info");

                                    if (string.eq(&toggle)) | (string.eq(&toggle_lower)) {
                                        //toggle
                                        screen.pop();
                                        screen.color_struct.toggle_status();
                                    } else if (string.eq(&find)) | (string.eq(&find_lower)) {
                                        //find
                                        screen.pop();
                                        trigger_find(&mut screen);
                                    } else if (string.eq(&info)) | (string.eq(&info_lower)) {
                                        //file info
                                        screen.pop();
                                        trigger_file_info(&mut screen, &opened_file_path);
                                    } else {
                                        screen.pop();
                                    }
                                }

                                None => {}
                            }
                        }

                        PageType::Find => {
                            screen.mode = Mode::Find(screen.active().contents.clone());
                            print!(
                                "\nThe text the user was looking for: {}",
                                screen.search_text().unwrap()
                            );
                            // screen.mode = Mode::Find(the_text_that_is_being_searched_for);
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
                                screen
                                    .text_page_mut()
                                    .set_prompt(format!("Found 1 match: (ESC to exit find mode)",));
                            } else {
                                screen.text_page_mut().set_prompt(format!(
                                    "Found no matches: (Try searching for something else, ESC to exit find mode)",
                                ));
                            }
                            /* screens_stack.first_mut().unwrap().contents =    //fix here
                            screens_stack.first_mut().unwrap().contents.replace(
                                the_text_that_is_being_searched_for.as_str(),
                                format!("|{}|", the_text_that_is_being_searched_for.as_str())
                                    .as_str(),
                            ); */
                            screen.pop();

                            //Find & Move Cursor operation below

                            indices = get_indices(
                                &screen.text_page().contents,
                                &screen.search_text().unwrap(),
                                number_found,
                            ); //list of indices where find text occurs
                            coordinates = get_xs_and_ys(indices, &screen.active().contents); //list of (x, y) pairs for moving the cursor

                            let (res1, res2) =
                                find_text(screen.text_page(), &screen.search_text().unwrap());
                            match res1 {
                                Some(_t) => {
                                    //if res1 is not a None, then at least one occurrence was found
                                    screen.key_handler.ip.x = res1.unwrap();
                                    screen.key_handler.ip.y = res2.unwrap();
                                    // let mut point = 0;
                                    // highlight the searching results
                                    // (_t.._t + the_text_that_is_being_searched_for.len())
                                    // .for_each(|_t| rendercontent.highlight[_t] = HighLight::Search);
                                    /* loop {
                                        if let Event::Key(event) =
                                        event::read().unwrap_or(Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE))) {
                                            match event {
                                                KeyEvent {      //user pressed Ctrl+n, advance to next instance
                                                    code: KeyCode::Char('n'),
                                                    modifiers: event::KeyModifiers::CONTROL,
                                                } => {
                                                    if point < coordinates.len() - 1 {
                                                        point += 1;
                                                        screen.key_handler.ip.x = coordinates[point].0;
                                                        screen.key_handler.ip.y = coordinates[point].1;
                                                    }
                                                },

                                                KeyEvent {      //user presed Ctrl+p, revert to previous instance
                                                    code: KeyCode::Char('p'),
                                                    modifiers: event::KeyModifiers::CONTROL,
                                                } => {
                                                    if point > 0 {
                                                        point -= 1;
                                                        screen.key_handler.ip.x = coordinates[point].0;
                                                        screen.key_handler.ip.y = coordinates[point].1;
                                                    }
                                                },

                                                _ => break     //all else, break the loop
                                            }
                                        }
                                    }   //end of loop */
                                }
                                None => {
                                    // let cursor_location = screen.text_page_mut().active_cursor_location.as_ref().unwrap();
                                    // screen.key_handler.ip.x = cursor_location.x;
                                    // screen.key_handler.ip.y = cursor_location.y;
                                }
                            }
                            /*
                            let cursor_location = match screens_stack.first_mut() {
                                Some(t) => t.active_cursor_location,
                                None => {break},
                            };
                            screen.key_handler.ip_x = cursor_location.0;
                            screen.key_handler.ip_y = cursor_location.1;
                            */
                            //continue;
                        }
                        PageType::ReplaceP1 => {
                            screen.mode = Mode::Replace(match screen.page_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            });
                            // screens_stack.first_mut().unwrap().contents = screens_stack.first_mut().unwrap().contents.replace(the_text_that_is_being_searched_for.as_str(), format!("|{}|", the_text_that_is_being_searched_for.as_str()).as_str());
                            screen.pop();
                            screen.add(PageType::ReplaceP2);
                            screen
                                .active_mut()
                                .set_prompt(String::from("Replace P2:\nReplace:"));
                            println!("{}", screen.search_text().unwrap());
                            if screen.find_mode() {
                                // screen.text_page_mut().set_prompt(String::from(""));
                                screen.reset_prompt();
                            }
                            // screen.mode = Mode::Replace();
                            continue;
                        }
                        PageType::ReplaceP2 => {
                            let to_replace = match screen.page_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            };
                            // println!("{}", screen.mode.to_str());
                            let temp007 = match &screen.mode {
                                Mode::Normal => break,
                                Mode::Find(_) => break,
                                Mode::Replace(t) => t,
                                Mode::SaveAs(_t) => break,
                                Mode::Command(t) => break,
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
                            // for
                            println!("{}", to_replace);
                            if screen.find_mode() {
                                // screen.text_page_mut().set_prompt(String::from(""));
                                screen.reset_prompt();
                            }
                            // screen.mode = Mode::Normal;
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
                        // screen.text_page_mut().set_prompt(String::from(""));
                        screen.reset_prompt();
                    }
                    screen.mode = Mode::Normal;
                }

                // Triggers find screen
                KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    // Hunter's version
                    // test_alt_screen();
                    // Jarod's Version

                    if screen.page_stack.len() == 1 {
                        // find_display
                        screen.add(PageType::Find);
                        screen
                            .active_mut()
                            .set_prompt(String::from("Text to find:"));

                        /*
                            At this point we want to get the user's input for the text they'd like to find.
                            Using io::stdin doesn't seem to work, so we may need to use something else here.
                        */

                        /*if s.len() > 0 {
                            screens_stack.pop();
                            let cursor_location = match screens_stack.first_mut() {
                                Some(t) => t.active_cursor_location,
                                None => {break},
                            };
                            screen.key_handler.ip_x = cursor_location.0;
                            screen.key_handler.ip_y = cursor_location.1;
                            }
                        */
                    }
                    if screen.find_mode() {
                        // screen.text_page_mut().set_prompt(String::from(""));
                        screen.reset_prompt();
                    }
                    screen.mode = Mode::Normal;
                }

                // Triggers find screen
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
                        // screen.text_page_mut().set_prompt(String::from(""));
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
                        // let cursor_location = screen.text_page_mut().active_cursor_location.unwrap();
                        // screen.key_handler.ip_x = cursor_location.0;
                        // screen.key_handler.ip_y = cursor_location.1;
                    } else {
                        if screen.find_mode() || screen.mode.to_str() == "replace" {
                            // screen.text_page_mut().set_prompt(String::from(""));
                            screen.reset_prompt();
                            screen.mode = Mode::Normal;
                            continue;
                        }
                        if screen.active().display_type != PageType::Info {
                            screen.add_help_page();
                        }
                    }
                }
                // This part is to implement the function of keyboard interacting with the text file.
                // KeyEvent{
                //     code: roll,
                //     modifiers:event::KeyModifiers::NONE,

                // } => {
                //     match roll{
                //         KeyCode::PageDown | KeyCode::PageUp => screen.key_Handler.move_ip(roll),
                //         _ => ()
                //     }
                // },
                _ => {
                    //todo
                }
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
    Struct for disabling raw mode on program exit (when instance is dropped)
*/
struct TidyUp;
impl Drop for TidyUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Unable to disable raw mode terminal");
        Screen::clear_screen().expect("Error");
    }
}

//this funciton is meant to test out the AlternateScreen feature
//you can access this by pressing 'Esc' during execution
//the idea here is to allow the user to enter their find/replace text here,
// then move back to the main screen to 'find' it
fn test_alt_screen() -> CResult<()> {
    execute!(stdout(), EnterAlternateScreen)?; //move to alternate screen
    terminal::enable_raw_mode()?; //enable raw mode in alternate screen
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();

    println!("Text to find: ");
    handle.read_line(&mut buffer)?;
    thread::sleep(time::Duration::from_millis(1500));

    /* queue!(stdout(), Print("alt screen".to_string()));
    let mut s = String::new();
    io::stdin().read_line(&mut s).expect("failed to read input");
    thread::sleep(time::Duration::from_millis(1500)); */
    execute!(stdout(), LeaveAlternateScreen) //move back to main screen
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
struct RowContent {
    row_content: String,
    render: String,
    highlight: Vec<HighLight>,
}

struct EachRowContent {
    row_content_each: Vec<RowContent>,
}

impl RowContent {
    fn new(row_content: String, render: String, highlight: Vec<HighLight>) -> Self {
        Self {
            row_content,
            render,
            highlight,
        }
    }
}
impl EachRowContent {
    fn new() -> Self {
        Self {
            row_content_each: Vec::new(),
        }
    }

    fn render_content(&self, row: &mut RowContent) {
        let mut ip = 0;
        let capacity = row
            .row_content
            .chars()
            .fold(0, |acc, next| acc + if next == '\t' { 8 } else { 1 });
        row.render = String::with_capacity(capacity);
        row.row_content.chars().for_each(|c| {
            ip += 1;
            if c == '\t' {
                row.render.push(' ');
                while ip % 8 != 0 {
                    row.render.push(' ');
                    ip += 1
                }
            } else {
                row.render.push(c);
            }
        });
    }

    pub fn make_render(&self, t: usize) -> &String {
        &self.row_content_each[t].render
    }

    fn edit_row(&self, t: usize) -> &RowContent {
        &self.row_content_each[t]
    }
}

// highlight the search result
// syntax highlight function // not used in version2

enum HighLight {
    Normal,
    Number,
    Search,
}

trait ColorContent {
    fn set_color(&self, highlight_type: &HighLight) -> Color;
    fn match_type(&self, page: &Page) -> HighLight;
    // fn color_row(&self, render: &str, highlight: &[HighLight], temp:&mut String) {
    //     render.chars().enumerate().for_each(|(i, c)| {
    //         let _ = execute!(stdout(), SetForegroundColor(self.set_color(&highlight[i])));
    //         temp.push(c);
    //         let _ = queue!(stdout(),Print(temp),ResetColor);
    //     });
    // }
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
