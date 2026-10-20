//! Renders the empty-queue image to a PNG, to iterate on how it looks:
//! `cargo run --example attempt_png -- <width> <height> <out.png>`

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (w, h): (u32, u32) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    review_queue::attempt::render(w, h).save(&args[3]).unwrap();
}
