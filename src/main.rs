use std::io::{Read, stdout, Write, ErrorKind,Cursor, BufRead};
use std::fs::{File, OpenOptions};
use std::{cmp, env, fs, io, thread, time};
use std::error::Error;

use crossterm::{event, terminal, execute, cursor, queue, style};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::ClearType;
use crossterm::style::Print;
use crossterm::Result as CResult;

use chrono::DateTime;
use chrono::offset::Local;

//use device_query::{DeviceQuery, DeviceState, Keycode};

// Configurations
static AUTOSAVE : bool = false;
static AUTOSAVEEVERYNOPERATIONS : usize = 1000;

fn main() {
    // SETUP
    //introduce Tidy_Up instance so that raw mode is disabled at end of main
    let _tidy_up = TidyUp;
    
    // If the user is working on a saved file, it will hold the path to the target file
    // If the user is working on an unsaved file, it will hold None
    let opened_file_path : Option<String> = {
        let args: Vec<String> = env::args().collect();
        if args.len() >= 2 {
            let file_path = &args[1];
            match FileIO::get_file(file_path) {
                Some(_f) => {
                    // If the user uses the autosave, it replaces the current save with the auto save
                    // If the user does not use the autosave, it simply ignores the autosave
                    if FileIO::check_for_auto_save(file_path) {
                        println!("Use autosave?");
                        let mut line = String::new();
                        std::io::stdin().read_line(&mut line).unwrap();
                        println!("{}", line.trim());
                        if line.trim().eq("y") || line.trim().eq("yes") {
                            FileIO::overwrite_to_file(file_path, &FileIO::read_from_file(&FileIO::get_auto_save_path(file_path)).unwrap()).unwrap();
                            FileIO::delete_auto_save(file_path);
                        }
                    }
                    Some(String::from(file_path))
                },
                None => None
            }
        } else {
            None
        }
    };

    // Creates a stack of screens
    let mut screens_stack : Vec<Display> = Vec::new();
    
    // Creates the screen for the file contents
    let mut file_screen : Display = Display::new(DisplayType::Text);
    match &opened_file_path {
        Some(f) => {
            let test = FileIO::read_from_file(&f);
            match test {
                Ok(f) => file_screen.set_contents(String::from(f)),
                Err(e) => {
                    eprintln!("{}", e);
                    panic!("ERROR");
                }
            }
        },
        None => file_screen.set_contents(String::new())
    }
    screens_stack.push(file_screen);

    // Setup
    match crossterm::terminal::enable_raw_mode() {
        Ok(_a) => {},
        Err(e) => eprint!("{}", e),
    };
    
    //Creates the screen on which everything is displayed
    let mut screen=Screen::new();
    
    // Counts the number of operations that have been executed since the last autosave or file opening
    let mut operations : usize = 0;

    let mut the_text_that_is_being_searched_for = String::new();

    let help_text = "Ctrl + h = help page \nCtrl + f = find\nCtrl + w = close file";

    // PROGRAM RUNNING
    loop {
        // Displays the contents of the top screen
        match screen.refresh_screen(match screens_stack.last() {
            Some(t) => t,
            None => {break},
        }) {
            Ok(_) => {},
            Err(e) => eprint!("{}", e),
        };

        if the_text_that_is_being_searched_for != "" {
            screens_stack.first_mut().unwrap().contents = screens_stack.first_mut().unwrap().contents.replace(format!("|{}|", the_text_that_is_being_searched_for.as_str()).as_str(), the_text_that_is_being_searched_for.as_str());
        }
        the_text_that_is_being_searched_for = String::new();
        
        // Watches for key commands
        if let Event::Key(event) = event::read().unwrap_or(Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE))) {
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
                    screens_stack.last_mut().unwrap().set_prompt(String::from("Saved!"));
                    let pathname : String = String::from(match &opened_file_path {
                        Some(t) => t.as_str(),
                        None => "",
                    });
                    let new_text : &String = match screens_stack.first() {
                        Some(t) => &t.contents,
                        None => {break},
                    };
                    match FileIO::overwrite_to_file(&pathname, new_text) {
                        Ok(_) => {},
                        Err(e) => eprint!("Failed to save because of error {}", e),
                    };
                    // break
                },

                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => {
                    let pathname : String = String::from(match &opened_file_path {
                        Some(t) => t.as_str(),
                        None => "",
                    });
                    
                    Display::new_on_stack(&mut screen, &mut screens_stack, DisplayType::Help);
                    screens_stack.last_mut().unwrap().set_contents(String::from(FileIO::print_metadata(FileIO::get_file(&pathname).unwrap())));
                    match screen.refresh_screen(match screens_stack.last() {
                        Some(t) => t,
                        None => {break},
                    }) {
                        Ok(_) => {},
                        Err(e) => eprint!("{}", e),
                    };
                },

                // Events that move the cursor
                KeyEvent{
                    code: direction@(KeyCode::Up| KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End),
                    modifiers:event::KeyModifiers::NONE,

                } =>screen.key_handler.move_ip(direction),

                // Events that change the text
                KeyEvent{
                    code:input@(KeyCode::Char(..) | KeyCode::Tab |  KeyCode::Backspace | KeyCode::Delete),
                    modifiers:event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
                }=>screen.key_handler.insertion(input,match screens_stack.last_mut() {
                    Some(t) => t,
                    None => {break},
                }),

                KeyEvent {
                    code:KeyCode::Enter,
                    modifiers:event::KeyModifiers::NONE,
                } => {
                    if screens_stack.len() == 1 {
                        screen.key_handler.insertion(KeyCode::Enter, match screens_stack.last_mut() {
                            Some(t) => t,
                            None => {break},
                        })
                    }
                    if screens_stack.len() > 1 {
                        the_text_that_is_being_searched_for = match screens_stack.last() {
                            Some(t) => String::from(t.contents.as_str()),
                            None => String::new(),
                        };
                        print!("\nThe text the user was looking for: {}", the_text_that_is_being_searched_for);
                        screens_stack.first_mut().unwrap().contents = screens_stack.first_mut().unwrap().contents.replace(the_text_that_is_being_searched_for.as_str(), format!("|{}|", the_text_that_is_being_searched_for.as_str()).as_str());
                        screens_stack.pop();
                        let cursor_location = match screens_stack.first_mut() {
                            Some(t) => t.active_cursor_location,
                            None => {break},
                        };
                        screen.key_handler.ip_x = cursor_location.0;
                        screen.key_handler.ip_y = cursor_location.1;
                    }
                },

                KeyEvent {
                    code:KeyCode::Char('h'),
                    modifiers:event::KeyModifiers::CONTROL,
                } => {
                    if screens_stack.last().unwrap().display_type != DisplayType::Help {
                        Display::new_on_stack(&mut screen, &mut screens_stack, DisplayType::Help);
                        screens_stack.last_mut().unwrap().set_prompt(String::from("Help:"));
                        screens_stack.last_mut().unwrap().set_contents(String::from(help_text));
                    }
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
                        screens_stack.last_mut().unwrap().set_prompt(String::from("Text to find:"));

                        
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
                },

                KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: event::KeyModifiers::NONE,
                } => {
                    if screens_stack.len() > 1 {
                        screens_stack.pop();
                        let cursor_location = match screens_stack.first_mut() {
                            Some(t) => t.active_cursor_location,
                            None => {break},
                        };
                        screen.key_handler.ip_x = cursor_location.0;
                        screen.key_handler.ip_y = cursor_location.1;
                    } else {
                        if screens_stack.last().unwrap().display_type != DisplayType::Help {
                            Display::new_on_stack(&mut screen, &mut screens_stack, DisplayType::Help);
                            screens_stack.last_mut().unwrap().set_prompt(String::from("Help:"));
                            screens_stack.last_mut().unwrap().set_contents(String::from(help_text));
                        }
                    }
                },
                
                
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
                FileIO::auto_save(&opened_file_path, match screens_stack.first() {
                    Some(t) => &t.contents,
                    None => {break},
                });
            }
        }

        //render to user save question
    }
        // EXIT
}

