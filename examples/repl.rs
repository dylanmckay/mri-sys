use std::io::Write;

fn main() {
    unsafe { mri_sys::ruby_init() };

    loop {
        print!("cool-interpreter:8=====D -- ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("could not read from stdin");

        match &input.trim().to_lowercase()[..] {
            "exit" | "quit" => break,
            "help" => println!("-> https://www.eapservices.co.nz/"),
            _ => match mri_sys::helpers::eval(&input, mri_sys::helpers::Binding::top_level(), Some("mock-filename.rb")) {
                Ok(value) => println!("-> {:?}", value),
                Err(e) => eprintln!("ERROR, EXCEPTION RAISED: {}", e),
            },
        }
    }

    unsafe { mri_sys::ruby_cleanup(0) };
}
