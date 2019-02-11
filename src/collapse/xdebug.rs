use hashbrown::HashMap;
use std::io;
use std::io::prelude::*;

const SCALE_FACTOR: f32 = 1_000_000.0;
static CALLS: &[&str] = &["require", "require_once", "include", "include_once"];

pub struct Options;

pub fn handle_file<R: BufRead, W: Write>(
    _opts: Options,
    mut reader: R,
    mut writer: W,
) -> io::Result<()> {
    let mut stacks: HashMap<String, f32> = HashMap::new();
    let mut current_stack = Vec::with_capacity(16);
    let mut prev_start_time = 0.0;
    let mut line = String::new();

    loop {
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        if line.contains("TRACE START") {
            break;
        }

        line.clear();
    }

    loop {
        line.clear();

        if reader.read_line(&mut line)? == 0 {
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

        let collapsed = current_stack.join(";");
        let duration = SCALE_FACTOR * (time - prev_start_time);
        *stacks.entry(collapsed).or_insert(0.0) += duration;

        if is_exit {
            current_stack.pop();
        } else {
            let func_name = parts.by_ref().skip(1).next();
            let path_name = parts.by_ref().skip(1).next();

            if let (Some(func_name), Some(path_name)) = (func_name, path_name) {
                if CALLS.contains(&func_name) {
                    current_stack.push(format!("{}({})", func_name.clone(), path_name.clone()));
                } else {
                    current_stack.push(format!("{}", func_name.clone()));
                }
            }
        }

        prev_start_time = time;
    }

    for (key, value) in stacks {
        writeln!(writer, "{} {}", key, value)?;
    }

    Ok(())
}
