use std::{fmt::Display, fs::File, io, path::Path, process};

use clap::{App, Arg, ArgMatches};
use infer::Infer;
use ncd::{NCDBuild, NCDBuildConfig, NCDFlatConfig, NCDFlatSource, NCDValueSource};

fn looks_like_utf8(bytes: &[u8]) -> bool {
    for b in bytes {
        if *b == 0xC0 || *b == 0xC1 || *b > 0xF4 { return false; }
    }
    if String::from_utf8(bytes.to_vec()).is_ok() { return true; }
    let mut valid = bytes;
    let mut chop = 0;
    while chop < 3 && valid.len() > 0 && valid[valid.len()-1] > 0x7F && valid[valid.len()-1] < 0xC0 {
        valid = &valid[0..(valid.len()-1)];
        chop += 1;
    }
    if valid.len() > 0 && valid[valid.len()-1] > 0xBF { valid = &valid[0..(valid.len()-1)]; chop += 1; }
    if chop > 0 && valid.len() == 0 { return false; }
    String::from_utf8(valid.to_vec()).is_ok()
}

#[derive(Debug)]
enum Format {
    Flat
}

impl Format {
    fn from_cli(name: &str, path: &str) -> Format {
        match name {
            "flat" => Format::Flat,
            "guess" => {
                if let Some(format) = guess_format(path) {
                    format
                } else {
                    die(format!("unknown file format for {}",path));                    
                }
            },
            _ => {
                die(format!("unknown file format for {}",path));
            }
        }
    }

    fn from_mime_type(mime_type: &str) -> Option<Format> {
        match mime_type {
            "text/plain" => Some(Format::Flat),
            _ => None
        }
    }

    fn to_source(&self, path: &str, flat_config: &NCDFlatConfig) -> io::Result<Box<dyn NCDValueSource>> {
        Ok(match self {
            Format::Flat => {
                Box::new(NCDFlatSource::new(Path::new(path),flat_config)?)
            },
        })
    }
}

fn guess_format(path: &str) -> Option<Format> {
    let mut inferer = Infer::new();
    inferer.add("text/plain",".txt",|bytes| {
        looks_like_utf8(bytes)
    });
    let value = die_on_error(inferer.get_from_path(path));
    value.and_then(|value| Format::from_mime_type(value.mime_type()))
}

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

fn make_flat_config(matches: &ArgMatches) -> NCDFlatConfig {
    let field = die_on_error(str_to_u32(matches.value_of("field").unwrap()));
    let separator = matches.value_of("delimiter").map(|s| s.to_string());
    let skip_blank = !matches.is_present("keep-blank");
    let comment = matches.value_of("comment").map(|s| s.to_string());
    let inline_comments = matches.is_present("inline-comments");
    let trim_tail = !matches.is_present("keep-tail");
    NCDFlatConfig::new()
        .index(field as usize)
        .separator(separator)
        .skip_blank(skip_blank)
        .comment_char(comment)
        .inline_comments(inline_comments)
        .trim_tail(trim_tail)
}

fn make_careful_config() -> NCDBuildConfig {
    NCDBuildConfig::new()
        .target_page_size(16384)
        .heap_wiggle_room(1.1)
        .target_load_factor(0.75)
        .rebuild_page_factor(1.1)
}

/*
chain!(force_header_size,get_force_header_size,Option<u32>,NCDBuildConfig);
*/

fn modify_build_config(config: &mut NCDBuildConfig, matches: &ArgMatches) {
    if let Some(page_size) = matches.value_of("page-size") {
        *config = config.target_page_size(die_on_error(str_to_u32(page_size)));
    }
    if let Some(load_factor) = matches.value_of("load-factor") {
        *config = config.target_load_factor(die_on_error(str_to_f64(load_factor)));
    }
    if let Some(heap_wiggle) = matches.value_of("heap-wiggle") {
        *config = config.heap_wiggle_room(die_on_error(str_to_f64(heap_wiggle)));
    }
    if let Some(page_size) = matches.value_of("min-entries") {
        *config = config.min_entries_per_page(die_on_error(str_to_u32(page_size)) as u64);
    }
    if let Some(external_threshold) = matches.value_of("external-threshold") {
        *config = config.external_trheshold(die_on_error(str_to_f64(external_threshold)));
    }
    if let Some(rebuild_factor) = matches.value_of("rebuild-factor") {
        *config = config.rebuild_page_factor(die_on_error(str_to_f64(rebuild_factor)));
    }
    if let Some(force_header_size) = matches.value_of("force-header-size") {
        *config = config.force_header_size(Some(die_on_error(str_to_u32(force_header_size))));
    }
}

