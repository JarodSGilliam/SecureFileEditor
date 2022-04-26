use crate::file_io::FileIO;
use crate::insertion_point::*;
use crate::key_handler::*;
use crate::language::Language;
use crate::page::*;
use crossterm::event::KeyCode;
use crossterm::style::*;
use crossterm::terminal::ClearType;
use crossterm::{cursor, execute, queue, terminal};
use std::io::{stdout, Write};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

use crossterm::{
    style::{self, Stylize},
    QueueableCommand,
}; //ExecutableCommand, Result

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Find(String),
    Replace(String),
    SaveAs(String),
    Command(String),
}

impl Mode {
    pub fn to_str(&self) -> &str {
        match self {
            Mode::Normal => "normal",
            Mode::Find(_) => "find",
            Mode::Replace(_) => "replace",
            Mode::SaveAs(_) => "saveas",
            Mode::Command(_) => "command",
        }
    }
}

/*
Screen show the content to the screen
*/
// fix the cursor in some special cases
pub struct Screen {
    pub page_stack: Vec<Page>,
    pub key_handler: KeyHandler,
    pub mode: Mode,
    pub file_name: Option<String>,
    pub modified: bool,
    pub color_struct: ColorWord,
}
impl Screen {
    pub fn new(file_name: Option<String>, extension: String) -> Self {
        let screen_size = terminal::size()
            .map(|(x, y)| (x as usize, y as usize))
            .unwrap();
        Self {
            page_stack: Vec::new(),
            key_handler: KeyHandler::new(screen_size),
            mode: Mode::Normal,
            file_name,
            modified: false,
            color_struct: Screen::get_color_struct(extension),
        }
    }

    pub fn get_color_struct(extension: String) -> ColorWord {
        let language = FileIO::get_highlights(extension.to_owned());
        let color = ColorWord::new(
            None,
            match language {
                Some(l) => l,
                None => Language::new("".to_owned()),
            },
        );
        color
    }

    pub fn find_mode(&self) -> bool {
        self.mode.to_str() == "find"
    }

    pub fn search_text(&self) -> Option<String> {
        match &self.mode {
            Mode::Normal => None,
            Mode::Find(t) => Some(t.clone()),
            Mode::Replace(t) => Some(t.clone()),
            Mode::SaveAs(t) => Some(t.clone()),
            Mode::Command(t) => Some(t.clone()),
        }
    }

    pub fn active(&self) -> &Page {
        self.page_stack.last().unwrap()
    }

    pub fn active_mut(&mut self) -> &mut Page {
        self.page_stack.last_mut().unwrap()
    }

    pub fn text_page(&self) -> &Page {
        self.page_stack.first().unwrap()
    }

    pub fn text_page_mut(&mut self) -> &mut Page {
        self.page_stack.first_mut().unwrap()
    }

    pub fn reset_prompt(&mut self) {
        let name = match &self.file_name {
            Some(t) => t.clone(),
            None => String::from("Unsaved File"),
        };
        self.text_page_mut().set_prompt(name);
    }

    pub fn push(&mut self, page: Page) {
        self.page_stack.push(page);
    }

    pub fn pop(&mut self) -> Option<Page> {
        if self.page_stack.len() > 1 {
            let temp = self.page_stack.pop().unwrap();
            self.key_handler.ip = self
                .active()
                .active_cursor_location
                .as_ref()
                .unwrap()
                .clone();
            self.active_mut().active_cursor_location = None;
            Some(temp)
        } else {
            None
        }
    }

    // Saves the location of the cursor on the screen, creates a new display, resets the cursor location to 0, 0.
    pub fn add(&mut self, display_type: PageType) {
        self.active_mut().active_cursor_location = Some(self.key_handler.ip.clone());
        self.key_handler.ip = InsertionPoint::new();
        self.push(Page::new(display_type));
    }

