use itertools::Itertools;
use regex::Regex;
use std::collections::HashMap;

macro_attr! {
    #[derive(Debug, Clone, Default, Builder!)]
    pub struct Match {
        pub pattern: &'static str,
        pub i: usize,
        pub j: usize,
        pub token: String,
        pub matched_word: Option<String>,
        pub rank: Option<usize>,
        pub dictionary_name: Option<&'static str>,
        pub graph: Option<String>,
        pub reversed: bool,
        pub l33t: bool,
        pub sub: Option<HashMap<char, char>>,
        pub sub_display: Option<String>,
        pub turns: Option<usize>,
        pub shifted_count: Option<usize>,
    }
}

impl Match {
    pub fn build(&mut self) -> Match {
        self.clone()
    }
}

#[doc(hidden)]
pub fn omnimatch(password: &str, user_inputs: &Option<Vec<String>>) -> Vec<Match> {
    let user_inputs = user_inputs.clone()
        .map(|items| items.iter().enumerate().map(|(i, x)| (x.clone(), i + 1)).collect());
    MATCHERS.iter()
        .flat_map(|x| x.get_matches(password, &user_inputs))
        .sorted_by(|a, b| Ord::cmp(&a.i, &b.i))
        .into_iter()
        .sorted_by(|a, b| Ord::cmp(&a.j, &b.j))
}

lazy_static! {
    static ref L33T_TABLE: HashMap<char, Vec<char>> = {
        let mut table = HashMap::with_capacity(12);
        table.insert('a', vec!['4', '@']);
        table.insert('b', vec!['8']);
        table.insert('c', vec!['(', '{', '[', '<']);
        table.insert('e', vec!['3']);
        table.insert('g', vec!['6', '9']);
        table.insert('i', vec!['1', '!', '|']);
        table.insert('l', vec!['1', '|', '7']);
        table.insert('o', vec!['0']);
        table.insert('s', vec!['$', '5']);
        table.insert('t', vec!['+', '7']);
        table.insert('x', vec!['%']);
        table.insert('z', vec!['2']);
        table
    };
    static ref GRAPHS: HashMap<&'static str, HashMap<char, Vec<Option<&'static str>>>> = {
        let mut table = HashMap::with_capacity(4);
        table.insert("qwerty", super::adjacency_graphs::QWERTY.clone());
        table.insert("dvorak", super::adjacency_graphs::DVORAK.clone());
        table.insert("keypad", super::adjacency_graphs::KEYPAD.clone());
        table.insert("mac_keypad", super::adjacency_graphs::MAC_KEYPAD.clone());
        table
    };
}

trait Matcher: Sync {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match>;
}

lazy_static! {
    static ref MATCHERS: [Box<Matcher>; 8] = [
        Box::new(DictionaryMatch {}),
        Box::new(ReverseDictionaryMatch {}),
        Box::new(L33tMatch {}),
        Box::new(SpatialMatch {}),
        Box::new(RepeatMatch {}),
        Box::new(SequenceMatch {}),
        Box::new(RegexMatch {}),
        Box::new(DateMatch {}),
    ];
}

struct DictionaryMatch {}

impl Matcher for DictionaryMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        fn do_trials(matches: &mut Vec<Match>,
                     password: &str,
                     dictionary_name: &'static str,
                     ranked_dict: &HashMap<&str, usize>) {
            let len = password.len();
            let password_lower = password.to_lowercase();
            for i in 0..(len + 1) {
                for j in 0..(len + 1) {
                    let word = &password_lower[i..j];
                    if let Some(rank) = ranked_dict.get(word) {
                        matches.push(Match::default()
                            .pattern("dictionary")
                            .i(i)
                            .j(j)
                            .token(password[i..j].to_string())
                            .matched_word(Some(word.to_string()))
                            .rank(Some(*rank))
                            .dictionary_name(Some(dictionary_name))
                            .build());
                    }
                }
            }
        }

        let mut matches = Vec::new();

        for (dictionary_name, ranked_dict) in super::frequency_lists::RANKED_DICTIONARIES.iter() {
            do_trials(&mut matches, &password, dictionary_name, ranked_dict);
        }
        if let &Some(ref inputs) = user_inputs {
            do_trials(&mut matches,
                      &password,
                      "user_inputs",
                      &inputs.iter().map(|(x, &i)| (x.as_str(), i)).collect());
        }

        matches
    }
}

