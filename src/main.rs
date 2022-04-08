use std::io::{stdout, BufRead, Write};
use std::{cmp, fs, io, thread, time};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::*;
use crossterm::terminal::ClearType;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::Result as CResult;
use crossterm::{cursor, event, execute, queue, style, terminal};

use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

pub mod file_io;
use file_io::FileIO;

//use device_query::{DeviceQuery, DeviceState, Keycode};

// Configurations
static AUTOSAVE: bool = false;
static AUTOSAVEEVERYNOPERATIONS: usize = 1000;

fn main() {
    // SETUP
    //introduce Tidy_Up instance so that raw mode is disabled at end of main
    let _tidy_up = TidyUp;
    let opened_file_path = FileIO::get_file_path(std::env::args());
    // Creates a stack of screens
    let mut screens_stack: Vec<Display> = Vec::new();
    // Creates the screen for interacting with the file
    screens_stack.push(Display::new_with_contents(
        DisplayType::Text,
        FileIO::get_file_contents(&opened_file_path),
    ));

    // Setup
    match crossterm::terminal::enable_raw_mode() {
        Ok(_a) => {}
        Err(e) => eprint!("{}", e),
    };
    //Creates the screen on which everything is displayed
    let mut screen: Screen = Screen::new();
    // Counts the number of operations that have been executed since the last autosave or file opening
    let mut operations: usize = 0;

    let mut the_text_that_is_being_searched_for = String::new();
    let mut find_mode: bool = true;

    let mut indices: Vec<usize> = Vec::new(); //list of indices where find text occurs
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
        match screen.refresh_screen(match screens_stack.last_mut() {
            Some(t) => t,
            None => break,
        }) {
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
                    screens_stack
                        .last_mut()
                        .unwrap()
                        .set_prompt(String::from("Saved!"));
                    let pathname: String = String::from(match &opened_file_path {
                        Some(t) => t.as_str(),
                        None => "",
                    });
                    let new_text: &String = match screens_stack.first() {
                        Some(t) => &t.contents,
                        None => break,
                    };
                    match FileIO::overwrite_to_file(&pathname, new_text) {
                        Ok(_) => {}
                        Err(e) => eprint!("Failed to save because of error {}", e),
                    };
                    if (find_mode) {
                        screens_stack
                            .first_mut()
                            .unwrap()
                            .set_prompt(String::from(""));
                    }
                    find_mode = false;
                    // break
                }

                KeyEvent {
                    //move to next occurrence
                    code: KeyCode::Char('n'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if (find_mode) && coordinates.len() > 0 && (point < coordinates.len() - 1) {
                        point += 1;
                        screen.key_handler.ip_x = coordinates[point].0;
                        screen.key_handler.ip_y = coordinates[point].1;
                    }
                }

                KeyEvent {
                    //move to previous occurrence
                    code: KeyCode::Char('p'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if (find_mode) && coordinates.len() > 0 && (point > 0) {
                        //println!("\nctrl p OK\n");
                        point -= 1;
                        screen.key_handler.ip_x = coordinates[point].0;
                        screen.key_handler.ip_y = coordinates[point].1;
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
                    Display::new_on_stack(&mut screen, &mut screens_stack, DisplayType::Help);
                    screens_stack
                        .last_mut()
                        .unwrap()
                        .set_contents(String::from(FileIO::get_metadata(&pathname)));
                    match screen.refresh_screen(match screens_stack.last_mut() {
                        Some(t) => t,
                        None => break,
                    }) {
                        Ok(_) => {}
                        Err(e) => eprint!("{}", e),
                    };
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
                } => screen
                    .key_handler
                    .move_ip(direction, screens_stack.last().unwrap()),

                // Events that change the text
                KeyEvent {
                    code:
                        input
                        @ (KeyCode::Char(..) | KeyCode::Tab | KeyCode::Backspace | KeyCode::Delete),
                    modifiers: event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
                } => screen.key_handler.insertion(
                    input,
                    match screens_stack.last_mut() {
                        Some(t) => t,
                        None => break,
                    },
                ),

                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    match screens_stack.last().unwrap().display_type {
                        DisplayType::Text => {
                            if the_text_that_is_being_searched_for != "" {
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
                                the_text_that_is_being_searched_for = String::new();
                                continue;
                            }
                            screen.key_handler.insertion(
                                KeyCode::Enter,
                                match screens_stack.last_mut() {
                                    Some(t) => t,
                                    None => break,
                                },
                            );
                            continue;
                        }
                        DisplayType::Find => {
                            the_text_that_is_being_searched_for = match screens_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            };
                            print!(
                                "\nThe text the user was looking for: {}",
                                the_text_that_is_being_searched_for
                            );
                            find_mode = true;
                            let number_found = screens_stack
                                .first()
                                .unwrap()
                                .contents
                                .matches(&the_text_that_is_being_searched_for)
                                .count();
                            if number_found > 0 {
                                screens_stack.first_mut().unwrap().set_prompt(format!(
                                    "Found {} matches: Ctrl + P for previous, Ctrl + N for next, ESC for exit find mode",
                                    number_found
                                ));
                            } else {
                                screens_stack.first_mut().unwrap().set_prompt(format!(
                                    "Found {} matches: Try searching for something else, ESC for exit find mode",
                                    number_found
                                ));
                            }
                            /* screens_stack.first_mut().unwrap().contents =    //fix here
                            screens_stack.first_mut().unwrap().contents.replace(
                                the_text_that_is_being_searched_for.as_str(),
                                format!("|{}|", the_text_that_is_being_searched_for.as_str())
                                    .as_str(),
                            ); */
                            screens_stack.pop();

                            //Find & Move Cursor operation below

                            indices = get_indices(
                                &screens_stack.first().unwrap().contents,
                                &the_text_that_is_being_searched_for,
                                number_found,
                            ); //list of indices where find text occurs
                            coordinates =
                                get_xs_and_ys(indices, &screens_stack.first().unwrap().contents); //list of (x, y) pairs for moving the cursor

                            let (res1, res2) = find_text(
                                screens_stack.first().unwrap(),
                                &the_text_that_is_being_searched_for,
                            );
                            match res1 {
                                Some(_t) => {
                                    //if res1 is not a None, then at least one occurrence was found
                                    screen.key_handler.ip_x = res1.unwrap();
                                    screen.key_handler.ip_y = res2.unwrap();
                                    let mut point = 0;
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
                                                        screen.key_handler.ip_x = coordinates[point].0;
                                                        screen.key_handler.ip_y = coordinates[point].1;
                                                    }
                                                },

                                                KeyEvent {      //user presed Ctrl+p, revert to previous instance
                                                    code: KeyCode::Char('p'),
                                                    modifiers: event::KeyModifiers::CONTROL,
                                                } => {
                                                    if point > 0 {
                                                        point -= 1;
                                                        screen.key_handler.ip_x = coordinates[point].0;
                                                        screen.key_handler.ip_y = coordinates[point].1;
                                                    }
                                                },

                                                _ => break     //all else, break the loop
                                            }
                                        }
                                    }   //end of loop */
                                }
                                None => {
                                    let cursor_location = match screens_stack.first_mut() {
                                        Some(t) => t.active_cursor_location,
                                        None => break,
                                    };
                                    screen.key_handler.ip_x = cursor_location.0;
                                    screen.key_handler.ip_y = cursor_location.1;
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
                        DisplayType::ReplaceP1 => {
                            the_text_that_is_being_searched_for = match screens_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            };
                            // screens_stack.first_mut().unwrap().contents = screens_stack.first_mut().unwrap().contents.replace(the_text_that_is_being_searched_for.as_str(), format!("|{}|", the_text_that_is_being_searched_for.as_str()).as_str());
                            screens_stack.pop();
                            Display::new_on_stack(
                                &mut screen,
                                &mut screens_stack,
                                DisplayType::ReplaceP2,
                            );
                            screens_stack
                                .last_mut()
                                .unwrap()
                                .set_prompt(String::from("Replace P2:\nReplace:"));
                            println!("{}", the_text_that_is_being_searched_for);
                            if (find_mode) {
                                screens_stack
                                    .first_mut()
                                    .unwrap()
                                    .set_prompt(String::from(""));
                            }
                            find_mode = false;
                            continue;
                        }
                        DisplayType::ReplaceP2 => {
                            let to_replace = match screens_stack.last() {
                                Some(t) => String::from(t.contents.as_str()),
                                None => String::new(),
                            };
                            screens_stack.first_mut().unwrap().contents =
                                screens_stack.first_mut().unwrap().contents.replace(
                                    the_text_that_is_being_searched_for.as_str(),
                                    to_replace.as_str(),
                                );
                            screens_stack.pop();
                            the_text_that_is_being_searched_for = String::from("");
                            // for
                            println!("{}", to_replace);
                            if (find_mode) {
                                screens_stack
                                    .first_mut()
                                    .unwrap()
                                    .set_prompt(String::from(""));
                            }
                            find_mode = false;
                            continue;
                        }
                        _ => {}
                    }
                }

                KeyEvent {
                    code: KeyCode::Char('h'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if screens_stack.last().unwrap().display_type != DisplayType::Help {
                        add_help_screen(&mut screen, &mut screens_stack);
                    }
                    if (find_mode) {
                        screens_stack
                            .first_mut()
                            .unwrap()
                            .set_prompt(String::from(""));
                    }
                    find_mode = false;
                }

                // Triggers find screen
                KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    // Hunter's version
                    // test_alt_screen();
                    // Jarod's Version

                    if screens_stack.len() == 1 {
                        // find_display
                        Display::new_on_stack(&mut screen, &mut screens_stack, DisplayType::Find);
                        screens_stack
                            .last_mut()
                            .unwrap()
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
                    if (find_mode) {
                        screens_stack
                            .first_mut()
                            .unwrap()
                            .set_prompt(String::from(""));
                    }
                    find_mode = false;
                }

                // Triggers find screen
                KeyEvent {
                    code: KeyCode::Char('r'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    if screens_stack.len() == 1 {
                        Display::new_on_stack(
                            &mut screen,
                            &mut screens_stack,
                            DisplayType::ReplaceP1,
                        );
                        screens_stack
                            .last_mut()
                            .unwrap()
                            .set_prompt(String::from("Replace P1:\nFind:"));
                    }
                    if find_mode {
                        screens_stack
                            .first_mut()
                            .unwrap()
                            .set_prompt(String::from(""));
                    }
                    find_mode = false;
                }

                KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    if screens_stack.len() > 1 {
                        screens_stack.pop();
                        let cursor_location = match screens_stack.first_mut() {
                            Some(t) => t.active_cursor_location,
                            None => break,
                        };
                        screen.key_handler.ip_x = cursor_location.0;
                        screen.key_handler.ip_y = cursor_location.1;
                    } else {
                        if find_mode {
                            screens_stack
                                .first_mut()
                                .unwrap()
                                .set_prompt(String::from(""));
                            find_mode = false;
                            continue;
                        }
                        if screens_stack.last().unwrap().display_type != DisplayType::Help {
                            add_help_screen(&mut screen, &mut screens_stack);
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
                FileIO::auto_save(
                    &opened_file_path,
                    match screens_stack.first() {
                        Some(t) => &t.contents,
                        None => break,
                    },
                );
            }
        }

        //render to user save question
    }
    // EXIT
}

