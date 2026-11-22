# SearchToolCLI

Speedy, multithreaded command line(CLI) search utility written in Rust for Windows. `SearchToolCLI` includes recursive traversal of directories, performs fast regex free substring & pipe delimited pattern matching, measures the single threaded vs multithreaded execution speed, with an installer for PowerShell, initialising `s4` / `S4` shortcut with structured PowerShell `Format-Table` output on first launch.

## Features

* **Multithreaded File Scanning:** Scales working threads dynamically using `std::thread::available_parallelism()` and shares a read only search configuration safely across threads with `Arc<SearchConfig>`.

* **Case-Insensitive Matching (`-i`):** Support for both case sensitive and case insensitive searches without regular expression overhead.

* **Pipe-Delimited Alternation (`|`):** Matches multiple patterns in just a single pass (e.g., `"attack|network|work"`), reporting each matching line only once.

* **Built-In Performance Benchmarking:** Compares a single threaded (`1` thread) vs multithreaded (`N` threads) execution time with `std::time::Instant` and prints the metrics to `stderr`.

* **Automatic PowerShell `s4` / `S4` Integration:** On the first launch on Windows, `SearchToolCLI` will install the `s4` shortcut functionality into your PowerShell `$PROFILE` (`WindowsPowerShell\Microsoft.PowerShell_profile.ps1`) so that it can format search results cleanly via `Format-Table`.

## Project Structure

```text
SearchToolCLI/
├── src/
│   └── main.rs          # CLI argument parser, directory crawler, concurrent search, and PS profile installer
├── install.ps1          # One-command release build and PowerShell profile setup script
├── Cargo.toml           # Crate manifest (zero external dependencies)
├── Cargo.lock
└── README.md
```

## Prerequisites

* **Rust Toolchain:** `rustc` and `cargo` (2021 edition or newer)

* **Operating System:** Windows 10 / 11 with PowerShell 5.1+ (for automatic `s4` / `S4` profile integration)

## Build & Installation

### Quick Install (Recommended)

Clone the repository and dot-source the included PowerShell installer:

```powershell
git clone <your-repo-url>
cd SearchToolCLI
. .\install.ps1
```

> **Important:** Include the leading dot and space (`. .\install.ps1`). This runs the script in your current session so `s4` and `S4` are available immediately without restarting PowerShell.

The installer automatically:
1. Compiles the optimized release binary (`.\target\release\SearchToolCLI.exe`).
2. Runs `SearchToolCLI.exe --install` to register the `s4` (and case-insensitive `S4`) table-formatted function in your PowerShell `$PROFILE`.
3. Reloads `$PROFILE` so `s4` works right away in your current terminal and in every future PowerShell session from any directory.

*(If you run `.\install.ps1` without the leading dot, the build and profile setup still succeed—just open a new PowerShell window or run `. $PROFILE` once to activate `s4`.)*

### Manual Build & First-Launch Setup

If you prefer to build manually without the script:

```powershell
cargo build --release
.\target\release\SearchToolCLI.exe --install
. $PROFILE
```

Running `.\target\release\SearchToolCLI.exe` directly on any search also triggers the first-launch profile installer automatically.

> **Note:** On first launch, `SearchToolCLI` delegates the profile write to `powershell.exe` so it works cleanly even when Windows Security Controlled Folder Access restricts direct writes to the `Documents` folder. Subsequent runs check for the `# >>> SearchToolCLI s4/S4 <<<` marker and skip the installer completely.

## Usage

### Using the PowerShell `s4` / `S4` Shortcut (Recommended)

The `s4` function wraps `SearchToolCLI.exe` and formats every match into a wrapped table (`File`, `Line`, `Text`):

```powershell
# Case-sensitive search
s4 "search-term/pattern" "C:\User\Example\Path\DeskTop"

# Case-insensitive search (-i)
s4 -i "SEARCH-TERM/PATTERN" "C:\USER\EXAMPLE\PATH\DOWNLOADS"

# Pipe-delimited OR alternation with case-insensitivity
S4 -i "attack|network|work" "C:\USER\EXAMPLE\PATH"
```

**Example Table Output:**

```text
File                                   Line Text
----                                   ---- ----
Anonymous Networking.txt                  5 Malicious and Colluding Relays (Sybil Attacks)
The attacks detailed in your study.txt    1 The attacks detailed in your study guide...
```

### Running the Binary Directly

You can also invoke `SearchToolCLI.exe` directly for raw `path:line:content` output:

```powershell
.\target\release\SearchToolCLI.exe [-i] "<pattern>" "<file_or_dir>"
```

To inspect only the single-threaded vs. multithreaded timing metrics on `stderr`, redirect `stdout` to `$null`:

```powershell
.\target\release\SearchToolCLI.exe -i "SEARCHCONFIG|PUB FN" .\bench_tmp > $null
```

## Performance Benchmark

Tested on Windows (8 logical threads) across a synthetic dataset of **500 files / 100,000 lines** using `-i "SEARCHCONFIG|PUB FN"`:

| Execution Mode | Matches Found | Elapsed Time | Speedup |
| ----- | ----- | ----- | ----- |
| Single-threaded (`1` thread) | 2,500 | `138.59 ms` | 1.00x (baseline) |
| Multithreaded (`8` threads) | 2,500 | `33.01 ms` | **~4.20x** |

## How It Works

1. **Argument Parsing (`parse_args`):** Extracts the optional `-i` flag, search pattern, and target path from `std::env::args()`.

2. **Recursive Traversal (`collect_files`):** Walks the target path and collects all regular file `PathBuf` entries.

3. **Chunked Parallel Search (`search_concurrent`):** Splits the file list into balanced chunks across worker threads, reads files line-by-line via `BufReader`, and evaluates patterns with zero external regex dependencies.

4. **Profile Automation (`ensure_powershell_shortcuts`):** Checks for the `# >>> SearchToolCLI s4/S4 <<<` marker in `Microsoft.PowerShell_profile.ps1` and appends the `s4` wrapper function once via `powershell.exe` if absent.

## Study Reference

I have added to the repo is a copy of **_The Rust Programming Language_** by Steve Klabnik and Carol Nichols, with contributions from the Rust Community. Provided as a reference with explanations of some of the core Rust concepts used in this project—including ownership, borrowing, error handling (`Result` / `io::Result`), `Arc` for thread safe shared state and multithreaded concurrency using `std::thread`.