struct ReverseDictionaryMatch {}

impl Matcher for ReverseDictionaryMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        let reversed_password = password.chars().rev().collect::<String>();
        (DictionaryMatch {})
            .get_matches(&reversed_password, user_inputs)
            .into_iter()
            .map(|mut x| {
                // Reverse token back
                x.token = x.token.chars().rev().collect();
                x.reversed = true;
                x.i = password.len() - 1 - x.j;
                x.j = password.len() - 1 - x.i;
                x
            })
            .collect()
    }
}

struct L33tMatch {}

impl Matcher for L33tMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        let mut matches = Vec::new();
        for sub in enumerate_l33t_replacements(&relevant_l33t_subtable(password)) {
            if sub.is_empty() {
                break;
            }
            let subbed_password = translate(password, &sub);
            for mut m4tch in (DictionaryMatch {}).get_matches(&subbed_password, user_inputs) {
                let token = &password[m4tch.i..m4tch.j];
                if Some(token.to_lowercase()) == m4tch.matched_word {
                    // Only return the matches that contain an actual substitution
                    continue;
                }
                let match_sub: HashMap<char, char> = sub.clone()
                    .into_iter()
                    .filter(|&(subbed_chr, _)| token.contains(subbed_chr))
                    .collect();
                m4tch.l33t = true;
                m4tch.token = token.to_string();
                m4tch.sub_display =
                    Some(match_sub.iter().map(|(k, v)| format!("{} -> {}", k, v)).collect());
                m4tch.sub = Some(match_sub);
                matches.push(m4tch);
            }
        }
        matches.into_iter().filter(|ref x| !x.token.is_empty()).collect()
    }
}

fn translate(string: &str, chr_map: &HashMap<char, char>) -> String {
    string.chars().map(|c| *chr_map.get(&c).unwrap_or(&c)).collect()
}

fn relevant_l33t_subtable(password: &str) -> HashMap<char, Vec<char>> {
    let password_chars: Vec<char> = password.chars().collect();
    let mut subtable: HashMap<char, Vec<char>> = HashMap::new();
    for (letter, subs) in L33T_TABLE.iter() {
        let relevant_subs: Vec<char> =
            subs.iter().filter(|&x| password_chars.contains(x)).cloned().collect();
        if !relevant_subs.is_empty() {
            subtable.insert(*letter, relevant_subs);
        }
    }
    subtable
}

fn enumerate_l33t_replacements(table: &HashMap<char, Vec<char>>) -> Vec<HashMap<char, char>> {
    /// Recursive function that does the work
    fn helper(table: &HashMap<char, Vec<char>>,
              subs: Vec<Vec<(char, char)>>,
              remaining_keys: &[char])
              -> Vec<Vec<(char, char)>> {
        if remaining_keys.is_empty() {
            return subs;
        }
        let (first_key, rest_keys) = remaining_keys.split_first().unwrap();
        let mut next_subs: Vec<Vec<(char, char)>> = Vec::new();
        for l33t_chr in &table[first_key] {
            for sub in &subs {
                let mut dup_l33t_index = None;
                for i in 0..(sub.len() + 1) {
                    if sub[i].0 == *l33t_chr {
                        dup_l33t_index = Some(i);
                        break;
                    }
                }
                if let Some(idx) = dup_l33t_index {
                    let mut sub_alternative = sub.clone();
                    sub_alternative.remove(idx);
                    sub_alternative.push((*l33t_chr, *first_key));
                    next_subs.push(sub.clone());
                    next_subs.push(sub_alternative);
                } else {
                    let mut sub_extension = sub.clone();
                    sub_extension.push((*l33t_chr, *first_key));
                    next_subs.push(sub_extension);
                }
            }
        }
        helper(table,
               next_subs.into_iter().map(|x| x.iter().unique().cloned().collect()).collect(),
               rest_keys)
    }

    helper(table,
           vec![vec![]],
           table.keys().cloned().collect::<Vec<char>>().as_slice())
        .into_iter()
        .map(|sub| sub.into_iter().collect::<HashMap<char, char>>())
        .collect()
}

