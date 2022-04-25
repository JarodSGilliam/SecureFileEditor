use chrono::{DateTime, Local};
<<<<<<< HEAD
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
=======
use crate::language::Language;
>>>>>>> 8eef899c5d773f4fcdafbe5493dcfd91cb8d8a10

// Deals with all the reading and writing to the file
pub struct FileIO;
impl FileIO {
    /* Read from the file */
    pub fn read_from_file(pathname: &String) -> Result<String, io::Error> {
        let mut data = String::new();
        File::open(pathname)?.read_to_string(&mut data)?;
        Ok(data)
    }

    pub fn read_from_file_object(mut file: &File) -> Result<String, io::Error> {
        let mut output = String::new();
        file.read_to_string(&mut output)?;
        Ok(output)
    }

    // Gets the file at the given location, returns None if it does not exist
    pub fn get_file(file_path: &String) -> Option<File> {
        let f = File::open(file_path);
        match f {
            Ok(file) => Some(file),
            Err(error) => match error.kind() {
                ErrorKind::NotFound => None,
                other_error => {
                    panic!("Problem opening the file: {:?}", other_error)
                }
            },
        }
    }

    pub fn create_file(file_path: &String) -> File {
        match File::create(file_path) {
            Ok(fc) => fc,
            Err(e) => panic!("Problem creating the file: {:?}", e),
        }
    }

    pub fn append_to_file(pathname: &String, new_text: &String) -> Result<bool, io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(pathname)
            .unwrap();
        write!(file, "{}", new_text)?;
        Ok(true)
    }

    pub fn overwrite_to_file(pathname: &String, new_text: &String) -> Result<bool, io::Error> {
        FileIO::create_file(pathname); // If applied to a file that exists it wipes the file contents
        FileIO::append_to_file(pathname, new_text)
    }

    pub fn auto_save(pathname: &Option<String>, current_state_of_text: &String) {
        println!("Autosaving...");
        let pathname: String = {
            match pathname {
                Some(s) => FileIO::get_auto_save_path(s),
                None => String::from(""),
            }
        };
        let result = FileIO::overwrite_to_file(&pathname, current_state_of_text);
        match result {
            Ok(_f) => {
                println!("Autosaved");
            }
            Err(e) => {
                eprintln!("There was an error autosaving: {}", e)
            }
        }
    }

    pub fn get_auto_save_path(pathname: &String) -> String {
        format!("{}~", pathname)
    }

    pub fn delete_auto_save(pathname: &String) {
        FileIO::delete_file(&FileIO::get_auto_save_path(pathname));
    }

    pub fn check_for_auto_save(pathname: &String) -> bool {
        match FileIO::get_file(&FileIO::get_auto_save_path(pathname)) {
            Some(_f) => true,
            None => false,
        }
    }

    pub fn delete_file(pathname: &String) {
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

    pub fn get_metadata(pathname: &String) -> String {
        let file = FileIO::get_file(&pathname).unwrap();
        let metadata = match file.metadata() {
            Err(e) => panic!("Could not get metadata from file: {}", e),
            Ok(f) => f,
        };

        let mut temp: DateTime<Local> = metadata.accessed().unwrap().into();
        let accessed: String = format!("{}", temp.format("%T on %m/%d/%Y"));
        temp = metadata.created().unwrap().into();
        let created: String = format!("{}", temp.format("%T on %m/%d/%Y"));
        temp = metadata.modified().unwrap().into();
        let modified: String = format!("{}", temp.format("%T on %m/%d/%Y"));
        let mut file_text = String::new();
        let mut file_type = String::new();
        for a in pathname.chars() {
            if a == '.' {
                file_text += file_type.as_str();
                file_type = String::new();
            }
            file_type += format!("{}", a).as_str();
            // print!("{} ", a);
        }

        println!("{}", file_text);
        println!("{}", file_type);

        let output : String = format!(
            "File name: {}\nFile type: {}\nLast accessed: {}\nCreated:       {}\nLast Modified: {}\nLength:        {} characters\nPermissions:   {}",
            // pathname.chars()[0..pathname.chars().find('.')], 
            file_text, file_type, accessed, created, modified, metadata.len(), if metadata.permissions().readonly() {"Read only"} else {"Writeable"}
        );
        output
    }

    pub fn get_file_contents(path: &Option<String>) -> String {
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
            }
            None => String::new(),
        }
    }

    // If the user is working on a saved file, it will hold the path to the target file
    // If the user is working on an unsaved file, it will hold None
    pub fn get_file_path(args: std::env::Args) -> Option<String> {
        let inputs: Vec<String> = args.collect();
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

<<<<<<< HEAD
    pub fn get_highlights(
        file_type: String,
    ) -> Option<(Vec<String>, Vec<String>, Vec<String>, Vec<String>)> {
=======
    pub fn get_highlights(file_type : String) -> Option<Language> {
>>>>>>> 8eef899c5d773f4fcdafbe5493dcfd91cb8d8a10
        if file_type == "" {
            return None;
        }
        let lines : Vec<String> = FileIO::get_file_contents(&Some(String::from("highlighting.txt"))).split("\n").map(|x| x.trim().to_owned()).collect();
        if lines.len() == 0 {
            return None;
        }
<<<<<<< HEAD
        let lines: Vec<&str> = info.split("\n").collect();
        for i in 0..((lines.len() + 1) / 10) {
            // println!("{}", lines[i*10]);
            let b = i * 10;
            for a in lines[b].split(",") {
                if a.trim() == file_type {
                    let red: Vec<String> = lines[b + 2]
                        .split(",")
                        .map(|x| String::from(x.trim()))
                        .collect();
                    let blue: Vec<String> = lines[b + 4]
                        .split(",")
                        .map(|x| String::from(x.trim()))
                        .collect();
                    let green: Vec<String> = lines[b + 6]
                        .split(",")
                        .map(|x| String::from(x.trim()))
                        .collect();
                    let yellow: Vec<String> = lines[b + 8]
                        .split(",")
                        .map(|x| String::from(x.trim()))
                        .collect();
                    return Some((red, blue, green, yellow));
=======
        for i in 0..lines.len() {
            if match lines[i].split_once(" "){Some(t) => t, None => ("","")}.0 != "!" {
                continue;
            }
            for a in lines[i].split(" ").map(|x| x.trim()) {
                if a.trim() == file_type {
                    let mut related : String = String::new();
                    let mut n = i+1;
                    while n < lines.len() && match lines[n].split_once(" "){Some(s) => s, None => ("","")}.0 != "!" {
                        related += &lines[n];
                        related += "\n";
                        n += 1;
                    }
                    return Some(Language::new(related));
>>>>>>> 8eef899c5d773f4fcdafbe5493dcfd91cb8d8a10
                }
            }
        }
        None
    }
}