fn str_to_u32(s: &str) -> Result<u32,String> {
    s.parse::<u32>().map_err(|e| format!("Invalid integer: {}",e))
}

fn str_to_f64(s: &str) -> Result<f64,String> {
    s.parse::<f64>().map_err(|e| format!("Invalid floating-point number: {}",e))
}

pub fn make_app() -> App<'static,'static> {
    App::new("ncd file builder").version("0.0.1")
        .author("Dan Sheppard <dan@ebi.ac.uk")
        .about("Builds ncd files from a variety of sources")
        .arg(Arg::with_name("INPUT")
            .help("input file to convert")
            .index(1)
            .required(true)
        )
        .arg(Arg::with_name("OUTPUT")
            .help("output file to create")
            .index(2)
            .required(true)
        )
        .arg(Arg::with_name("format")
            .short("-t")
            .long("--type")
            .help("specify input file format (optional: will guess)")
            .takes_value(true)
            .possible_value("flat")
            .possible_value("gdbm")
            .possible_value("guess")
            .default_value("guess")
        )
        .arg(Arg::with_name("field")
            .short("-f")
            .long("--field")
            .takes_value(true)
            .help("when using separated file, which field to use (first is 1)")
            .default_value("1")
            .validator(|v| str_to_u32(&v).map(|_| ()))
        )
        .arg(Arg::with_name("delimiter")
            .short("-d")
            .long("--delimiter")
            .takes_value(true)
            .help("when using separated file, which delimiter to use (default is arbitrary whitespace)")
        )
        .arg(Arg::with_name("keep-blank")
            .short("-B")
            .long("--blank")
            .help("when using separated file, keep blank lines (default is to discard)")
        )
        .arg(Arg::with_name("comment")
            .short("-C")
            .long("--comment")
            .takes_value(true)
            .help("when using separated file, treat as comment character (default is none)")
        )
        .arg(Arg::with_name("inline-comments")
            .short("-I")
            .long("--inline")
            .help("when using separated file, strip trailing comments (default is none)")
            .requires("comment")
        )
        .arg(Arg::with_name("keep-tail")
            .short("-T")
            .long("--keep-tail")
            .help("when using separated file, don't strip trailing whitespace (default is none)")
        )
        .arg(Arg::with_name("careful")
            .short("-c")
            .long("--careful")
            .help("Use careful settings for building (will take longer but probably result in smaller file)")
        )
        .arg(Arg::with_name("page-size")
            .short("-p")
            .long("--page-size")
            .takes_value(true)
            .help("target page size in bytes (requests will be of this size) (default 32768, careful 16384)")
            .validator(|v| str_to_u32(&v).map(|_| ()))
        )
        .arg(Arg::with_name("load-factor")
            .long("--load-factor")
            .takes_value(true)
            .help("target hash-table load factor (requests will be of this size) (default 0.5, careful 0.75)")
            .validator(|v| str_to_f64(&v).map(|_| ()))
        )
        .arg(Arg::with_name("heap-wiggle")
            .long("--heap-wiggle")
            .takes_value(true)
            .help("heap-figgle room (requests will be of this size) (default 1.25, careful 1.1)")
            .validator(|v| str_to_f64(&v).map(|_| ()))
        )
        .arg(Arg::with_name("min-entries")
            .long("--min-entries")
            .takes_value(true)
            .help("minimum entries per page (default 100)")
            .validator(|v| str_to_u32(&v).map(|_| ()))
        )
        .arg(Arg::with_name("external-threshold")
            .short("-e")
            .long("--external-threshold")
            .takes_value(true)
            .help("store entries longer than this proportion of page size are external (default 0.1)")
            .validator(|v| str_to_f64(&v).map(|_| ()))
        )
        .arg(Arg::with_name("rebuild-factor")
            .short("-r")
            .long("--rebuild-factor")
            .takes_value(true)
            .help("increase page size by this factor each attempt (default 1.2, careful 1.1)")
            .validator(|v| str_to_f64(&v).map(|_| ()))
        )
        .arg(Arg::with_name("force-header-size")
            .long("--force-header")
            .takes_value(true)
            .help("force header size in bytes when possible (default no-forcing)")
            .possible_value("2")
            .possible_value("4")
        )
    }

