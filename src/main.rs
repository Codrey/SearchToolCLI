use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::process;

fn search_file(path: &Path, query: &str) -> io::Result<usize> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut line_num: usize = 0;
    let mut matches: usize = 0;

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }
        line_num += 1;

        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.contains(query) {
            println!("{}:{}:{}", path.display(), line_num, trimmed);
            matches += 1;
        }
    }

    Ok(matches)
}

fn main() {
    let mut args = env::args().skip(1);
    let query = match args.next() {
        Some(q) => q,
        None => {
            eprintln!("Usage: SearchToolCLI <pattern> <file_or_dir>");
            process::exit(2);
        }
    };
    let target = match args.next() {
        Some(t) => t,
        None => {
            eprintln!("Usage: SearchToolCLI <pattern> <file_or_dir>");
            process::exit(2);
        }
    };

    let path = Path::new(&target);
    if let Err(err) = search_file(path, &query) {
        eprintln!("Error search {}: {}", path.display(), err);
        process::exit(1);
    }
}
