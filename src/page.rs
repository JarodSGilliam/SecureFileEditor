use crate::insertion_point::*;

#[derive(PartialEq)]
pub enum PageType {
    Text,
    Find,
    Info,
    ReplaceP1,
    ReplaceP2,
    Command,
    SaveAs,
}

impl PageType {
    pub fn overwrites(&self) -> bool {
        match self {
            PageType::Find => false,
            PageType::Command => false,
            PageType::ReplaceP1 => false,
            PageType::ReplaceP2 => false,
            PageType::SaveAs => false,
            _ => true,
        }
    }
}

/*
    Struct for displaying file contents to user
*/
pub struct Page {
    pub display_type: PageType,
    pub contents: String,
    pub row_contents: Vec<String>,
    pub prompt: String,
    pub active_cursor_location: Option<InsertionPoint>,
}
impl Page {
    pub fn new(display_type: PageType) -> Page {
        Page {
            display_type,
            contents: String::new(),
            row_contents: Vec::new(),
            prompt: String::new(),
            active_cursor_location: None,
        }
    }

    pub fn new_with_contents(display_type: PageType, contents: String) -> Page {
        Page {
            display_type,
            contents,
            row_contents: Vec::new(),
            prompt: String::new(),
            active_cursor_location: None,
        }
    }

    pub fn set_contents(&mut self, new_contents: String) {
        self.contents = new_contents;
    }

    pub fn set_prompt(&mut self, new_prompt: String) {
        self.prompt = new_prompt;
        if self.prompt != "" {
            self.prompt += "\n";
        }
    }
    // pub fn insert_content_here(&mut self, before_here : usize, new_string : String) {
    //     self.contents = format!("{}{}{}",&self.contents[..before_here],new_string,&self.contents[before_here..]);
    // }

    pub fn save_active_cursor_location(&mut self, ip: InsertionPoint) {
        self.active_cursor_location = Some(ip);
    }

    /*
    pub fn draw_info_bar(&mut self) {
        self.contents.push_str(&style::Attribute::Reverse.to_string());
    }
    */
}

// Potential additions to page
// pub pub fn clear_prompt(&mut self) {
// pub pub fn set_IP(&mut self, insertion_point : InsertionPoint) {
