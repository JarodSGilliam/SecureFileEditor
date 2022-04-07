use std::io::{Read, Write, ErrorKind, self};
use std::fs::{File, OpenOptions, self};
use chrono::{DateTime, Local};

// Deals with all the reading and writing to the file
pub struct FileIO;
impl FileIO {
    /* Read from the file */
    pub fn read_from_file(pathname: &String) -> Result<String, io::Error> {
        let mut data = String::new();
        File::open(pathname)?.read_to_string(&mut data)?;
        Ok(data)
    }

    pub fn read_from_file_object(mut file : &File) -> Result<String, io::Error> {
        let mut output = String::new();
        file.read_to_string(& mut output)?;
        Ok(output)
    }

    // Gets the file at the given location, returns None if it does not exist
    pub fn get_file(file_path : &String) -> Option<File> {
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

    pub fn create_file(file_path : &String) -> File {
        match File::create(file_path) {
            Ok(fc) => fc,
            Err(e) => panic!("Problem creating the file: {:?}", e),
        }
    }

    pub fn append_to_file(pathname : &String, new_text : &String) -> Result<bool, io::Error> {
        let mut file = OpenOptions::new().write(true).append(true).open(pathname).unwrap();
        write!(file, "{}", new_text)?;
        Ok(true)
    }

    pub fn overwrite_to_file(pathname : &String, new_text : &String) -> Result<bool, io::Error> {
        FileIO::create_file(pathname); // If applied to a file that exists it wipes the file contents
        FileIO::append_to_file(pathname, new_text)
    }

    pub fn auto_save(pathname : &Option<String>, current_state_of_text : &String) {
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

    pub fn get_auto_save_path(pathname : &String) -> String {
        format!("{}~", pathname)
    }
    
    pub fn delete_auto_save(pathname : &String) {
        FileIO::delete_file(&FileIO::get_auto_save_path(pathname));
    }

    pub fn check_for_auto_save(pathname : &String) -> bool{
        match FileIO::get_file(&FileIO::get_auto_save_path(pathname)) {
            Some(_f) => {
                true
            },
            None => {
                false
            }
        }
    }

    pub fn delete_file(pathname : &String) {
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

    pub fn get_metadata(file : File) -> String {
        let metadata = match file.metadata() {
            Err(e) => panic!("Could not get metadata from file: {}", e),
            Ok(f) => f,
        };
        
        let mut temp : DateTime<Local> = metadata.accessed().unwrap().into();
        let accessed : String = format!("{}", temp.format("%T on %m/%d/%Y"));
        temp = metadata.created().unwrap().into();
        let created : String = format!("{}", temp.format("%T on %m/%d/%Y"));
        temp = metadata.modified().unwrap().into();
        let modified : String = format!("{}", temp.format("%T on %m/%d/%Y"));
        let output : String = format!("Last accessed: {}\nCreated:       {}\nLast Modified: {}\nLength:        {} characters\nPermissions:   {}", accessed, created, modified, metadata.len(), if metadata.permissions().readonly() {"Read only"} else {"Writeable"});
        output
    }

    pub fn get_file_contents(path : &Option<String>) -> String {
        match path {
            Some(f) => {
                let test = FileIO::read_from_file(&f);
                match test {
                    Ok(f) => String::from(f),
                    Err(e) => {
                        eprintln!("{}", e);
                        panic!("ERROR");
                    }
                }
            },
            None => String::new()
        }
    }

    // If the user is working on a saved file, it will hold the path to the target file
    // If the user is working on an unsaved file, it will hold None
    pub fn get_file_path(args : std::env::Args) -> Option<String> {
        let inputs : Vec<String> = args.collect();
        if inputs.len() >= 2 {
            let file_path = &inputs[1];
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
                            FileIO::overwrite_to_file(
                                file_path,
                                &FileIO::read_from_file(&FileIO::get_auto_save_path(file_path))
                                    .unwrap(),
                            )
                            .unwrap();
                            FileIO::delete_auto_save(file_path);
                        }
                    }
                    Some(String::from(file_path))
                }
                None => None,
            }
        } else {
            None
        }
    }
}
