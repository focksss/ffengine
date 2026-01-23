#![warn(unused_qualifications)]
use ffengine::engine::Engine;

fn main() {
    let mut app = Engine::new("editor/resources/editor/scene");

    println!("starting");

    app.run()
}