use clap::{App, Arg, ArgMatches};
use std::{fmt::Display, fs::File, io::{self, Write}, path::Path, process, time::Duration};
use ncd::{CurlConfig, CurlNCDReadAccessor, NCDFileReader, NCDReadAccessor, StdNCDReadAccessor};

fn die<E: Display>(value: E) -> ! {
    eprintln!("{}",value);
    process::exit(1);
}

fn die_on_error<T,E: Display>(value: Result<T,E>) -> T {
    match value {
        Ok(v) => v,
        Err(e) => die(e)
    }
}

enum Source {
    File,
    Http
}

fn guess_source(path: &str) -> Source {
    if path.contains("//") {
        Source::Http
    } else {
        Source::File
    }
}

impl Source {
    fn new(arg: Option<&str>, path: &str) -> Source {
        match arg {
            Some("file") => Source::File,
            Some("http") => Source::Http,
            _ => guess_source(path)
        }
    }

    fn make_accessor(&self, path: &str, curl_config: &CurlConfig) -> io::Result<Box<dyn NCDReadAccessor>> {
        Ok(match self {
            Source::File => {
                let file_path = Path::new(path);
                if !file_path.exists() {
                   die(format!("No such file: {}",path)); 
                }        
                let file = File::open(file_path)?;
                Box::new(StdNCDReadAccessor::new(file)?)
            },
            Source::Http => {
                // XXX ocnfigurable
                Box::new(CurlNCDReadAccessor::new(curl_config,path)?)
            }
        })
    }
}

fn str_to_u32(s: &str) -> Result<u32,String> {
    s.parse::<u32>().map_err(|e| format!("Invalid integer: {}",e))
}

fn make_curl_config(matches: &ArgMatches) -> CurlConfig {
    let mut config = CurlConfig::new();
    if let Some(timeout) = matches.value_of("timeout") {
        let timeout = die_on_error(str_to_u32(timeout));
        config = config.connect_timeout(Duration::from_millis(timeout as u64));
    }
    config
}

pub fn make_app() -> App<'static,'static> {
    App::new("ncd file lookcup").version("0.0.1")
        .author("Dan Sheppard <dan@ebi.ac.uk")
        .about("Looks up data in ncd files (locally or remotely)")
        .arg(Arg::with_name("KEY")
            .help("input file to convert")
            .index(1)
            .required(true)
        )
        .arg(Arg::with_name("PATH")
            .help("output file to create")
            .index(2)
            .required(true)
        )
        .arg(Arg::with_name("source")
            .short("-s")
            .long("--source")
            .help("specify source type (optional: will guess)")
            .takes_value(true)
            .possible_value("file")
            .possible_value("http")
            .possible_value("guess")
            .default_value("guess")
        )
        .arg(Arg::with_name("timeout")
            .short("-t")
            .long("--timeout")
            .help("specify timeout for remote methods (ms)")
            .takes_value(true)
        )
    }

fn main() {
    let app = make_app();
    let matches = app.get_matches();
    let path = matches.value_of("PATH").unwrap();
    let key =  matches.value_of("KEY").unwrap().as_bytes();
    let source_type = Source::new(matches.value_of("source"),path);
    let curl_config = make_curl_config(&matches);
    let accessor = die_on_error(source_type.make_accessor(path,&curl_config));
    let mut reader = die_on_error(NCDFileReader::new_box(accessor));
    let value = die_on_error(reader.get(key));
    if let Some(value) = value.as_ref() {
        die_on_error(io::stdout().write_all(value));
        process::exit(0);
    } else {
        process::exit(1);
    }
}
