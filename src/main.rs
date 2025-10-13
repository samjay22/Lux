//! Lux Language CLI
//!
//! Command-line interface for the Lux programming language.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

use lux_lang::{run, Lexer, VERSION};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        // No arguments: start REPL
        println!("Lux v{} - Language Interpreter", VERSION);
        println!("Type 'exit' to quit\n");
        repl();
        return;
    }

    // Check for flags
    let mut show_tokens = false;
    let mut show_help = false;
    let mut filename: Option<&String> = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--tokens" | "-t" => show_tokens = true,
            "--help" | "-h" => show_help = true,
            _ if arg.starts_with('-') => {
                eprintln!("Unknown flag: {}", arg);
                print_usage();
                process::exit(1);
            }
            _ => filename = Some(arg),
        }
    }

    if show_help {
        print_help();
        return;
    }

    if let Some(file) = filename {
        if show_tokens {
            if let Err(e) = show_file_tokens(file) {
                eprintln!("{}", e);
                process::exit(1);
            }
        } else {
            if let Err(e) = run_file(file) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    } else {
        eprintln!("Error: No input file specified");
        print_usage();
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!("Usage: lux [OPTIONS] [script]");
    eprintln!("       lux --help");
}

fn print_help() {
    println!("Lux v{} - A custom programming language", VERSION);
    println!();
    println!("USAGE:");
    println!("    lux [OPTIONS] [script]");
    println!();
    println!("OPTIONS:");
    println!("    -t, --tokens    Show tokenization output (lexer only)");
    println!("    -h, --help      Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    lux script.lux           Run a Lux script");
    println!("    lux --tokens script.lux  Show tokens from lexer");
    println!("    lux                      Start interactive REPL");
    println!();
    println!("IMPLEMENTATION STATUS:");
    println!("    ✅ Phase 1: Project Setup & Error Handling");
    println!("    ✅ Phase 2: Lexer (Tokenization)");
    println!("    ✅ Phase 3: Parser (AST Generation)");
    println!("    ⏳ Phase 4: Type System");
    println!("    ⏳ Phase 5: Semantic Analysis");
    println!("    ✅ Phase 6: Interpreter");
    println!("    ⏳ Phase 7: Async Runtime");
}

/// Run a Lux script from a file
fn run_file(filename: &str) -> Result<(), String> {
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read file '{}': {}", filename, e))?;

    run(&source, Some(filename))
        .map_err(|e| format!("{}", e))
}

/// Show tokens from lexing a file
fn show_file_tokens(filename: &str) -> Result<(), String> {
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read file '{}': {}", filename, e))?;

    let mut lexer = Lexer::new(&source, Some(filename));
    let tokens = lexer.tokenize()
        .map_err(|e| format!("{}", e))?;

    println!("Tokens for '{}':", filename);
    println!("{}", "=".repeat(60));

    for (i, token) in tokens.iter().enumerate() {
        println!("{:4}: {:20} | {:?}", i, format!("{:?}", token.token_type), token.lexeme);
    }

    println!("{}", "=".repeat(60));
    println!("Total tokens: {}", tokens.len());

    Ok(())
}

/// Start an interactive REPL (Read-Eval-Print Loop)
fn repl() {
    let mut line_number = 1;

    loop {
        print!("lux:{} > ", line_number);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = input.trim();
                
                if input == "exit" || input == "quit" {
                    break;
                }

                if input.is_empty() {
                    continue;
                }

                // Run the input
                if let Err(e) = run(input, Some("<repl>")) {
                    eprintln!("{}", e);
                }

                line_number += 1;
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    println!("\nGoodbye!");
}