fn add_help_screen(screen: &mut Screen, screens_stack: &mut Vec<Display>) {
    Display::new_on_stack(screen, screens_stack, DisplayType::Help);
    screens_stack
        .last_mut()
        .unwrap()
        .set_prompt(String::from("Help:"));
    let help_text: String =
        FileIO::read_from_file(&String::from("help.txt")).unwrap_or(String::from(
            "Help file not found. More on \"https://github.com/JarodSGilliam/SecureFileEditor\"",
        ));
    screens_stack.last_mut().unwrap().set_contents(help_text);
}

/*
    Struct responsible for moving the user's (i)nsertion (p)oint while
    the program is running.
*/
// ip_x, ip_y indicates the index of cursor and use the screen_cols and rows to store the screen size
struct KeyHandler {
    ip_x: usize,
    ip_y: usize,
    screen_cols: usize,
    screen_rows: usize,
    bytes_in_row: Vec<usize>,
    width_in_row: Vec<usize>,
    num_of_rows: usize,
    row_offset: usize,
    column_offset: usize,
}
impl KeyHandler {
    //create new KeyHandler with insertion point at origin (top-left corner)
    fn new(window_size: (usize, usize)) -> KeyHandler {
        KeyHandler {
            ip_x: 0,
            ip_y: 0,
            screen_cols: window_size.0,
            screen_rows: window_size.1,
            bytes_in_row: Vec::new(),
            width_in_row: Vec::new(),
            num_of_rows: 0,
            row_offset: 0,
            column_offset: 0,
        }
    }
    //check cursor position when scroll
    fn scroll(&mut self) {
        self.row_offset = cmp::min(self.row_offset, self.ip_y);
        if self.ip_y >= self.row_offset + self.screen_rows {
            self.row_offset = self.ip_y - self.screen_rows + 1;
        }
        self.column_offset = cmp::min(self.column_offset, self.ip_x);
        if self.ip_x >= self.column_offset + self.screen_cols {
            self.column_offset = self.ip_x - self.screen_cols + 1;
        }
    }

