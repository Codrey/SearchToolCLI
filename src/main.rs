use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path,PathBuf};
use std::process;
use std::sync::Arc;
use std::thread;

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            // skip symlinks if desired or recursive into directories
            if child_path.is_dir() {
                collect_files(&child_path, files)?;
            } else if child_path.is_file() {
                files.push(child_path);
            }
        }
    }

    Ok(())
}

fn search_file(path: &Path, query: &str) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut line_num: usize = 0;
    let mut hits = Vec::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }
        line_num += 1;

        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.contains(query) {
            hits.push(format!("{}:{}:{}", path.display(), line_num, trimmed));
            // matches += 1;
        }
    }

    Ok(hits)
}

fn search_concurrent(files: Vec<PathBuf>, query: Arc<String>, num_threads: usize) -> usize {
    let worker_count = num_threads.max(1).min(files.len().max(1));
    let chunk_size = (files.len() + worker_count - 1) / worker_count;

    let mut chunks: Vec<Vec<PathBuf>> = Vec::with_capacity(worker_count);
    let mut iter = files.into_iter();
    for _ in 0..worker_count {
        let batch: Vec<PathBuf> = iter.by_ref().take(chunk_size).collect();
        if !batch.is_empty() {
            chunks.push(batch);
        }
    }

    let mut handles = Vec::with_capacity(chunks.len());
    for batch in chunks {
        let query_ref = Arc::clone(&query);
        let handle = thread::spawn(move || {
            let mut local_hits = Vec::new();
            for file_path in batch {
                match search_file(&file_path, query_ref.as_str()) {
                    Ok(mut file_hits) => local_hits.append(&mut file_hits),
                    Err(err) => eprintln!("Error search {}: {}", file_path.display(), err),
                }
            }
            local_hits
        });
        handles.push(handle);
    }

    let mut total_matches = 0;
    for handle in handles {
        if let Ok(hits) = handle.join() {
            total_matches += hits.len();
            for line in hits {
                println!("{}", line);
            }
        }
    }

    total_matches
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

    let root = Path::new(&target);
    let mut files = Vec::new();
    if let Err(err) = collect_files(root, &mut files) {
        eprintln!("Error reading {}: {}", root.display(), err);
        process::exit(1);
    }

    let thread_count = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    
    let shared_query = Arc::new(query);
    let _total = search_concurrent(files, shared_query, thread_count);

    // let path = Path::new(&target);
    // if let Err(err) = search_file(path, &query) {
    //     eprintln!("Error search {}: {}", path.display(), err);
    //     process::exit(1);
    // }
}
