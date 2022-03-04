use std::io::{Read, stdout, Write, ErrorKind,Cursor};
use std::fs::{File, OpenOptions};
use std::{cmp, env, fs, io};
use std::error::Error;

use crossterm::{event, terminal, execute, cursor, queue};
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::terminal::ClearType;
use crossterm::style::Print;
use std::time::Duration; // for autosave

//use device_query::{DeviceQuery, DeviceState, Keycode};

static MINAUTOSAVESIZE : usize = 100;
static AUTOSAVEEVERYMINUTES : u64 = 1;

/*//UP-TO: part 2
// Part 3 will introduce more input handling as well as 
//  allowing the user to move the cursor around on the screen.

// The linked tutorial page explains each bit of code in detail well 
//  enough, but I have added some comments in the code to try
//  and explain some things not covered yet in class

use std::io;        //bring in io library to read usr input
use std::io::Read;  //trait that provides the read() function
use std::time::Duration; //for timeout

use crossterm::{event, terminal};    //crate containing function for raw mode
                            //crate also for interacting with the terminal
use crossterm::event::{Event, KeyCode, KeyEvent};

struct CleanUp;
impl Drop for CleanUp {     //implementation block
    fn drop(&mut self) {    //called when a CleanUp object goes out of scope
        terminal::disable_raw_mode().expect("Could not disable raw mode");
    }
}

fn main() -> crossterm::Result<()> {
    let _clean_up = CleanUp;
    terminal::enable_raw_mode()?; //? operator only for returning Option or Result

    loop {
        if event::poll(Duration::from_millis(5000)).expect("Error") {
            if let Event::Key(event) = event::read().expect("Failed to read line") {
                match event {   //event (enum) returned by event::read()
                    KeyEvent {
                      code: KeyCode::Char('q'),
                        modifiers: event::KeyModifiers::NONE,
                    } => break, //if event is a pressed key 'q' without modifiers, exit
                 _ => {
                     //todo
                 }
             }
             println!("{:?}\r", event);
          };
        } else {
            break;
        } //if-else for terminal timeout (break means exit the loop from user's inactivity)
    }
    
    Ok(())
} */


