use hashbrown::HashMap;
use std::fmt::Write as WriteFmt;
use std::io::{self, Write};
use regex::Regex;
use std::io::prelude::*;

const SCALE_FACTOR: f32 = 1_000_000.0;
static CALLS: &[&str] = &["require", "require_once", "include", "include_once"];

pub struct Options;

#[derive(PartialEq, Eq, Hash)]
struct Call(String);

#[derive(Default)]
struct CallStack {
    with_path: HashMap<(String, String), usize>,
    without_path: HashMap<String, usize>,
    interned: Vec<Call>,
    stack: Vec<usize>,
}

pub fn handle_file<R: BufRead, W: Write>(
    _opts: Options,
    mut reader: R,
    mut writer: W,
) -> io::Result<()> {
    let mut stacks: HashMap<_, f32> = HashMap::new();
    let mut current_stack = CallStack::default();
    let mut prev_start_time = 0.0;
    let mut line = String::new();

    let searcher = Regex::new("TRACE START").unwrap();
    let end_searcher = Regex::new("TRACE END").unwrap();

    loop {
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        if searcher.is_match(&line) {
            break;
        }

        line.clear();
    }

    loop {
        line.clear();

        if reader.read_line(&mut line)? == 0 {
            break;
        }

        if end_searcher.is_match(&line) {
            break;
        }

        let mut parts = line.split_whitespace().into_iter().skip(2);

        let (is_exit, time) =
            if let (Some(is_exit), Some(time)) = (parts.by_ref().next(), parts.by_ref().next()) {
                let is_exit = match is_exit {
                    "1" => true,
                    "0" => false,
                    a => panic!(format!("uh oh: {}", a)),
                };

                let time = time.parse::<f32>().unwrap();

                (is_exit, time)
            } else {
                continue;
            };

        if is_exit && current_stack.is_empty() {
            eprintln!("[WARNING] Found function exit without corresponding entrance. Discarding line. Check your input.\n");
            continue;
        }

        {
            let current = current_stack.current();
            let duration = SCALE_FACTOR * (time - prev_start_time);
            if let Some(call_time) = stacks.get_mut(current) {
                *call_time += duration;
            } else {
                stacks.insert(current.to_vec().into_boxed_slice(), duration);
            }
        }

        if is_exit {
            current_stack.pop();
        } else {
            let func_name = parts.by_ref().skip(1).next();
            let path_name = parts.by_ref().skip(1).next();

            if let (Some(func_name), Some(path_name)) = (func_name, path_name) {
                if CALLS.contains(&func_name) {
                    current_stack.call_with_path(func_name, path_name);
                } else {
                    current_stack.call_without_path(func_name);
                }
            }
        }

        prev_start_time = time;
    }

    for (key, value) in stacks {
        line.clear();
        current_stack.write_name(&key, &mut line);
        writeln!(writer, "{} {}", &line, value)?;
    }

    Ok(())
}

impl Call {
    fn with_path(name: &str, path: &str) -> Self {
        Call(format!("{}({})", name, path))
    }

    fn without_path(name: &str) -> Self {
        Call(format!("{}", name))
    }

    fn display_name(&self) -> &str {
        &self.0
    }
}

impl CallStack {
    fn call_with_path(&mut self, name: &str, path: &str) {
        let entry_key = (name.into(), path.into());
        let (map, interned) = (&mut self.with_path, &mut self.interned);
        let unique = map.entry(entry_key)
            .or_insert_with(move || {
                let index = interned.len();
                interned.push(Call::with_path(name, path));
                index
            });
        self.stack.push(*unique)
    }

    fn call_without_path(&mut self, name: &str) {
        let (map, interned) = (&mut self.without_path, &mut self.interned);
        if let Some(unique) = map.get(name) {
            return self.stack.push(*unique)
        }

        let index = interned.len();
        interned.push(Call::without_path(name));
        map.insert(name.into(), index);
        self.stack.push(index)
    }

    fn pop(&mut self) {
        self.stack.pop();
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn current(&self) -> &[usize] {
        self.stack.as_slice()
    }

    /// Create a name for the current stack.
    ///
    /// This is potentially costly.
    fn write_name(&self, indices: &[usize], buffer: &mut String) {
        let mut indices = indices.iter().cloned();
        if let Some(first) = indices.by_ref().next() {
            buffer.push_str(self.interned[first].display_name());
        }
        while let Some(next) = indices.next() {
            write!(buffer, ";{}", self.interned[next].display_name()).unwrap();
        }
    }
}
