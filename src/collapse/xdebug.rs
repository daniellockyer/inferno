use hashbrown::HashMap;
use std::io;
use std::io::prelude::*;

const SCALE_FACTOR: f32 = 1_000_000.0;
static CALLS: &[&str] = &["require", "require_once", "include", "include_once"];

#[derive(Debug, Default)]
pub struct Options {
    /// include TID and PID with process names [1]
    pub invocation_count: bool,
}

pub fn handle_file<R: BufRead, W: Write>(
    opts: Options,
    mut reader: R,
    mut writer: W,
) -> io::Result<()> {
    let mut stacks: HashMap<String, f32> = HashMap::new();
    let mut current_stack = Vec::with_capacity(16);
    let mut was_exit = false;
    let mut prev_start_time = 0.0;
    let mut line = String::new();

    let invocation_count = !opts.invocation_count;

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

        let parts = line.split('\t').collect::<Vec<&str>>();

        if parts.len() < 4 {
            continue;
        }

        let (is_exit, time) = (parts[2], parts[3]);
        let time = time.parse::<f32>().unwrap();

        let is_exit = match is_exit {
            "1" => true,
            "0" => false,
            _ => panic!("uh oh"),
        };

        if is_exit {
            if invocation_count  {
                if current_stack.is_empty() {
                    println!("[WARNING] Found function exit without corresponding entrance. Discarding line. Check your input.\n");
                    continue;
                }

                let collapsed = current_stack.join(";");
                let duration = SCALE_FACTOR * (time - prev_start_time);
                *stacks.entry(collapsed).or_insert(0.0) += duration;

                current_stack.pop();
            } else {
                if !was_exit {
                    let collapsed = current_stack.join(";");
                    *stacks.entry(collapsed).or_insert(0.0) += 1.0;
                }
                current_stack.pop();
                was_exit = true;
            }
        } else {
            let mut func_name = parts[5].to_owned();

            if invocation_count {
                if CALLS.contains(&parts[5]) {
                    func_name.push_str(&format!("({})", parts[7]));
                }

                if !current_stack.is_empty() {
                    let collapsed = current_stack.join(";");
                    let duration = SCALE_FACTOR * (time - prev_start_time);
                    *stacks.entry(collapsed).or_insert(0.0) += duration;
                }
            } else {
                was_exit = false;
            }

            current_stack.push(func_name);
        }
        prev_start_time = time;
    }

    for (key, value) in stacks {
        writeln!(writer, "{} {}", key, value)?;
    }

    Ok(())
}