    //move the insertion point based on user's keypress
    fn move_ip(&mut self, operation: KeyCode, on_screen: &Display) {
        match operation {
            KeyCode::Up => {
                if self.ip_y > 0 {
                    self.ip_y -= 1;
                    self.ip_x = cmp::min(self.ip_x, *self.width_in_row.get(self.ip_y).unwrap());
                    self.ip_x = on_screen
                        .row_contents
                        .get(self.ip_y)
                        .unwrap()
                        .unicode_truncate(self.ip_x)
                        .1;
                }
            }
            KeyCode::Down => {
                if self.ip_y < self.num_of_rows - 1 {
                    self.ip_y += 1;
                    self.ip_x = cmp::min(self.ip_x, *self.width_in_row.get(self.ip_y).unwrap());
                    self.ip_x = on_screen
                        .row_contents
                        .get(self.ip_y)
                        .unwrap()
                        .unicode_truncate(self.ip_x)
                        .1;
                }
            }
            KeyCode::Left => {
                if self.ip_x > 0 {
                    self.ip_x -= 1;
                    let (_, mut w) = on_screen
                        .row_contents
                        .get(self.ip_y)
                        .unwrap()
                        .unicode_truncate(self.ip_x);
                    while w != self.ip_x {
                        self.ip_x -= 1;
                        w = on_screen
                            .row_contents
                            .get(self.ip_y)
                            .unwrap()
                            .unicode_truncate(self.ip_x)
                            .1;
                    }
                } else if self.ip_y > 0 {
                    self.ip_y -= 1;
                    self.ip_x = *self.width_in_row.get(self.ip_y).unwrap();
                }
            }
            KeyCode::Right => {
                if self.ip_x < *self.width_in_row.get(self.ip_y).unwrap() {
                    self.ip_x += 1;
                    let (_, mut w) = on_screen
                        .row_contents
                        .get(self.ip_y)
                        .unwrap()
                        .unicode_truncate(self.ip_x);
                    while w != self.ip_x {
                        self.ip_x += 1;
                        w = on_screen
                            .row_contents
                            .get(self.ip_y)
                            .unwrap()
                            .unicode_truncate(self.ip_x)
                            .1;
                    }
                } else if self.ip_y < self.num_of_rows - 1 {
                    self.ip_x = 0;
                    self.ip_y += 1;
                }
                // else is for default, the limit is set to be the screen size for further adjustment
            }
            KeyCode::End => self.ip_x = *self.width_in_row.get(self.ip_y).unwrap(),
            KeyCode::Home => self.ip_x = 0,
            _ => {} //more code needed
        }
    }

