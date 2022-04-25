use crossterm::style::Color;

#[derive(Debug)]
#[derive(Clone)]
pub struct Keywords {
    pub color : Color,
    pub keywords : Vec<String>,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct Language {
    pub colors : Vec<Keywords>,
    pub comment_keyword : String,
    pub ml_comment_start_keyword : String,
    pub ml_comment_end_keyword : String,
    pub capitals_color : Color,
    pub numbers_color : Color,
    pub text_color : Color,
}

impl Language {
    pub fn new(input : String)-> Language {
        if input == "" {
            return Language {
                colors: vec![],
                comment_keyword: String::new(),
                ml_comment_start_keyword: String::new(),
                ml_comment_end_keyword: String::new(),
                capitals_color: Color::Reset,
                numbers_color: Color::Reset,
                text_color: Color::Reset,
            };
        }
        let mut colors : Vec<Keywords> = Vec::new();
        let mut comment_keyword = String::new();
        let mut ml_comment_start_keyword = String::new();
        let mut ml_comment_end_keyword = String::new();
        let mut capitals_color = Color::Rgb{r:100, g:255, b:255}; //Color::Reset
        let mut numbers_color = Color::Rgb{r:100, g:255, b:100}; //Color::Reset
        let mut text_color = Color::Magenta; //Color::Reset;
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
            if temp.0 == "mlcomment" {
                let temp = match temp.1.split_once(" ") {
                    Some(t) => t,
                    None => continue,
                };
                ml_comment_start_keyword = temp.0.to_owned();
                ml_comment_end_keyword = temp.1.to_owned();
                continue;
            }
            if temp.0 == "capitals" {
                capitals_color = match Language::parse_color(temp.1) {
                    Some(c) => c,
                    None => continue,
                };
                continue;
            }
            if temp.0 == "numbers" {
                numbers_color = match Language::parse_color(temp.1) {
                    Some(c) => c,
                    None => continue,
                };
                continue;
            }
            if temp.0 == "text" {
                text_color = match Language::parse_color(temp.1) {
                    Some(c) => c,
                    None => continue,
                };
                continue;
            }
            let colorthing = match i.split_once(")") {
                Some(t) => t,
                None => continue,
            };
            let color = match Language::parse_color(colorthing.0) {
                Some(c) => c,
                None => continue,
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
            ml_comment_end_keyword,
            capitals_color,
            numbers_color,
            text_color,
        }
    }

    fn parse_color(input : &str) -> Option<Color> {
        let stuff : Vec<String> = input.replace("(", "").replace(")", "").split(",").map(|x| x.trim().to_owned()).collect();
        Some(Color::Rgb {
            r: match stuff[0].trim().parse() {
                Ok(t) => t,
                Err(_e) => {
                    return None;
                },
            }, g: match stuff[1].trim().parse() {
                Ok(t) => t,
                Err(_e) => {
                    return None;
                },
            }, b: match stuff[2].trim().parse() {
                Ok(t) => t,
                Err(_e) => {
                    return None;
                },
            }
        })
    }

    pub fn get_color(&self, input : &String) -> Option<Color> {
        for k in &self.colors {
            for s in &k.keywords {
                if input == s {
                    return Some(k.color);
                }
            }
        }
        return None;
    }
}