fn main() {
    let app = make_app();
    let matches = app.get_matches();
    let flat_config = make_flat_config(&matches);
    let mut build_config = if matches.is_present("careful") { make_careful_config() } else { NCDBuildConfig::new() };
    modify_build_config(&mut build_config,&matches);
    let input = matches.value_of("INPUT").unwrap();
    let input_path = Path::new(input);
    if !input_path.exists() {
        die(&format!("File does not exist: {}",input));
    }
    let output = matches.value_of("OUTPUT").unwrap();
    let output_path = Path::new(output);
    if File::create(output_path).is_err() {
        die(&format!("Cannot create output file: {}",output));
    }
    let format = Format::from_cli(matches.value_of("format").unwrap(),matches.value_of("INPUT").unwrap());
    let source = die_on_error(format.to_source(&input,&flat_config));
    let mut builder = die_on_error(NCDBuild::new(&build_config,source.as_ref(),&output_path));
    loop {
        println!("Attempting to build: {}",builder.describe_attempt());
        let success = die_on_error(builder.attempt());
        println!("  {}",builder.result());
        if success { break }
    }
}

#[cfg(test)]
mod test {
    use crate::{looks_like_utf8, make_app, make_careful_config, make_flat_config, modify_build_config};

    #[test]
    fn test_looks_like_utf8() {
        assert_eq!(looks_like_utf8(b""),true);
        assert_eq!(looks_like_utf8(b"ab"),true);
        assert_eq!(looks_like_utf8(b"abc"),true);
        assert_eq!(looks_like_utf8(b"abcd"),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xC2]),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xF3,0x90,0x90,0x90]),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xF3,0x90,0x90]),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xF3,0x90]),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xF3]),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xF3,0xF1]),false);
        assert_eq!(looks_like_utf8(&[0xF3,0x90]),false);
        assert_eq!(looks_like_utf8(&[0xC2]),false);
        assert_eq!(looks_like_utf8(&[0xC2,0xA0]),true);
        assert_eq!(looks_like_utf8(&[0x21,0x21,0xF3,0x90,0x90,0x90,0x90]),false);
        assert_eq!(looks_like_utf8(&[0x21,0xC0,0x21,0xF3,0x90,0x90,0x90]),false);
    }

    // XXX pr gdbm print
    // XXX verbose
    #[test]
    fn test_flat_config() {
        let app = make_app();
        let matches = app.get_matches_from(["file","x","y"].iter());
        let config = make_flat_config(&matches);
        assert_eq!(1,*config.get_index());
        assert_eq!(None,*config.get_separator());
        assert_eq!(true,*config.get_skip_blank());
        assert_eq!(None,*config.get_comment_char());
        assert_eq!(false,*config.get_inline_comments());
        assert_eq!(true,*config.get_trim_tail());
        let app = make_app();
        let matches = app.get_matches_from([
            "file","x","y",
            "-f","2",
            "-d","\t",
            "-B",
            "-C","#",
            "-I",
            "-T"
        ].iter());
        let config = make_flat_config(&matches);
        assert_eq!(2,*config.get_index());
        assert_eq!(Some("\t".to_string()),*config.get_separator());
        assert_eq!(false,*config.get_skip_blank());
        assert_eq!(Some("#".to_string()),*config.get_comment_char());
        assert_eq!(true,*config.get_inline_comments());
        assert_eq!(false,*config.get_trim_tail());        
    }

    #[test]
    fn test_build_config() {
        let config = make_careful_config();
        assert_eq!(16384,*config.get_target_page_size());
        assert_eq!(0.75,*config.get_target_load_factor());
        assert_eq!(1.1,*config.get_heap_wiggle_room());
        assert_eq!(100,*config.get_min_entries_per_page());
        assert_eq!(0.1,*config.get_external_trheshold());
        assert_eq!(1.1,*config.get_rebuild_page_factor());
        assert_eq!(None,*config.get_force_header_size());
        let app = make_app();
        let mut config = make_careful_config();
        let matches = app.get_matches_from([
            "file","x","y",
            "-p","8192",
            "--load-factor","0.6",
            "--heap-wiggle","1.3",
            "--min-entries","200",
            "-e","0.15",
            "-r","1.05",
            "--force-header","4",
        ].iter());
        modify_build_config(&mut config,&matches);
        assert_eq!(8192,*config.get_target_page_size());
        assert_eq!(0.6,*config.get_target_load_factor());
        assert_eq!(1.3,*config.get_heap_wiggle_room());
        assert_eq!(200,*config.get_min_entries_per_page());
        assert_eq!(0.15,*config.get_external_trheshold());
        assert_eq!(1.05,*config.get_rebuild_page_factor());
        assert_eq!(Some(4),*config.get_force_header_size());
    }
}