    fn insertion(&mut self, operation: KeyCode, on_screen: &mut Display) {
        match operation {
            KeyCode::Char(c) => {
                on_screen
                    .contents
                    .insert(self.get_current_location_in_string(on_screen), c);
                on_screen.row_contents = split_with_n(&on_screen.contents);
                self.bytes_in_row[self.ip_y] += c.to_string().len();
                self.ip_x += 1;
                let (_, mut w) = on_screen
                    .row_contents
                    .get(self.ip_y)
                    .unwrap()
                    .unicode_truncate(self.ip_x);
                while w != self.ip_x {
                    self.ip_x += 1;
                    w = on_screen
                        .row_contents
                        .get(self.ip_y)
                        .unwrap()
                        .unicode_truncate(self.ip_x)
                        .1;
                }
            }

            KeyCode::Tab => {
                //println!("tabbing");
                on_screen
                    .contents
                    .insert_str(self.get_current_location_in_string(on_screen), "    ");
                self.bytes_in_row[self.ip_y] += 4;
                self.ip_x += 4;
            }
            // KeyCode::Tab => {
            //     on_screen.contents.insert(self.get_current_location_in_string(on_screen), '\t');
            //     self.rows[self.ip_y]+=1;
            //     self.ip_x += 1;
            // },
            KeyCode::Backspace => {
                if self.ip_x == 0 {
                    if self.ip_y == 0 {
                        //do nothing since insertion point is at origin (top-left)
                    } else {
                        on_screen
                            .contents
                            .remove(self.get_current_location_in_string(on_screen) - 1);
                        on_screen.row_contents = split_with_n(&on_screen.contents);
                        self.ip_x = self.width_in_row[self.ip_y - 1];
                        self.bytes_in_row[self.ip_y - 1] =
                            on_screen.row_contents[self.ip_y - 1].len();
                        self.width_in_row[self.ip_y - 1] =
                            on_screen.row_contents[self.ip_y - 1].width();
                        self.bytes_in_row.remove(self.ip_y);
                        self.ip_y -= 1;
                    }
                } else {
                    let a = on_screen.contents[..self.get_current_location_in_string(on_screen)]
                        .to_string()
                        .pop()
                        .unwrap()
                        .len_utf8();
                    let deleted = on_screen
                        .contents
                        .remove(self.get_current_location_in_string(on_screen) - a);
                    self.bytes_in_row[self.ip_y] -= deleted.len_utf8();
                    self.ip_x -= 1;
                    let (_, mut w) = on_screen
                        .row_contents
                        .get(self.ip_y)
                        .unwrap()
                        .unicode_truncate(self.ip_x);
                    while w != self.ip_x {
                        self.ip_x -= 1;
                        w = on_screen
                            .row_contents
                            .get(self.ip_y)
                            .unwrap()
                            .unicode_truncate(self.ip_x)
                            .1;
                    }
                }
                //println!("bleh: back\r");
            }

            KeyCode::Delete => {
                if self.ip_x == self.bytes_in_row[self.ip_y] {
                    if self.ip_y == self.num_of_rows - 1 {
                        //do nothing since insertion point is at end of file (bottom-right)
                    } else {
                        //println!("{}", self.get_current_location_in_string());
                        on_screen
                            .contents
                            .remove(self.get_current_location_in_string(on_screen));
                        self.bytes_in_row[self.ip_y] += self.bytes_in_row[self.ip_y + 1] - 1;
                        self.bytes_in_row.remove(self.ip_y + 1);
                        self.num_of_rows -= 1;
                    }
                } else {
                    on_screen
                        .contents
                        .remove(self.get_current_location_in_string(on_screen));
                    self.bytes_in_row[self.ip_y] -= 1;
                }
            }

            KeyCode::Enter => {
                on_screen
                    .contents
                    .insert(self.get_current_location_in_string(on_screen), '\n');
                self.bytes_in_row
                    .insert(self.ip_y + 1, self.bytes_in_row[self.ip_y] - self.ip_x);
                self.bytes_in_row[self.ip_y] = self.ip_x + 1;
                self.ip_x = 0;
                self.ip_y += 1;
            }
            _ => {}
        }
    }

