use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};

const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/VERSION"));

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Option {
    pub dest: Vec<String>,
    pub check: bool,   //-c
    pub speed: u64,    //-s
    pub thread: usize, //-t
    pub hidden: bool,  //-H
    pub link: bool,    //-l
    pub quiet: bool,   //-q
    pub status: bool,  //-a
    pub strict: bool,  //-r
    pub warn: bool,    //-w
}

impl Option {
    // pub fn new() -> Option {
    //     Option::default()
    // }

    pub fn from_args() -> Option {
        let opt = Option::default();
        let args = Command::new("fastmd5")
            .version(VERSION)
            .about("Print or check MD5 checksums.\nFor more information, see https://github.com/moold/fastMD5")
            .arg_required_else_help(true)
            .arg(
                Arg::new("dest")
                    .value_name("FILE|DIRECTORY")
                    .num_args(1..)
                    .required(true)
                    .help("when the input is a directory, each file within the directory will be calculated individually."),
            ).arg(
                Arg::new("check")
                    .short('c')
                    .long("check")
                    .help("read MD5 sums from the FILEs and check them.")
                    .action(ArgAction::SetTrue),
            ).arg(
                Arg::new("speed")
                    .short('s')
                    .long("speed")
                    .value_name("INT")
                    .default_value(opt.speed.to_string())
                    .value_parser(clap::value_parser!(u64).range(0..10))
                    .help("speed level ranges from 0 (slowest) to 9 (fastest).\n0 = slowest, performs a full sequential computation, equivalent to standard `md5sum`, but ~20% faster.\n1 = full-file computation with optimized buffering, significantly faster than level 0.\n2-9 = block-based sampling with progressively fewer data blocks processed,\nmuch faster than levels 0 and 1, but produces approximate checksums."),
            ).arg(
                Arg::new("thread")
                    .short('t')
                    .long("thread")
                    .value_name("INT")
                    .default_value(opt.thread.to_string())
                    .value_parser(value_parser!(usize))
                    .help("number of threads."),
            )
            .arg(
                Arg::new("hidden")
                    .short('H')
                    .long("hidden")
                    .help("when the input is a directory, don't ignore hidden files.")
                    .action(ArgAction::SetTrue),
            ).arg(
                Arg::new("link")
                    .short('l')
                    .long("link")
                    .help("when the input is a directory, don't ignore symbolic links.")
                    .action(ArgAction::SetTrue),
            ).arg(
                Arg::new("quiet")
                    .short('q')
                    .long("quiet")
                    .help("don't print OK for each successfully verified file.")
                    .action(ArgAction::SetTrue),
            ).arg(
                Arg::new("status")
                    .short('a')
                    .long("status")
                    .help("don't output anything, status code shows success.")
                    .action(ArgAction::SetTrue),
            ).arg(
                Arg::new("strict")
                    .short('S')
                    .long("strict")
                    .hide(true)
                    .help("exit non-zero for improperly formatted checksum lines.")
                    .action(ArgAction::SetTrue),
            ).arg(
                Arg::new("warn")
                    .short('w')
                    .long("warn")
                    .help("warn about improperly formatted checksum lines.")
                    .hide(true)
                    .action(ArgAction::SetTrue),
            ).get_matches();

        opt.update(args)
    }

    fn update(self, mut args: ArgMatches) -> Option {
        Option {
            //safely unwrap, becasue the default values have been set
            dest: args.remove_many::<String>("dest").expect("Missing input!").collect::<Vec<_>>(),
            check: args.get_flag("check"),
            speed: args.remove_one::<u64>("speed").unwrap(),
            thread: args.remove_one::<usize>("thread").unwrap(),
            hidden: args.get_flag("hidden"),
            link: args.get_flag("link"),
            quiet: args.get_flag("quiet"),
            status: args.get_flag("status"),
            strict: args.get_flag("strict"),
            warn: args.get_flag("warn"),
            ..Default::default()
        }
    }
}

impl Default for Option {
    fn default() -> Self {
        Option {
            dest: Vec::new(),
            check: false,
            speed: 5,
            thread: 3,
            hidden: false,
            link: false,
            quiet: false,
            status: false,
            strict: false,
            warn: false,
        }
    }
}
