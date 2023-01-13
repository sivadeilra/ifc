use anyhow::Context;
use regex::Regex;
use structopt::StructOpt;

fn parse_regex(value: &str) -> anyhow::Result<Regex> {
    regex::RegexBuilder::new(value)
        .build()
        .context("Invalid regex")
}

#[derive(Clone, Debug, StructOpt, Default)]
pub struct Options {
    /// Derive Debug impls for types, by default
    #[structopt(long)]
    pub derive_debug: bool,

    /// Emit the code for use as a standalone crate, rather than to be used as a
    /// module.
    #[structopt(long)]
    pub standalone: bool,

    /// Only generate object-like macros that match any allowlist <regex>. If no
    /// allow list regexes are provided, then all macros will be generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    allowlist_macro: Vec<Regex>,

    /// After filtering object-like macros through the allowlist (if any),
    /// prevent macros in the blocklist from being generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    blocklist_macro: Vec<Regex>,

    /// Only generate types that match any allowlist <regex>. If no
    /// allow list regexes are provided, then all types will be generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    allowlist_type: Vec<Regex>,

    /// After filtering types through the allowlist (if any),
    /// prevent types in the blocklist from being generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    blocklist_type: Vec<Regex>,

    /// Only generate non-member functions that match any allowlist <regex>. If
    /// no allow list regexes are provided, then all non-member functions will
    /// be generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    allowlist_function: Vec<Regex>,

    /// After filtering non-member functions through the allowlist (if any),
    /// prevent non-member functions in the blocklist from being generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    blocklist_function: Vec<Regex>,

    /// Only generate non-member variables that match any allowlist <regex>. If
    /// no allow list regexes are provided, then all non-member variables will
    /// be generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    allowlist_variable: Vec<Regex>,

    /// After filtering non-member variables through the allowlist (if any),
    /// prevent non-member variables in the blocklist from being generated.
    #[structopt(long, parse(try_from_str = parse_regex))]
    blocklist_variable: Vec<Regex>,
}

impl Options {
    pub fn macro_filter(&self) -> Filter {
        Filter {
            allowlist: &self.allowlist_macro,
            blocklist: &self.blocklist_macro,
        }
    }

    pub fn type_filter(&self) -> Filter {
        Filter {
            allowlist: &self.allowlist_type,
            blocklist: &self.blocklist_type,
        }
    }

    pub fn function_filter(&self) -> Filter {
        Filter {
            allowlist: &self.allowlist_function,
            blocklist: &self.blocklist_function,
        }
    }

    pub fn variable_filter(&self) -> Filter {
        Filter {
            allowlist: &self.allowlist_variable,
            blocklist: &self.blocklist_variable,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Filter<'a> {
    allowlist: &'a Vec<Regex>,
    blocklist: &'a Vec<Regex>,
}

impl<'a> Filter<'a> {
    pub fn is_allowed(&self, name: &str) -> bool {
        (self.allowlist.is_empty() || self.allowlist.iter().any(|regex| regex.is_match(name)))
            && !self.blocklist.iter().any(|regex| regex.is_match(name))
    }

    pub fn is_allowed_qualified_name(&self, name: &str, parent_name: &str) -> bool {
        (self.allowlist.is_empty() && self.blocklist.is_empty())
            || self.is_allowed(&format!("{}::{}", parent_name, name))
    }
}

#[derive(Default)]
pub struct TestOptions<'a> {
    pub allowlist_macro: &'a [&'static str],
    pub blocklist_macro: &'a [&'static str],
    pub allowlist_type: &'a [&'static str],
    pub blocklist_type: &'a [&'static str],
    pub allowlist_function: &'a [&'static str],
    pub blocklist_function: &'a [&'static str],
    pub allowlist_variable: &'a [&'static str],
    pub blocklist_variable: &'a [&'static str],
}

impl Options {
    pub fn default_for_testing() -> Self {
        Self {
            derive_debug: true,
            standalone: true,
            allowlist_macro: Vec::new(),
            blocklist_macro: Vec::new(),
            allowlist_type: Vec::new(),
            blocklist_type: Vec::new(),
            allowlist_function: Vec::new(),
            blocklist_function: Vec::new(),
            allowlist_variable: Vec::new(),
            blocklist_variable: Vec::new(),
        }
    }

    pub fn for_testing(options: &TestOptions) -> Self {
        Self {
            allowlist_macro: options
                .allowlist_macro
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            blocklist_macro: options
                .blocklist_macro
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            allowlist_type: options
                .allowlist_type
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            blocklist_type: options
                .blocklist_type
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            allowlist_function: options
                .allowlist_function
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            blocklist_function: options
                .blocklist_function
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            allowlist_variable: options
                .allowlist_variable
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            blocklist_variable: options
                .blocklist_variable
                .iter()
                .map(|item| parse_regex(item).unwrap())
                .collect::<Vec<_>>(),
            ..Default::default()
        }
    }
}