    //Backspace and moving forward when typing

    fn get_current_location_in_string(&mut self, on_screen: &Display) -> usize {
        let mut x = 0;
        for i in 0..self.ip_y {
            x += self.bytes_in_row[i];
        }
        let (s, _) = on_screen
            .row_contents
            .get(self.ip_y)
            .unwrap()
            .unicode_truncate(self.ip_x);
        x += s.replace('\n', "").len();
        //println!("{}",x);
        x
    }
}

#[derive(PartialEq)]
enum DisplayType {
    Text,
    Find,
    Help,
    ReplaceP1,
    ReplaceP2,
}

/*
    Struct for displaying file contents to user
*/
struct Display {
    display_type: DisplayType,
    contents: String,
    row_contents: Vec<String>,
    prompt: String,
    active_cursor_location: (usize, usize),
}
impl Display {
    fn new(display_type: DisplayType) -> Display {
        Display {
            display_type,
            contents: String::new(),
            row_contents: Vec::new(),
            prompt: String::new(),
            active_cursor_location: (0, 0),
        }
    }

    fn new_with_contents(display_type: DisplayType, contents: String) -> Display {
        Display {
            display_type,
            contents,
            row_contents: Vec::new(),
            prompt: String::new(),
            active_cursor_location: (0, 0),
        }
    }