// Deals with all the reading and writing to the file
struct FileIO;
impl FileIO {
    /* Read from the file */
    fn read_from_file(pathname: &String) -> Result<String, io::Error> {
        let mut data = String::new();
        File::open(pathname)?.read_to_string(&mut data)?;
        Ok(data)
    }

    fn read_from_file_object(mut file : &File) -> Result<String, io::Error> {
        let mut output = String::new();
        file.read_to_string(& mut output)?;
        Ok(output)
    }

    // Gets the file at the given location, returns None if it does not exist
    fn get_file(file_path : &String) -> Option<File> {
        let f = File::open(file_path);
        match f {
            Ok(file) => Some(file),
            Err(error) => match error.kind() {
                ErrorKind::NotFound => None,
                other_error => {
                    panic!("Problem opening the file: {:?}", other_error)
                }
            }
        } 
    }

    fn create_file(file_path : &String) -> File {
        match File::create(file_path) {
            Ok(fc) => fc,
            Err(e) => panic!("Problem creating the file: {:?}", e),
        }
    }

    fn append_to_file(pathname : &String, new_text : &String) -> Result<bool, io::Error> {
        let mut file = OpenOptions::new().write(true).append(true).open(pathname).unwrap();
        write!(file, "{}", new_text)?;
        Ok(true)
    }

