use crate::insertion_point::*;
use crate::page::*;
use crossterm::event::KeyCode;
use std::cmp;
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

/*
    Struct responsible for moving the user's (i)nsertion (p)oint while
    the program is running.
*/
// ip.x, ip.y indicates the index of cursor and use the screen_cols and rows to store the screen size
pub struct KeyHandler {
    pub ip: InsertionPoint,
    pub screen_cols: usize,
    pub screen_rows: usize,
    pub bytes_in_row: Vec<usize>,
    pub width_in_row: Vec<usize>,
    pub num_of_rows: usize,
    pub row_offset: usize,
    pub column_offset: usize,
}
impl KeyHandler {
    //create new KeyHandler with insertion point at origin (top-left corner)
    pub fn new(window_size: (usize, usize)) -> KeyHandler {
        let instance = KeyHandler {
            ip: InsertionPoint::new(),
            screen_cols: window_size.0,
            screen_rows: window_size.1 - 2,
            bytes_in_row: Vec::new(),
            width_in_row: Vec::new(),
            num_of_rows: 0,
            row_offset: 0,
            column_offset: 0,
        };
        //println!("x:{} y:{}", instance.ip.x, instance.ip.y);
        return instance;
    }

    //check cursor position when scroll
    pub fn scroll(&mut self) {
        self.row_offset = cmp::min(self.row_offset, self.ip.y);
        if self.ip.y >= self.row_offset + self.screen_rows {
            self.row_offset = self.ip.y - self.screen_rows + 1;
        }
        self.column_offset = cmp::min(self.column_offset, self.ip.x);
        if self.ip.x >= self.column_offset + self.screen_cols {
            self.column_offset = self.ip.x - self.screen_cols + 1;
        }
    }

    //move the insertion point based on user's keypress
    pub fn move_ip(&mut self, operation: KeyCode, on_screen: &Page) {
        match operation {
            KeyCode::Up => {
                if self.ip.y > 0 {
                    self.ip.y -= 1;
                    self.ip.x = cmp::min(self.ip.x, *self.width_in_row.get(self.ip.y).unwrap());
                    self.ip.x = on_screen
                        .row_contents
                        .get(self.ip.y)
                        .unwrap()
                        .unicode_truncate(self.ip.x)
                        .1;
                }
            }
            KeyCode::Down => {
                if self.ip.y < self.num_of_rows - 1 {
                    self.ip.y += 1;
                    self.ip.x = cmp::min(self.ip.x, *self.width_in_row.get(self.ip.y).unwrap());
                    self.ip.x = on_screen
                        .row_contents
                        .get(self.ip.y)
                        .unwrap()
                        .unicode_truncate(self.ip.x)
                        .1;
                }
            }
            KeyCode::Left => {
                if self.ip.x > 0 {
                    self.ip.x -= 1;
                    let (_, mut w) = on_screen
                        .row_contents
                        .get(self.ip.y)
                        .unwrap()
                        .unicode_truncate(self.ip.x);
                    while w != self.ip.x {
                        self.ip.x -= 1;
                        w = on_screen
                            .row_contents
                            .get(self.ip.y)
                            .unwrap()
                            .unicode_truncate(self.ip.x)
                            .1;
                    }
                } else if self.ip.y > 0 {
                    self.ip.y -= 1;
                    self.ip.x = *self.width_in_row.get(self.ip.y).unwrap();
                }
            }
            KeyCode::Right => {
                if self.ip.x < *self.width_in_row.get(self.ip.y).unwrap() {
                    self.ip.x += 1;
                    let (_, mut w) = on_screen
                        .row_contents
                        .get(self.ip.y)
                        .unwrap()
                        .unicode_truncate(self.ip.x);
                    while w != self.ip.x {
                        self.ip.x += 1;
                        w = on_screen
                            .row_contents
                            .get(self.ip.y)
                            .unwrap()
                            .unicode_truncate(self.ip.x)
                            .1;
                    }
                } else if self.ip.y < self.num_of_rows - 1 {
                    self.ip.x = 0;
                    self.ip.y += 1;
                }
                // else is for default, the limit is set to be the screen size for further adjustment
            }
            KeyCode::End => self.ip.x = *self.width_in_row.get(self.ip.y).unwrap(),
            KeyCode::Home => self.ip.x = 0,
            _ => {} //more code needed
        }
    }