    fn set_contents(&mut self, new_contents: String) {
        self.contents = new_contents;
    }

    fn set_prompt(&mut self, new_prompt: String) {
        self.prompt = new_prompt;
        if self.prompt != "" {
            self.prompt += "\n";
        }
    }
    // fn insert_content_here(&mut self, before_here : usize, new_string : String) {
    //     self.contents = format!("{}{}{}",&self.contents[..before_here],new_string,&self.contents[before_here..]);
    // }

    fn save_active_cursor_location(&mut self, keyhandler: &KeyHandler) {
        self.active_cursor_location = (keyhandler.ip_x, keyhandler.ip_y);
    }

    /*
    fn draw_info_bar(&mut self) {
        self.contents.push_str(&style::Attribute::Reverse.to_string());
    }
    */

    // Saves the location of the cursor on the screen, creates a new display, resets the cursor location to 0, 0.
    fn new_on_stack(
        screen: &mut Screen,
        screens_stack: &mut Vec<Display>,
        display_type: DisplayType,
    ) {
        match screens_stack.first_mut() {
            Some(t) => t.save_active_cursor_location(&screen.key_handler),
            None => return,
        }
        screen.key_handler.ip_x = 0;
        screen.key_handler.ip_y = 0;
        screens_stack.push(Display::new(display_type));
    }
}

/*
Screen show the content to the screen
*/
// fix the cursor in some special cases
struct Screen {
    key_handler: KeyHandler,
}
impl Screen {
    fn new() -> Self {
        let screen_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap();
        Self {
            key_handler: KeyHandler::new(screen_size),
        }
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }
    //print the char, and get the char of each row, get the total row number
    fn draw_content(&mut self, on_screen: &mut Display) {
        on_screen.row_contents = split_with_n(&on_screen.contents);
        // let calculator: Vec<&str> = on_screen.contents.split("\n").collect();
        self.key_handler.num_of_rows = on_screen.row_contents.len();
        let mut width: Vec<usize> = Vec::new();
        let mut bytes: Vec<usize> = Vec::new();
        for i in &on_screen.row_contents {
            bytes.push(i.len());
            width.push(i.width());
        }
        let mut content = String::new();
        for i in 0..self.key_handler.screen_rows {
            let row_in_content = i + self.key_handler.row_offset;
            if row_in_content < self.key_handler.num_of_rows {
                let mut offset_string = String::from("");
                let (len, start) = if width[row_in_content] <= self.key_handler.column_offset {
                    (0, 0)
                } else {
                    let (mut st, mut w) = on_screen
                        .row_contents
                        .get(row_in_content)
                        .unwrap()
                        .unicode_truncate(self.key_handler.column_offset);
                    while w != self.key_handler.column_offset + offset_string.len() {
                        offset_string.push_str(" ");
                        let temp_for_unicode = on_screen
                            .row_contents
                            .get(row_in_content)
                            .unwrap()
                            .unicode_truncate(self.key_handler.column_offset + offset_string.len());
                        st = temp_for_unicode.0;
                        w = temp_for_unicode.1;
                    }
                    if width[row_in_content] - w <= self.key_handler.screen_cols {
                        (
                            on_screen.row_contents.get(row_in_content).unwrap().len() - st.len(),
                            st.len(),
                        )
                    } else {
                        let (s_temp, _) = on_screen
                            .row_contents
                            .get(row_in_content)
                            .unwrap()
                            .unicode_truncate(
                                self.key_handler.column_offset + self.key_handler.screen_cols,
                            );
                        (s_temp.len() - st.len(), st.len())
                    }
                };
                // let len = cmp::min(
                //     bytes[row_in_content].saturating_sub(self.key_handler.column_offset),
                //     self.key_handler.screen_cols,
                // );
                // let start = if len == 0 {
                //     0
                // } else {
                //     self.key_handler.column_offset
                // };
                content.push_str(&offset_string);
                if i < self.key_handler.screen_rows - 1 {
                    if start + len == bytes[row_in_content] {
                        content.push_str(
                            &on_screen.row_contents[row_in_content].to_string()[start..start + len]
                                .replace('\n', "\r\n"),
                        );
                    } else {
                        content.push_str(
                            &on_screen.row_contents[row_in_content].to_string()[start..start + len],
                        );
                        content.push_str("\r\n");
                    };
                } else {
                    if start + len == bytes[row_in_content] {
                        content.push_str(
                            &on_screen.row_contents[row_in_content].to_string()[start..start + len]
                                .replace('\n', ""),
                        );
                    } else {
                        content.push_str(
                            &on_screen.row_contents[row_in_content].to_string()[start..start + len],
                        );
                    };
                }
                // use the position of search words to match display content and color it
                // for a in indices.iter(){
                //     queue!(stdout(),Print(content[1..2].red())).unwrap();

                //     content[*a..*a + text.len()].red();

                // }
            }
        }
        self.key_handler.bytes_in_row = bytes;
        self.key_handler.width_in_row = width;
        queue!(stdout(), Print(&on_screen.prompt.replace('\n', "\r\n"))).unwrap();
        queue!(stdout(), Print(content)).unwrap();
        // println!("{:?}", &on_screen.prompt);
    }