    fn overwrite_to_file(pathname : &String, new_text : &String) -> Result<bool, io::Error> {
        FileIO::create_file(pathname); // If applied to a file that exists it wipes the file contents
        FileIO::append_to_file(pathname, new_text)
    }

    fn auto_save(pathname : &Option<String>, current_state_of_text : &String) {
        println!("Autosaving...");
        let pathname : String = {
            match pathname {
                Some(s) => FileIO::get_auto_save_path(s),
                None => String::from(""),
            }
        };
        let result = FileIO::overwrite_to_file(&pathname, current_state_of_text);
        match result {
            Ok(_f) => {
                println!("Autosaved");
            },
            Err(e) => {
                eprintln!("There was an error autosaving: {}", e)
            },
        }
    }

    fn get_auto_save_path(pathname : &String) -> String {
        format!("{}~", pathname)
    }
    
    fn delete_auto_save(pathname : &String) {
        FileIO::delete_file(&FileIO::get_auto_save_path(pathname));
    }

    fn check_for_auto_save(pathname : &String) -> bool{
        match FileIO::get_file(&FileIO::get_auto_save_path(pathname)) {
            Some(_f) => {
                true
            },
            None => {
                false
            }
        }
    }

    fn delete_file(pathname : &String) {
        let result = fs::remove_file(pathname);
        match result {
            Ok(_f) => {
                println!("File deleted");
            }
            Err(e) => {
                eprintln!("Error deleting file: {}", e);
            }
        }
    }

    fn print_metadata(file : File) -> String {
        // let debug = true;
        let metadata = match file.metadata() {
            Err(e) => panic!("Could not get metadata from file: {}", e),
            Ok(f) => f,
        };
        
        let mut temp : DateTime<Local> = metadata.accessed().unwrap().into();
        let accessed : String = format!("{}", temp.format("%T on %d/%m/%Y"));
        temp = metadata.created().unwrap().into();
        let created : String = format!("{}", temp.format("%T on %d/%m/%Y"));
        temp = metadata.modified().unwrap().into();
        let modified : String = format!("{}", temp.format("%T on %d/%m/%Y"));
        // if debug {
        //     print!("{:#?}", metadata);
        // };
        let output : String = format!("Last accessed: {}\nCreated:       {}\nLast Modified: {}\nLength:        {} characters\nPermissions:   {}", accessed, created, modified, metadata.len(), if metadata.permissions().readonly() {"Read only"} else {"Writeable"});
        output
    }
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
    rows: Vec<usize>,
    columns: usize,
}
impl KeyHandler {
    //create new KeyHandler with insertion point at origin (top-left corner)
    fn new(window_size: (usize, usize)) -> KeyHandler {
        KeyHandler {
            ip_x: 0,
            ip_y: 0,
            screen_cols: window_size.0,
            screen_rows: window_size.1,
            rows: Vec::new(),   //number of chars in row
            columns: 0,
        }
    }

