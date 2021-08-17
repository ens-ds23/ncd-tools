use clap::{App, Arg};
use std::{fmt::Display, fs::File, io::{self, Write}, path::Path, process};
use ncd::{NCDFileReader, NCDReadAccessor, StdNCDReadAccessor };

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

    fn make_accessor(&self, path: &str) -> io::Result<Box<dyn NCDReadAccessor>> {
        let file_path = Path::new(path);
        if !file_path.exists() {
           die(format!("No such file: {}",path)); 
        }
        let file = File::open(file_path)?;
        Ok(match self {
            Source::File => Box::new(StdNCDReadAccessor::new(file)?),
            Source::Http => Box::new(StdNCDReadAccessor::new(file)?)
        })
    }
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
            //.possible_value("http")
            .possible_value("guess")
            .default_value("guess")
        )
}

fn main() {
    let app = make_app();
    let matches = app.get_matches();
    let path = matches.value_of("PATH").unwrap();
    let key =  matches.value_of("KEY").unwrap().as_bytes();
    let source_type = Source::new(matches.value_of("source"),path);
    let accessor = die_on_error(source_type.make_accessor(path));
    let mut reader = die_on_error(NCDFileReader::new_box(accessor));
    let value = die_on_error(reader.get(key));
    if let Some(value) = value.as_ref() {
        die_on_error(io::stdout().write_all(value));
        process::exit(0);
    } else {
        process::exit(1);
    }
}
