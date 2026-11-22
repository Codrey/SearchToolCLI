use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

struct SearchConfig {
    query: String,
    case_insensitive: bool,
}

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

fn line_matches(trimmed: &str, config: &SearchConfig) -> bool {
    if config.case_insensitive {
        let lower_line = trimmed.to_lowercase();
        config.query.split('|').any(|part| {
            !part.is_empty() && lower_line.contains(&part.to_lowercase())
        })
    } else {
        config.query
            .split('|')
            .any(|part| !part.is_empty() && trimmed.contains(part))
    }
}

fn search_file(path: &Path, config: &SearchConfig) -> io::Result<Vec<String>> {
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
        if line_matches(trimmed, config) {
            hits.push(format!("{}:{}:{}", path.display(), line_num, trimmed));
        }
    }

    Ok(hits)
}

fn search_concurrent(files: Vec<PathBuf>, config: Arc<SearchConfig>, num_threads: usize) -> usize {
    if files.is_empty() {
        return 0;
    }
    
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
        let cfg = Arc::clone(&config);
        let handle = thread::spawn(move || {
            let mut local_hits = Vec::new();
            for file_path in batch {
                match search_file(&file_path, cfg.as_ref()) {
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

fn parse_args() -> Result<(SearchConfig, String), &'static str> {
    let mut case_insensitive = false;
    let mut positional: Vec<String> = Vec::new();

    for arg in env::args().skip(1) {
        if arg.trim() == "-i" {
            case_insensitive = true;
        } else {
            positional.push(arg);
        }
    }

    if positional.len() < 2 {
        return Err("Usage: SearchToolsCLI [-i] <pattern> <file_or_dir>");
    }

    Ok((
        SearchConfig {
            query: positional[0].clone(),
            case_insensitive,
        },
        positional[1].clone(),
    ))
}

// pwshll short cut installer and table output
fn ensure_powershell_shortcuts() {
    let Ok(exe_path) = env::current_exe() else {
        return;
    };
    let exe_str = exe_path.to_string_lossy().replace('\'', "''");

    let Ok(user_profile) = env::var("USERPROFILE") else {
        return;
    };

    let target_profile = PathBuf::from(&user_profile)
        .join("Documents")
        .join("WindowsPowerShell")
        .join("Microsoft.PowerShell_profile.ps1");

    let marker = "# >>> SearchToolCLI s4/S4 <<<";

    if let Ok(existing) = fs::read_to_string(&target_profile) {
        if existing.contains(marker) {
            return;
        }
    }

    let profile_str = target_profile.to_string_lossy().replace('\'', "''");
    let block = format!(
        "\r\n{marker}\r\n\
function s4 {{\r\n\
    $exe = '{exe_str}'\r\n\
    if (-not (Test-Path -LiteralPath $exe)) {{ Write-Error \"Executable not found: $exe\"; return }}\r\n\
    & $exe @args | ForEach-Object {{\r\n\
        if ($_ -match '^(.+?):(\\d+):(.*)$') {{\r\n\
            [PSCustomObject]@{{ File = Split-Path -Leaf $matches[1]; Line = [int]$matches[2]; Text = $matches[3].Trim() }}\r\n\
        }}\r\n\
    }} | Format-Table -Property File, Line, Text -AutoSize -Wrap\r\n\
}}\r\n\
# <<< SearchToolCLI s4/S4 <<<\r\n"
    );

    let block_escaped = block.replace('\'', "''");
    let ps_cmd = format!(
        "$ErrorActionPreference = 'Stop'; \
         $p = '{profile_str}'; \
         $dir = Split-Path -Parent $p; \
         if (-not (Test-Path -LiteralPath $dir)) {{ New-Item -ItemType Directory -Path $dir -Force | Out-Null }}; \
         Add-Content -LiteralPath $p -Value '{block_escaped}' -Encoding UTF8"
    );

    match Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_cmd])
        .output()
    {
        Ok(output) if !output.status.success() => {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: PowerShell profile install failed: {}", err_msg.trim());
        }
        Err(err) => {
            eprintln!("Warning: could not launch powershell.exe: {}", err);
        }
        _ => {}
    }
}

fn main() {
    ensure_powershell_shortcuts();

    let (config, target) = match parse_args() {
        Ok(parsed) => parsed,
        Err(usage) => {
            eprintln!("{}", usage);
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
    
    let shared_query = Arc::new(config);

    let serial_files = files.clone();
    let serial_start = Instant::now();
    let serial_total = search_concurrent(serial_files, Arc::clone(&shared_query), 1);
    let serial_elapsed = serial_start.elapsed();

    let parallel_start = Instant::now();
    let parallel_total = search_concurrent(files, Arc::clone(&shared_query), thread_count);
    let parallel_elapsed = parallel_start.elapsed();

    eprintln!(
        "Single_thread (1 thread): {} matches in {:?}",
        serial_total, serial_elapsed
    );

    eprintln!(
        "Multithreaded ({} threads): {} matches in {:?}",
        thread_count, parallel_total, parallel_elapsed
    );
}
