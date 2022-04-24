use crossterm::style::Color;

#[derive(Debug)]
pub struct Keywords {
    pub color : Color,
    pub keywords : Vec<String>,
}

#[derive(Debug)]
pub struct Language {
    pub colors : Vec<Keywords>,
    pub comment_keyword : String,
    pub ml_comment_start_keyword : String,
    pub ml_comment_end_keyword : String,
}

impl Language {
    pub fn new(input : String)-> Language {
        let mut colors : Vec<Keywords> = Vec::new();
        let mut comment_keyword = String::new();
        let mut ml_comment_start_keyword = String::new();
        let mut ml_comment_end_keyword = String::new();
        for i in input.split("\n") {
            if i == "" {
                continue;
            }
            let temp = match i.split_once(" ") {
                Some(t) => t,
                None => continue,
            };
            if temp.0 == "comment" {
                comment_keyword = temp.1.to_owned();
                continue;
            }
            if temp.0 == "mlcomments" {
                ml_comment_start_keyword = temp.1.to_owned();
                continue;
            }
            if temp.0 == "mlcommente" {
                ml_comment_end_keyword = temp.1.to_owned();
                continue;
            }
            let colorthing = match i.split_once(")") {
                Some(t) => t,
                None => continue,
            };
            let stuff : Vec<String> = colorthing.0.replace("(", "").split(",").map(|x| x.trim().to_owned()).collect();
            let color = Color::Rgb {
                r: match stuff[0].trim().parse() {
                    Ok(t) => t,
                    Err(_e) => continue,
                }, g: match stuff[1].trim().parse() {
                    Ok(t) => t,
                    Err(_e) => continue,
                }, b: match stuff[2].trim().parse() {
                    Ok(t) => t,
                    Err(_e) => continue,
                }
            };
            let keywords : Vec<String> = colorthing.1.split(",").map(|x| x.trim().to_owned()).collect();
            let keywords_set : Keywords = Keywords {
                color,
                keywords,
            };
            colors.push(keywords_set);
        }
        Language {
            colors,
            comment_keyword,
            ml_comment_start_keyword, 
            ml_comment_end_keyword
        }
    }

    pub fn get_color(&self, input : &String) -> Color {
        for k in &self.colors {
            for s in &k.keywords {
                if input == s {
                    return k.color;
                }
            }
        }
        return Color::Reset;
    }
}