    //move the insertion point based on user's keypress
    fn move_ip(&mut self, operation: KeyCode) {
        match operation {
            KeyCode::Up => {
                if self.ip_y < 1 {
                    //scoll up
                } else {
                    self.ip_y -= 1;
                    let new_max_x = self.rows.get(self.ip_y).unwrap();
                    if new_max_x <= &self.ip_x {
                        self.ip_x = *new_max_x - 1;
                    }
                }
            },
            KeyCode::Down => {
                if self.ip_y < self.columns - 1 {
                    if self.ip_y != self.screen_rows - 1 {
                        self.ip_y += 1;
                        let new_max_x = self.rows.get(self.ip_y).unwrap();
                        if new_max_x <= &self.ip_x {
                            self.ip_x = new_max_x - 1;
                        }
                    }
                }
            },
            KeyCode::Left => {
                if self.ip_x != 0 {
                    self.ip_x -= 1;
                } else {
                    if self.ip_y > 0 {
                        self.ip_x=self.rows.get(self.ip_y-1).unwrap()-1;
                    }
                    KeyHandler::move_ip(self, KeyCode::Up);
                }
            },
            KeyCode::Right => {
                let this_row = match self.rows.get(self.ip_y) {
                    Some(i) => i,
                    None => panic!("error"),
                };
                if self.ip_x < this_row - 1 {
                    if self.ip_x != self.screen_cols - 1 {
                        self.ip_x += 1;
                    } else{
                        self.ip_x = self.screen_cols;
                    }
                } else if self.ip_y != self.columns - 1{
                    self.ip_x = 0;
                    KeyHandler::move_ip(self, KeyCode::Down);
                } // else is for default, the limit is set to be the screen size for further adjustment
            },
            KeyCode::End => self.ip_x = self.rows[self.ip_y] -1,
            KeyCode::Home => { 
                self.ip_x = 0;
            },
            _ => {} //more code needed
        }
    }

    fn insertion(&mut self, operation : KeyCode, on_screen: &mut Display) {
        match operation {
            KeyCode::Char(c) => {
                on_screen.contents.insert(self.get_current_location_in_string(), c);
                self.rows[self.ip_y]+=1;
                self.ip_x += 1;
                //println!("{:?}",self.rows);
                //println!("bleh: {}\r", c);
            },

            KeyCode::Tab => {
                //println!("tabbing");
                on_screen.contents.insert_str(self.get_current_location_in_string(), "    ");
                self.rows[self.ip_y] += 4;
                self.ip_x += 4;
            },
            // KeyCode::Tab => {
            //     on_screen.contents.insert(self.get_current_location_in_string(on_screen), '\t');
            //     self.rows[self.ip_y]+=1;
            //     self.ip_x += 1;
            // },
            KeyCode::Backspace => {
                if self.ip_x==0{
                    if self.ip_y==0{
                        //do nothing since insertion point is at origin (top-left)
                    }
                    else {
                        let deleted : char = on_screen.contents.remove(self.get_current_location_in_string()-1);
                        self.ip_x=self.rows[self.ip_y-1]-1;
                        self.rows[self.ip_y-1]+=self.rows[self.ip_y]-1;
                        self.rows.remove(self.ip_y);
                        self.ip_y-=1;
                    }
                }
                else{
                    on_screen.contents.remove(self.get_current_location_in_string()-1);
                    self.rows[self.ip_y]-=1;
                    self.ip_x -= 1;
                }
                //println!("bleh: back\r");
            },

            KeyCode::Delete => {
                 if self.ip_x == self.rows[self.ip_y] {
                    if self.ip_y == self.columns-1 {
                        //do nothing since insertion point is at end of file (bottom-right)
                    } 
                    else {
                        //println!("{}", self.get_current_location_in_string());
                        on_screen.contents.remove(self.get_current_location_in_string());
                        self.rows[self.ip_y] += self.rows[self.ip_y+1] - 1;
                        self.rows.remove(self.ip_y+1);
                        self.columns -= 1;
                    }
                } 
                else {
                    on_screen.contents.remove(self.get_current_location_in_string());
                    self.rows[self.ip_y] -=1;
                }
            },

            KeyCode::Enter => {
                on_screen.contents.insert(self.get_current_location_in_string(), '\n');
                self.rows.insert(self.ip_y+1,self.rows[self.ip_y]-self.ip_x);
                self.rows[self.ip_y]=self.ip_x+1;
                self.ip_x = 0;
                self.ip_y += 1;
            },
            _ => {}
        }
    }

