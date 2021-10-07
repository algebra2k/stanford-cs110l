use std::env;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader};
use std::process;

fn wc_for_file(filename: &String) {
    let attr = metadata(filename).unwrap();
    let file = File::open(filename).unwrap();
    let lines = BufReader::new(file).lines();
    let mut lines_count = 0;
    let mut words_count = 0;
    for line in lines {
        let line = line.unwrap();
        let words: Vec<&str> = line.split(' ').collect();
        words_count += words.len();
        lines_count += 1;
    }

    println!(
        "\t{}\t{}\t{}\t{}",
        lines_count,
        words_count,
        attr.len(),
        filename
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    wc_for_file(filename)
}
