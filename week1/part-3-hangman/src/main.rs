// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    println!("random word: {}", secret_word);

    // Your code here! :)
    println!("Welcome to CS110L Hangman!");
    let mut counter: u32 = 0;
    let mut guess = String::new();
    let mut guessed_word_chars: Vec<char> = Vec::new();
    let mut so_far_word_chars: Vec<char> = "-".repeat(secret_word_chars.len()).chars().collect();
    loop {
        if counter == NUM_INCORRECT_GUESSES {
            break;
        }

        print_so_far_word(&so_far_word_chars);

        if guessed_word_chars.len() == 0 {
            println!("You have guessed the following letters:");
        } else {
            let s: String = guessed_word_chars.iter().collect();
            println!("You have guessed the following letters: {}", s);
        }

        println!("You have {} guessed left", NUM_INCORRECT_GUESSES - counter);

        guess.clear();
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line");

        if guess.len() != 2 {
            println!("You only input one character");
        }

        guess.truncate(guess.len() - 1);
        let input_char_vec: Vec<char> = guess.chars().collect();

        if !find_and_replace_char(
            &secret_word_chars,
            &mut guessed_word_chars,
            &mut so_far_word_chars,
            input_char_vec[0],
        ) {
            counter += 1;
        }

        if so_far_word_chars.iter().cloned().collect::<String>() == secret_word {
            println!(
                "Congratulations you guessed the secret word: {}!",
                secret_word
            );
            break;
        }
    }
}

fn print_so_far_word(so_far_word_chars: &Vec<char>) {
    println!(
        "The word so far is {}",
        so_far_word_chars.iter().cloned().collect::<String>()
    )
}

fn find_and_replace_char(
    secret_word_chars: &Vec<char>,
    guessed_word_chars: &mut Vec<char>,
    so_far_word_chars: &mut Vec<char>,
    input_char: char,
) -> bool {
    for (i, c) in secret_word_chars.iter().enumerate() {
        if *c == input_char {
            if so_far_word_chars[i] == '-' {
                so_far_word_chars[i] = *c;
                guessed_word_chars.push(input_char);
                return true;
            }
        }
    }
    return false;
}
