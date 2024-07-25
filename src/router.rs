use std::collections::HashMap;

use regex::RegexSet;

pub struct Router {
    pub routes: RegexSet,
}

impl Router {
    pub fn new() -> Router {
        Router { 
            routes: RegexSet::empty(),
        }
    }

    pub fn add_route(&mut self, path: &str) {
        assert!(path.len() > 0 && path.chars().nth(0).unwrap() == '/');

        let mut regex = String::from("^");

        for substr in path.split("/").skip(1) {
            let len = substr.len();

            if len == 0 {
                continue;
            }

            let first_char = substr.chars().nth(0).unwrap();
            let last_char = substr.chars().nth(len - 1).unwrap();

            if len > 2 && first_char == ':' && last_char == ':' {
                let group_name = substr[1..len-1].to_string();
                regex += &(r"/(?<".to_string() + &group_name + r">\d+)");
            } else if len > 2 && first_char == '$' && last_char == '$' {
                let group_name = substr[1..len-1].to_string();
                regex += &(r"/(?<".to_string() + &group_name + r">\w+)");
            } else {
                regex += &(r"/".to_string() + &substr);
            }
        }

        // in case path was "/"
        if regex == "^" {
            regex.push('/');
        }

        regex.push('$');

        let mut patterns = self.routes.patterns().to_vec();
        patterns.push(regex);
        self.routes = RegexSet::new(patterns).expect("Error when compiling regex")
    }
}
