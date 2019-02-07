use std::fs::File;
use std::io::{self, BufReader};
use structopt::StructOpt;

use inferno::collapse::xdebug::{handle_file, Options};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "inferno-collapse-xdebug",
    author = "",
    after_help = "\
[1] perf script must emit both PID and TIDs for these to work; eg, Linux < 4.1:
        perf script -f comm,pid,tid,cpu,time,event,ip,sym,dso,trace
    for Linux >= 4.1:
        perf script -F comm,pid,tid,cpu,time,event,ip,sym,dso,trace
    If you save this output add --header on Linux >= 3.14 to include perf info."
)]
struct Opt {
    /// Weight by time
    #[structopt(long = "time")]
    time_weighting: bool,

    /// Measure by invocation counts
    #[structopt(long = "tid")]
    invocation_count_only: bool,

    /// perf script output file, or STDIN if not specified
    infile: Option<String>,
}

impl Into<Options> for Opt {
    fn into(self) -> Options {
        Options {
            time_weighting: self.time_weighting,
            invocation_count_only: self.invocation_count_only,
        }
    }
}

fn main() -> io::Result<()> {
    let (infile, options) = {
        let opt = Opt::from_args();
        (opt.infile.clone(), opt.into())
    };

    match infile {
        Some(ref f) => {
            let r = BufReader::with_capacity(128 * 1024, File::open(f)?);
            handle_file(options, r, io::stdout().lock())
        }
        None => {
            let stdin = io::stdin();
            let r = BufReader::with_capacity(128 * 1024, stdin.lock());
            handle_file(options, r, io::stdout().lock())
        }
    }
}