    //Backspace and moving forward when typing

    fn get_current_location_in_string(&mut self) -> usize {
        let mut x=0;
        for i in 0..self.ip_y{
            x+=self.rows[i];
        }
        x+=self.ip_x;
        //println!("{}",x);
        x
    }
}

#[derive(PartialEq)]
enum DisplayType {
    Text,
    Find,
    Help,
}

/*
    Struct for displaying file contents to user
*/
struct Display {
    display_type : DisplayType,
    contents : String,
    prompt : String,
    active_cursor_location : (usize, usize),
}
impl Display {
    fn new(display_type : DisplayType) -> Display {
        Display {
            display_type : display_type,
            contents: String::new(),
            prompt : String::new(),
            active_cursor_location : (0, 0),
        }
    }
    
    fn set_contents(&mut self, new_contents : String) {
        self.contents = new_contents;
    }

    fn set_prompt(&mut self, new_prompt : String) {
        self.prompt = new_prompt + "\n";
    }
    
    fn insert_content_here(&mut self, before_here : usize, new_string : String) {
        self.contents = format!("{}{}{}",&self.contents[..before_here],new_string,&self.contents[before_here..]);
    }

    fn save_active_cursor_location(&mut self, keyhandler : &KeyHandler) {
        self.active_cursor_location = (keyhandler.ip_x, keyhandler.ip_y);
    }

    /*
    fn draw_info_bar(&mut self) {
        self.contents.push_str(&style::Attribute::Reverse.to_string());
    }
    */

    fn new_on_stack(screen : &mut Screen, screens_stack : &mut Vec<Display>, display_type : DisplayType) {
        match screens_stack.first_mut() {
            Some(t) => t.save_active_cursor_location(&screen.key_handler),
            None => {return},
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
struct Screen{
    key_handler: KeyHandler,
}
impl Screen {
    fn new() -> Self {
        let screen_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap(); 
        Self { 
            key_handler:KeyHandler::new(screen_size),
         }
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }
    //print the char, and get the char of each row, get the total row number
    fn draw_content(&mut self, on_screen: &Display) {
        // let screen_rows = self.screen_size.1;
        let mut content = on_screen.contents.replace('\n', "\r\n"); //.replace("\t", "    ")
        let temp2 = on_screen.contents.clone();
        let calculator : Vec<&str> = temp2.split("\n").collect();
        self.key_handler.columns = calculator.len();
        let mut rows : Vec<usize> = Vec::new();
        for i in calculator {
            rows.push(i.len() + 1);
        }
        self.key_handler.rows = rows;
        // content += format!("{}", content.len()).as_str();
        queue!(stdout(),Print(&on_screen.prompt.replace('\n', "\r\n"))).unwrap();
        queue!(stdout(),Print(content)).unwrap();
        // println!("text should be here"); 
    }

    /*
    fn draw_info_bar(&mut self, on_screen: &Display) {
        on_screen.contents.push_str(&style::Attribute::Reverse.to_string());
        (0..key_handler.)
    }
    */

    fn refresh_screen(&mut self,on_screen: &Display) -> crossterm::Result<()> {
        let mut stdout=stdout();
        queue!(stdout, cursor::Hide, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?; 
        self.draw_content(on_screen);
        let ip_x = self.key_handler.ip_x;
        let mut ip_y = self.key_handler.ip_y;
        if on_screen.prompt != "" {
            ip_y += on_screen.prompt.matches("\n").count();
        }
        queue!(stdout, cursor::MoveTo(ip_x as u16,ip_y as u16 ),cursor::Show)?;
        stdout.flush()
    }

    fn move_cursor(&mut self, operation:KeyCode) {
        self.key_handler.move_ip(operation);
    }
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
    execute!(stdout(), EnterAlternateScreen)?;  //move to alternate screen
    terminal::enable_raw_mode()?;               //enable raw mode in alternate screen
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
    execute!(stdout(), LeaveAlternateScreen)    //move back to main screen
}