// compile-flags: --edition 2021
#![allow(unused)]
fn main() {
enum Message {
    Quit,
    WriteString(String),
    Move { x: i32, y: i32 },
    ChangeColor(u8, u8, u8),
}
let message = Message::Quit;
match message {
    Message::Quit => println!("Quit"),
    Message::WriteString(write) => println!("{}", &write),
    Message::Move{ x, y: 0 } => println!("move {} horizontally", x),
    Message::Move{ .. } => println!("other move"),
    Message::ChangeColor { 0: red, 1: green, 2: _ } => {
        println!("color change, red: {}, green: {}", red, green);
    }
};
}