    pub fn insertion(&mut self, operation: KeyCode, on_screen: &mut Page) {
        match operation {
            KeyCode::Char(c) => {
                on_screen
                    .contents
                    .insert(self.get_current_location_in_string(on_screen), c);
                on_screen.row_contents = split_with_n(&on_screen.contents);
                //println!("\ninsertion b_i_r len: {}", self.bytes_in_row.len());
                //println!("\ninsertion ip x: {}, y: {}", self.ip.x, self.ip.y);
                if self.ip.y == self.bytes_in_row.len() {
                    self.bytes_in_row.push(c.to_string().len());
                } else {
                    self.bytes_in_row[self.ip.y] += c.to_string().len();
                }
                self.ip.x += 1;
                let (_, mut w) = on_screen
                    .row_contents
                    .get(self.ip.y)
                    .unwrap()
                    .unicode_truncate(self.ip.x);
                while w != self.ip.x {
                    self.ip.x += 1;
                    w = on_screen
                        .row_contents
                        .get(self.ip.y)
                        .unwrap()
                        .unicode_truncate(self.ip.x)
                        .1;
                }
            }

            KeyCode::Tab => {
                //println!("tabbing");
                on_screen
                    .contents
                    .insert_str(self.get_current_location_in_string(on_screen), "    ");
                self.bytes_in_row[self.ip.y] += 4;
                self.ip.x += 4;
            }
            // KeyCode::Tab => {
            //     on_screen.contents.insert(self.get_current_location_in_string(on_screen), '\t');
            //     self.rows[self.ip.y]+=1;
            //     self.ip.x += 1;
            // },
            KeyCode::Backspace => {
                if self.ip.x == 0 {
                    if self.ip.y == 0 {
                        //do nothing since insertion point is at origin (top-left)
                    } else {
                        on_screen
                            .contents
                            .remove(self.get_current_location_in_string(on_screen) - 1);
                        on_screen.row_contents = split_with_n(&on_screen.contents);
                        self.ip.x = self.width_in_row[self.ip.y - 1];
                        self.bytes_in_row[self.ip.y - 1] =
                            on_screen.row_contents[self.ip.y - 1].len();
                        self.width_in_row[self.ip.y - 1] =
                            on_screen.row_contents[self.ip.y - 1].width();
                        self.bytes_in_row.remove(self.ip.y);
                        self.ip.y -= 1;
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
                    self.bytes_in_row[self.ip.y] -= deleted.len_utf8();
                    self.ip.x -= 1;
                    let (_, mut w) = on_screen
                        .row_contents
                        .get(self.ip.y)
                        .unwrap()
                        .unicode_truncate(self.ip.x);
                    while w != self.ip.x {
                        self.ip.x -= 1;
                        w = on_screen
                            .row_contents
                            .get(self.ip.y)
                            .unwrap()
                            .unicode_truncate(self.ip.x)
                            .1;
                    }
                }
            }

            KeyCode::Delete => {
                if self.ip.x == self.bytes_in_row[self.ip.y] {
                    if self.ip.y == self.num_of_rows - 1 {
                        //do nothing since insertion point is at end of file (bottom-right)
                    } else {
                        //println!("{}", self.get_current_location_in_string());
                        on_screen
                            .contents
                            .remove(self.get_current_location_in_string(on_screen));
                        self.bytes_in_row[self.ip.y] += self.bytes_in_row[self.ip.y + 1] - 1;
                        self.bytes_in_row.remove(self.ip.y + 1);
                        self.num_of_rows -= 1;
                    }
                } else {
                    on_screen
                        .contents
                        .remove(self.get_current_location_in_string(on_screen));
                    self.bytes_in_row[self.ip.y] -= 1;
                }
            }

            KeyCode::Enter => {
                //println!("start_Enter b_i_r len: {}", self.bytes_in_row.len());
                on_screen
                    .contents
                    .insert(self.get_current_location_in_string(on_screen), '\n');
                self.ip.x = 0;
                self.ip.y += 1;
            }
            _ => {}
        }
    }

    //Backspace and moving forward when typing
    pub fn get_current_location_in_string(&mut self, on_screen: &Page) -> usize {
        let mut x = 0;
        for i in 0..self.ip.y {
            x += self.bytes_in_row[i];
        }
        let (s, _) = match on_screen.row_contents.get(self.ip.y) {
            Some(s) => s,
            None => "", // is this right?
        }
        .unicode_truncate(self.ip.x);
        x += s.replace('\n', "").len();
        x
    }
}

pub fn split_with_n(content: &String) -> Vec<String> {
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