    /*
    fn draw_info_bar(&mut self, on_screen: &Display) {
        on_screen.contents.push_str(&style::Attribute::Reverse.to_string());
        (0..key_handler.)
    }
    */

    fn refresh_screen(&mut self, on_screen: &mut Display) -> crossterm::Result<()> {
        self.key_handler.scroll();
        let mut stdout = stdout();
        queue!(
            stdout,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        self.draw_content(on_screen);
        let ip_x = self.key_handler.ip_x - self.key_handler.column_offset;
        let mut ip_y = self.key_handler.ip_y - self.key_handler.row_offset;
        if on_screen.prompt != "" {
            ip_y += on_screen.prompt.matches("\n").count();
        }
        queue!(
            stdout,
            cursor::MoveTo(ip_x as u16, ip_y as u16),
            cursor::Show
        )?;
        stdout.flush()
    }

    // fn move_cursor(&mut self, operation:KeyCode) {
    //     self.key_handler.move_ip(operation);
    // }
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

fn find_text(disp: &Display, text: &String) -> (Option<usize>, Option<usize>) {
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
                    let t=line.unicode_truncate(line[..i].width());
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
    let mut c:char='\0';
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
                c=new_str.remove(0);
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

fn split_with_n(content: &String) -> Vec<String> {
    let mut result = Vec::new();
    let mut last = 0;
    for (index, _) in content.match_indices('\n') {
        // if last != index {
        result.push(content[last..index + 1].to_string());
        // }
        last = index + 1;
    }
    if last < content.len() {
        result.push(content[last..].to_string());
    }
    if result.len() == 0 {
        return vec![String::from("")];
    } else {
        return result;
    }
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
    Search,
}

trait ColorContent {
    fn set_color(&self, highlight_type: &HighLight) -> Color;
    // fn color_row(&self, render: &str, highlight: &[HighLight], temp:&mut String) {
    //     render.chars().enumerate().for_each(|(i, c)| {
    //         let _ = execute!(stdout(), SetForegroundColor(self.set_color(&highlight[i])));
    //         temp.push(c);
    //         let _ = queue!(stdout(),Print(temp),ResetColor);
    //     });
    // }
}

#[macro_export]
macro_rules! highlight_struct {
    (
        struct $Name:ident;
    ) => {
        struct $Name;

        impl ColorContent for $Name {
            fn set_color(&self, content_type: &HighLight) -> Color {
                match highlight_type {
                    HighLight::Normal => Color::Reset,
                    HighLight::Search => Color::Blue,
                }
            }
        }
    };
}