    pub fn add_help_page(&mut self) {
        self.add(PageType::Info);
        self.active_mut().set_prompt(String::from("Help:"));
        let help_text: String = FileIO::read_from_file(&String::from("help.txt"))
            .unwrap_or(String::from(
            "Help file not found. More on \"https://github.com/JarodSGilliam/SecureFileEditor\"",
        ));
        self.active_mut().set_contents(help_text);
    }

    pub fn add_info_page(&mut self, info: String) {
        self.add(PageType::Info);
        self.active_mut().set_contents(info);
        match self.refresh_screen() {
            Ok(_) => {}
            Err(e) => eprint!("{}", e),
        };
    }

    pub fn move_ip(&mut self, direction: KeyCode) {
        self.key_handler
            .move_ip(direction, self.page_stack.first_mut().unwrap());
    }

    pub fn insertion(&mut self, input: KeyCode) {
        self.key_handler
            .insertion(input, self.page_stack.last_mut().unwrap());
    }

    pub fn clear_screen() -> crossterm::Result<()> {
        execute!(stdout(), terminal::Clear(ClearType::All))?;
        execute!(stdout(), cursor::MoveTo(0, 0))
    }
    pub fn render(&mut self) {
        if self.active().display_type.overwrites() {
            self.draw_content(self.page_stack.len() - 1);
            return;
        }
        for i in 0..self.page_stack.len() {
            self.draw_content(i);
        }
    }
    //print the char, and get the char of each row, get the total row number
    pub fn draw_content(&mut self, i: usize) {
        let on_screen = self.page_stack.get_mut(i).unwrap();
        on_screen.row_contents = split_with_n(&on_screen.contents);
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
                let row_contents = on_screen.row_contents.get(row_in_content).unwrap();
                let (len, start) = if width[row_in_content] <= self.key_handler.column_offset {
                    (0, 0)
                } else {
                    let (mut st, mut w) =
                        row_contents.unicode_truncate(self.key_handler.column_offset);
                    while w != self.key_handler.column_offset + offset_string.len() {
                        offset_string.push_str(" ");
                        let unicode_temp = row_contents
                            .unicode_truncate(self.key_handler.column_offset + offset_string.len());
                        st = unicode_temp.0;
                        w = unicode_temp.1;
                    }
                    if width[row_in_content] - w <= self.key_handler.screen_cols {
                        (row_contents.len() - st.len(), st.len())
                    } else {
                        let (s_temp, _) = row_contents.unicode_truncate(
                            self.key_handler.column_offset + self.key_handler.screen_cols,
                        );
                        (s_temp.len() - st.len(), st.len())
                    }
                };
                content.push_str(&offset_string);
                if i < self.key_handler.screen_rows - 1 {
                    if start + len == bytes[row_in_content] {
                        content.push_str(&row_contents[start..start + len].replace('\n', "\r\n"));
                    } else {
                        content.push_str(&row_contents[start..start + len]);
                        content.push_str("\r\n");
                    };
                } else {
                    if start + len == bytes[row_in_content] {
                        content.push_str(&row_contents[start..start + len].replace('\n', ""));
                    } else {
                        content.push_str(&row_contents[start..start + len]);
                    };
                }
            }
        }
        self.key_handler.bytes_in_row = bytes;
        self.key_handler.width_in_row = width;
        let temp01 = match &self.mode {
            Mode::Normal => None,
            Mode::Find(t) => Some(String::from(t.as_str())),
            Mode::Replace(t) => Some(String::from(t.as_str())),
            Mode::SaveAs(t) => Some(String::from(t.as_str())),
            Mode::Command(t) => Some(String::from(t.as_str())),
        };
        let mut stdout = stdout();
        let y = 0;
        let x = 0;
        if !on_screen.display_type.overwrites() {
            self.print_overlay(i, content);
            return;
        }
        match stdout.queue(cursor::MoveTo(x as u16, y as u16)) {
            Ok(_) => {}
            Err(_) => {}
        }
        if on_screen.display_type == PageType::Text {
            if self.key_handler.screen_cols > on_screen.prompt.len() {
                for _i in 0..(self.key_handler.screen_cols - on_screen.prompt.len()) / 2 {
                    match stdout.queue(style::PrintStyledContent(" ".reset())) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }
            if self.modified {
                match stdout.queue(style::PrintStyledContent(
                    on_screen.prompt.replace('\n', "\r\n").red(),
                )) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            } else {
                match stdout.queue(style::PrintStyledContent(
                    on_screen.prompt.replace('\n', "\r\n").reset(),
                )) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        } else {
            match stdout.queue(style::PrintStyledContent(
                on_screen.prompt.replace('\n', "\r\n").reset(),
            )) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        let text: &str = &content.clone()[..];
        self.color_struct.set_find(temp01);
        self.color_struct.coloring(text);

        if !on_screen.display_type.overwrites() {
            match stdout.queue(cursor::MoveTo(x as u16, (y + 3) as u16)) {
                Ok(_) => {}
                Err(_) => {}
            }
            for _i in 0..self.key_handler.screen_cols * 4 / 6 {
                match stdout.queue(style::PrintStyledContent("-".reset())) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
        match stdout.flush() {
            Ok(_) => {}
            Err(_) => {}
        };
        return;
    }

    pub fn print_overlay(&mut self, i: usize, content: String) {
        let mut stdout = stdout();
        let y = self.key_handler.screen_rows / 2;
        let x = self.key_handler.screen_cols / 6;

        let prompt_as_array: Vec<String> = self.page_stack[i]
            .prompt
            .clone()
            .trim()
            .split("\n")
            .map(|x| x.to_owned())
            .collect();
        let length = {
            if prompt_as_array == vec![""] {
                0
            } else {
                prompt_as_array.len()
            }
        };
        Screen::print_at_times(
            &mut stdout,
            x,
            y - 1 - length,
            "-",
            self.key_handler.screen_cols * 4 / 6,
        );
        for i in 0..length {
            Screen::create_line(
                &mut stdout,
                self.key_handler.screen_cols * 4 / 6,
                x,
                y - (length - i),
                prompt_as_array[i].to_owned(),
            );
        }
        Screen::create_line(
            &mut stdout,
            self.key_handler.screen_cols * 4 / 6,
            x,
            y,
            content,
        );
        Screen::print_at_times(
            &mut stdout,
            x,
            y + 1,
            "-",
            self.key_handler.screen_cols * 4 / 6,
        );

        if self.page_stack[i].display_type == PageType::Command {
            let x = self.key_handler.screen_cols / 4;
            Screen::create_line(
                &mut stdout,
                self.key_handler.screen_cols / 2,
                x,
                y + 2,
                "Toggle Highlight".to_owned(),
            );
            Screen::create_line(
                &mut stdout,
                self.key_handler.screen_cols / 2,
                x,
                y + 3,
                "Find".to_owned(),
            );
            Screen::create_line(
                &mut stdout,
                self.key_handler.screen_cols / 2,
                x,
                y + 4,
                "File Info".to_owned(),
            );
            Screen::print_at_times(&mut stdout, x, y + 5, "-", self.key_handler.screen_cols / 2);
        }
    }

    pub fn create_line(
        stdout: &mut std::io::Stdout,
        width: usize,
        x: usize,
        y: usize,
        text: String,
    ) {
        Screen::clean_line(stdout, width, x, y);
        Screen::print_at(stdout, x + 2, y, &text);
    }

    pub fn clean_line(stdout: &mut std::io::Stdout, width: usize, x: usize, y: usize) {
        Screen::print_at(stdout, x, y, "| ");
        Screen::print_at_times(stdout, x + 2, y, " ", width - 4);
        match stdout.queue(style::PrintStyledContent(" |".reset())) {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    pub fn print_at(stdout: &mut std::io::Stdout, x: usize, y: usize, text: &str) {
        Screen::print_at_times(stdout, x, y, text, 1);
    }

    pub fn print_at_times(
        stdout: &mut std::io::Stdout,
        x: usize,
        y: usize,
        text: &str,
        times: usize,
    ) {
        match stdout.queue(cursor::MoveTo(x as u16, y as u16)) {
            Ok(_) => {}
            Err(_) => {}
        }
        for _i in 0..times {
            match stdout.queue(style::PrintStyledContent(text.reset())) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        self.key_handler.scroll();
        let mut stdout = stdout();
        queue!(
            stdout,
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        self.render();
        let ip_x = self.key_handler.ip.x - self.key_handler.column_offset;
        let mut ip_y = self.key_handler.ip.y - self.key_handler.row_offset;
        if self.active().prompt != "" {
            ip_y += self.active().prompt.matches("\n").count();
        }
        if !self.active().display_type.overwrites() {
            queue!(
                stdout,
                cursor::MoveTo(
                    (self.key_handler.screen_cols / 6 + 2 + ip_x) as u16,
                    (self.key_handler.screen_rows / 2) as u16
                ),
                cursor::Show
            )?;
        } else {
            queue!(
                stdout,
                cursor::MoveTo(ip_x as u16, ip_y as u16),
                cursor::Show
            )?;
        }
        stdout.flush()
    }
}

// Potential additions to screen
// pub pub fn active_type(&self) -> PageType {
pub struct ColorWord {
    word: Option<String>,
    disabled: bool,
    pub language: Language,
    in_text_2: bool,
    in_text_1: bool,
    base_colors: Vec<Color>,
    parenthesis: usize,
    brackets: usize,
}
impl ColorWord {
    pub fn new(word: Option<String>, language: Language) -> Self {
        Self {
            word: word,
            disabled: false,
            language: language,
            in_text_2: false,
            in_text_1: false,
            base_colors: vec![
                Color::Yellow,
                Color::Red,
                Color::Black,
                Color::Green,
                Color::Blue,
                Color::Magenta,
            ],
            parenthesis: 0,
            brackets: 0,
        }
    }

    /*
     *  This function toggles the instance's disabled boolean
     *  for the command line.
     */

    pub fn toggle_status(&mut self) {
        match self.disabled {
            true => self.disabled = false,
            false => self.disabled = true,
        }
    }

    pub fn get_color(&mut self, word: &str) -> Color {
        // Colors all words in single or double quotes the same
        if (word == "\"") & !self.in_text_1 {
            self.in_text_2 = !self.in_text_2;
            return self.language.text_color;
        }
        if (word == "\'") & !self.in_text_2 {
            self.in_text_1 = !self.in_text_1;
            return self.language.text_color;
        }
        if self.in_text_2 || self.in_text_1 {
            return self.language.text_color;
        }
        // Does parenthesis and bracket coloring
        if word == "(" {
            let output = self.base_colors[self.parenthesis % self.base_colors.len()];
            self.parenthesis += 1;
            return output;
        }
        if word == ")" {
            if self.parenthesis > 0 {
                self.parenthesis -= 1;
            }
            return self.base_colors[self.parenthesis % self.base_colors.len()];
        }
        if word == "{" {
            let output = self.base_colors[self.brackets % self.base_colors.len()];
            self.brackets += 1;
            return output;
        }
        if word == "}" {
            if self.brackets > 0 {
                self.brackets -= 1;
            }
            return self.base_colors[self.brackets % self.base_colors.len()];
        }
        // Applies color based on highlighting.txt
        match self.language.get_color(&word.to_owned()) {
            Some(c) => {
                return c;
            }
            None => {}
        }
        // Colors all numbers the same
        let first = word.chars().next().unwrap();
        if first.is_numeric() {
            return self.language.numbers_color;
        }
        // Colors all uppercase words the same
        if first.is_uppercase() {
            return self.language.capitals_color;
        }
        // Colors all other words the default
        Color::Reset
    }

    pub fn set_find(&mut self, find: Option<String>) {
        self.word = find;
    }
    // Highlights the word which is currently being searched for (if there is one)
    pub fn get_background_color(&self, c: &str) -> Color {
        match &self.word {
            Some(color) => {
                if c == color {
                    Color::Red
                } else {
                    Color::Reset
                }
            }
            None => Color::Reset,
        }
    }
    // Prints the given text with the correct colors
    pub fn coloring(&mut self, text: &str) {
        let mut stdout = stdout();
        let line: Vec<&str> = text.split("\r\n").collect();
        let mut multi_line_comment: bool = false;
        for i in 0..line.len() {
            let words: Vec<String> = split_up(line[i].to_owned());
            let mut comment: bool = false;
            for i in 0..words.len() {
                if words[i] == self.language.comment_keyword {
                    comment = true;
                } else if words[i] == self.language.ml_comment_start_keyword {
                    multi_line_comment = true;
                }
                // The actual printing part \/
                let foreground_color = Some({
                    if self.disabled {
                        Color::Reset
                    } else if comment || multi_line_comment {
                        Color::Rgb { r: 0, g: 255, b: 0 }
                    } else if i < words.len() - 1 && words[i] != "(" && words[i + 1] == "(" {
                        Color::Rgb {
                            r: 255,
                            g: 150,
                            b: 0,
                        }
                    } else {
                        self.get_color(words[i].as_str())
                    }
                });
                if self.word == None {
                    match stdout.queue(style::PrintStyledContent(StyledContent::new(
                        ContentStyle {
                            foreground_color: foreground_color,
                            background_color: Some(self.get_background_color(words[i].as_str())),
                            attributes: Attributes::default(),
                        },
                        &words[i],
                    ))) {
                        Ok(_) => {}
                        Err(_) => {}
                    };
                } else {
                    let temp01 =
                        pop_off(vec![words[i].clone()], self.word.clone().unwrap().as_str());
                    for w in temp01 {
                        match stdout.queue(style::PrintStyledContent(StyledContent::new(
                            ContentStyle {
                                foreground_color: foreground_color,
                                background_color: Some(self.get_background_color(w.as_str())),
                                attributes: Attributes::default(),
                            },
                            &w,
                        ))) {
                            Ok(_) => {}
                            Err(_) => {}
                        };
                    }
                }
                if words[i] == self.language.ml_comment_end_keyword {
                    multi_line_comment = false;
                }
            }
            if i != line.len() - 1 {
                match stdout.queue(style::PrintStyledContent("\r\n".reset())) {
                    Ok(_) => {}
                    Err(_) => {}
                };
            }
        }
        match stdout.flush() {
            Ok(_) => {}
            Err(_) => {}
        };
    }
}

// Cals pop_off_these for all the nessisary strings
fn split_up(input: String) -> Vec<String> {
    return pop_off_these(
        vec![input],
        vec![
            " ", "(", ")", "{", "}", ".", ";", ":", "\"", "'", "[", "]", "//", "*/", "*/", "<?-",
            "->", "#", ",",
        ],
    );
}

// Calls pop_off over the entire given string
fn pop_off_these(mut input: Vec<String>, items: Vec<&str>) -> Vec<String> {
    for item in items {
        input = pop_off(input, item);
    }
    input
}

// Splits each element in a vector by the given item in place (instead of creating a vector of vectors, creates a single vector)
fn pop_off(input: Vec<String>, item: &str) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    for i in input {
        let mut rest = i;
        while rest.contains(item) {
            let (first, second) = match rest.split_once(item) {
                Some(t) => t,
                None => {
                    output.push(String::from(&rest));
                    break;
                }
            };
            if first != "" {
                output.push(String::from(first));
            }
            output.push(String::from(item));
            rest = second.to_owned();
        }
        if rest != "" {
            output.push(String::from(rest));
        }
    }
    output
}