fn main() -> crossterm::Result<()>{
    // SETUP
    //introduce Tidy_Up instance so that raw mode is disabled at end of main
    let _tidy_up = TidyUp;
    
    // If the user is working on a saved file, it will hold the path to the target file
    // If the user is working on an unsaved file, it will hold None
    let opened_file : Option<String> = {
        let args: Vec<String> = env::args().collect();
        if args.len() >= 2 {
            let file_path = &args[1];
            match FileIO::get_file(file_path) {
                Some(_f) => {
                    if FileIO::check_for_auto_save(file_path) {
                        println!("Use autosave?");
                        // If the user uses the autosave, it replaces the current save with the auto save
                        // If the user does not use the autosave, it simply ignores the autosave
                        if true {
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

    let mut on_screen : Display = Display::new();
    match &opened_file {
        Some(f) => {
            let test = FileIO::read_from_file(&f);
            match test {
                Ok(f) => on_screen.set_contents(String::from(f)),
                Err(e) => {
                    eprintln!("{}", e);
                    panic!("ERROR");
                }
            }
        },
        None => on_screen.set_contents(String::new())
    }

    // let key_handler : KeyHandler = KeyHandler::new((100, 100));

    // println!("read:\n{}", on_screen.contents);
    
    crossterm::terminal::enable_raw_mode()?;
    println!("{}",on_screen.contents.len());
    
    let mut screen=Screen::new();
        //PROGRAM RUNNING
    loop {
        // DISPLAY TEXT (from on_screen.contents) HERE
        screen.refresh_screen(&on_screen)?;
        if let Event::Key(event) = event::read()?{
            match event {
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: event::KeyModifiers::CONTROL,
                } => break,

                KeyEvent{
                    code: direction@(KeyCode::Up| KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End),
                    modifiers:event::KeyModifiers::NONE,

                } =>screen.key_handler.move_ip(direction),

                KeyEvent{
                    code:input@(KeyCode::Char(..) | /* KeyCode::Tab | */ KeyCode::Enter | KeyCode::Backspace),
                    modifiers:event::KeyModifiers::NONE | event::KeyModifiers::SHIFT,
                }=>screen.key_handler.insertion(input,&mut on_screen),
                
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
        

        // // Append test
        // on_screen.insert_content_here(0, String::from("more text"));
        // match &opened_file {
        //     Some(_f) => {
        //         let worked : bool = FileIO::overwrite_to_file(&opened_file.unwrap(), &on_screen.contents).unwrap();
        //         if worked {
        //             println!("Write successful");
        //         } else {
        //             println!("Problem writing to the file");
        //         }
        //     },
        //     None => println!("No file selected, working off empty file"),
        // }
        // break

        //render to user save question
    }
    Ok(())
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

    fn auto_save(pathname : &Option<String>, updates_count : &usize, current_state_of_text : &String) {
        if *updates_count < MINAUTOSAVESIZE {
            println!("Not enough content to autosave.");
            return;
        }
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

    fn print_metadata(file : File) {
        let debug = true;
        let metadata = match file.metadata() {
            Err(e) => panic!("Could not get metadata from file: {}", e),
            Ok(f) => f,
        };
        if debug {
            print!("{:#?}", metadata);
        };
    }
}

/*
    Struct responsible for moving the user's (i)nsertion (p)oint while
    the program is running.
*/
struct KeyHandler {
    ip_x: usize,
    ip_y: usize,
    screen_cols: usize,
    screen_rows: usize,
    updates: usize,
}
impl KeyHandler {
    //create new KeyHandler with insertion point at origin (top-left corner)
    fn new(window_size: (usize, usize)) -> KeyHandler {
        KeyHandler {
            ip_x: 0,
            ip_y: 0,
            screen_cols: window_size.0,
            screen_rows: window_size.1,
            updates: 111,
        }
    }

    //move the insertion point based on user's keypress
    fn move_ip(&mut self, operation: KeyCode) {
        match operation {
            KeyCode::Up => {
                if self.ip_y < 1 {
                    self.ip_y = 0;
                } else {
                    self.ip_y -= 1;
                }
            },
            KeyCode::Down => {
                if self.ip_y != self.screen_rows - 1 {
                    self.ip_y += 1;
                }
            },
            KeyCode::Left => {
                if self.ip_x != 0 {
                    self.ip_x -= 1;
                } else {
                    self.ip_x=0;
                }
            },
            KeyCode::Right => {
                if self.ip_x != self.screen_cols - 1 {
                    self.ip_x += 1;
                } else{
                    self.ip_x = self.screen_cols;
                }
            },
            KeyCode::End => self.ip_x = self.screen_cols -1,
            KeyCode::Home => { 
                self.ip_x = 0;
                self.ip_y = 0;
            },
            _ => {} //more code needed
        }
    }

    fn insertion(&mut self, operation : KeyCode,on_screen: &mut Display) {
        match operation {
            KeyCode::Char(c) => {
                on_screen.contents.insert(self.get_current_location_in_string(on_screen), c);
                on_screen.num_char_in_row[self.ip_y]+=1;
                self.updates += 1;
                self.ip_x += 1;
                println!("{:?}",on_screen.num_char_in_row);
                println!("bleh: {}\r", c);
            },
            // KeyCode::Tab => {
            //     on_screen.contents.insert(self.get_current_location_in_string(on_screen), '\t');
            //     on_screen.num_char_in_row[self.ip_y]+=1;
            //     self.updates += 1;
            //     self.ip_x += 1;
            // },
            KeyCode::Backspace => {
                if self.ip_x==0{
                    if self.ip_y==0{
                        //do nothing
                    }
                    else{
                        on_screen.contents.remove(self.get_current_location_in_string(on_screen)-1);
                        self.ip_x=on_screen.num_char_in_row[self.ip_y-1]-1;
                        on_screen.num_char_in_row[self.ip_y-1]+=on_screen.num_char_in_row[self.ip_y]-1;
                        on_screen.num_char_in_row.remove(self.ip_y);
                        self.ip_y-=1;
                    }
                }
                else{
                    on_screen.contents.remove(self.get_current_location_in_string(on_screen)-1);
                    on_screen.num_char_in_row[self.ip_y]-=1;
                    self.ip_x -= 1;
                }
                self.updates += 1;
                println!("bleh: back\r");
            }
            // KeyCode::Delete => {
            //     self.updates += 1;
            //     println!("bleh: delete");
            // }
            KeyCode::Enter => {
                on_screen.contents.insert(self.get_current_location_in_string(on_screen), '\n');
                on_screen.num_char_in_row.insert(self.ip_y+1,on_screen.num_char_in_row[self.ip_y]-self.ip_x);
                on_screen.num_char_in_row[self.ip_y]=self.ip_x+1;
                self.updates += 1;
                self.ip_x = 0;
                self.ip_y += 1;
            },
            _ => {}
        }
    }

    //Backspace and moving forward when typing

    fn get_current_location_in_string(&mut self,on_screen: &Display) -> usize {
        // let x = self.ip_y*self.screen_cols + self.ip_x; //wrong
        let mut x=0;
        for i in 0..self.ip_y{
            x+=on_screen.num_char_in_row[i];
        }
        x+=self.ip_x;
        println!("{}",x);
        x
    }
}

/*
    Struct for displaying file contents to user
*/
struct Display {
    contents : String,
    num_char_in_row: Vec<usize>,
}
impl Display {
    fn new() -> Display {
        Display {
            contents: String::new(),
            num_char_in_row:vec![],
        }
    }
    
    fn set_contents(&mut self, new_contents : String) {
        self.contents = new_contents;
        if self.contents.len()==0{
            self.num_char_in_row.push(0);
        }
        else{
            let mut num=0;
            for i in self.contents.chars(){
                num+=1;
                if i=='\n'{
                    self.num_char_in_row.push(num);
                    num=0;
                }
            }
            if num !=0{
                self.num_char_in_row.push(num);
            }
        }
        println!("{:?}",self.num_char_in_row);
    }
    
    fn insert_content_here(&mut self, before_here : usize, new_string : String) {
        // let mut result = String::from("");
        // for a in self.contents[..before_here].chars() {
        //     result.push(a);
        // }
        // for a in new_string.chars() {
        //     result.push(a);
        // }
        // for a in self.contents[before_here..].chars() {
        //     result.push(a);
        // }
        // let x=&self.contents[..before_here];
        self.contents = format!("{}{}{}",&self.contents[..before_here],new_string,&self.contents[before_here..]);
    }
}

/*
Screen show the content to the screen
*/
struct Screen{
    // screen_size: (usize, usize),
    key_handler: KeyHandler,
}

impl Screen {
    fn new() -> Self {
        let screen_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap(); 
        Self { 
            // screen_size,
            key_handler:KeyHandler::new(screen_size),
         }
    }

    fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }

    fn draw_content(&self,on_screen: &Display) {
        // let screen_rows = self.screen_size.1;
        let mut temp = on_screen.contents.replace('\n', "\r\n");
        queue!(stdout(),Print(temp)).unwrap();
        // println!("text should be here"); 
    }

    fn refresh_screen(&self,on_screen: &Display) -> crossterm::Result<()> {
        let mut stdout=stdout();
        queue!(stdout, cursor::Hide, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?; 
        self.draw_content(on_screen);
        let ip_x = self.key_handler.ip_x;
        let ip_y = self.key_handler.ip_y;
        queue!(stdout, cursor::MoveTo(ip_x as u16,ip_y as u16 ),cursor::Show)?;
        stdout.flush()
    }

    fn move_cursor(&mut self,operation:KeyCode) {
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