struct SpatialMatch {}

impl Matcher for SpatialMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        GRAPHS.iter()
            .flat_map(|(graph_name, graph)| spatial_match_helper(password, graph, graph_name))
            .collect()
    }
}

lazy_static! {
    static ref SHIFTED_REGEX: Regex = Regex::new("[~!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>?]").unwrap();
}

fn spatial_match_helper(password: &str,
                        graph: &HashMap<char, Vec<Option<&str>>>,
                        graph_name: &str)
                        -> Vec<Match> {
    let mut matches = Vec::new();
    let mut i = 0;
    while i < password.len() - 1 {
        let mut j = i + 1;
        let mut last_direction = None;
        let mut turns = 0;
        let mut shifted_count = 0;
        if ["qwerty", "dvorak"].contains(&graph_name) &&
           SHIFTED_REGEX.is_match(&password[i..(i + 1)]) {
            shifted_count = 1;
        }
        loop {
            let prev_char = password[j - 1..j].chars().next().unwrap();
            let mut found = false;
            let mut found_direction = -1;
            let mut cur_direction = -1;
            let mut adjacents = graph.get(&prev_char).cloned().unwrap_or(vec![]);
            // consider growing pattern by one character if j hasn't gone over the edge.
            if j < password.len() {
                let cur_char = password[j..(j + 1)].chars().next().unwrap();
                for adj in adjacents {
                    cur_direction += 1;
                    if let Some(adj) = adj {
                        if let Some(adj_position) = adj.find(cur_char) {
                            found = true;
                            found_direction = cur_direction;
                            if adj_position == 1 {
                                // index 1 in the adjacency means the key is shifted,
                                // 0 means unshifted: A vs a, % vs 5, etc.
                                // for example, 'q' is adjacent to the entry '2@'.
                                // @ is shifted w/ index 1, 2 is unshifted.
                                shifted_count += 1;
                            }
                            if last_direction != Some(found_direction) {
                                // adding a turn is correct even in the initial case when last_direction is null:
                                // every spatial pattern starts with a turn.
                                turns += 1;
                                last_direction = Some(found_direction);
                            }
                            break;
                        }
                    }
                }
            }
            if found {
                // if the current pattern continued, extend j and try to grow again
                j += 1;
            } else {
                // otherwise push the pattern discovered so far, if any...
                if j - 1 > 2 {
                    // Don't consider length 1 or 2 chains
                    matches.push(Match::default()
                        .pattern("spatial")
                        .i(i)
                        .j(j - 1)
                        .token(password[i..(j + 1)].to_string())
                        .graph(Some(graph_name.to_string()))
                        .turns(Some(turns))
                        .shifted_count(Some(shifted_count))
                        .build());
                }
                i = j;
                break;
            }
        }
    }
    matches
}

struct RepeatMatch {}

impl Matcher for RepeatMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        unimplemented!()
    }
}

struct SequenceMatch {}

impl Matcher for SequenceMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        unimplemented!()
    }
}

struct RegexMatch {}

impl Matcher for RegexMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        unimplemented!()
    }
}

struct DateMatch {}

impl Matcher for DateMatch {
    fn get_matches(&self,
                   password: &str,
                   user_inputs: &Option<HashMap<String, usize>>)
                   -> Vec<Match> {
        unimplemented!()
    }
}
