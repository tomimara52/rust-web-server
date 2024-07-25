use std::{collections::HashMap, iter::zip};

use regex::{Regex, RegexSet};

use crate::handler::{Handler, Params};

pub struct Router {
    routes: RegexSet,
    handlers: Vec<&'static dyn Handler>,
}

impl Router {
    pub fn new() -> Router {
        Router { 
            routes: RegexSet::empty(),
            handlers: Vec::new(),
        }
    }

    pub fn add_route(&mut self, path: &str, handler: &'static dyn Handler) {
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
        self.routes = RegexSet::new(patterns).expect("Error when compiling regex");
        
        self.handlers.push(handler);
    }

    pub fn get_handler(&self, path: &str) -> Option<(&'static dyn Handler, Params)> {
        let matches: Vec<_> = self.routes.matches(path).into_iter().collect();
        
        if matches.len() == 0 {
            return None;
        }

        let regex = Regex::new(&self.routes.patterns()[matches[0]]).unwrap();
        let caps = regex.captures(path)?;
        let names = regex.capture_names().skip(1);

        let mut params = HashMap::new();

        for (name, cap) in zip(names, caps.iter().skip(1)) {
            params.insert(name?.to_string(), cap?.as_str().to_string());
        }

        Some((
            self.handlers[matches[0]],
            params
        ))
    }
}
