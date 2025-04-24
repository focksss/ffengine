use std::io;
use std::cmp::Ordering; // compare, ordering
use rand::Rng;

fn times_two(x: i32) -> i32 {
    2*x
}
fn main() {
    let secret_number = rand::rng().random_range(1..=100);

    println!("(the secret number is {})", secret_number);

    println!("Guess the number!");

    println!("Please input your guess.");

    loop {
        let mut guess = String::new();

        io::stdin()
            .read_line(&mut guess)
            .expect("Failed to read line");

        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };

        println!("You guessed: {}", guess);
        match guess.cmp(&secret_number) {
            Ordering::Less => println!("Too small!"),
            Ordering::Greater => println!("Too big!"),
            Ordering::Equal => {
                println!("You win!");
                break;
            },
        }
    }
}