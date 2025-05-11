extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico"); 
        match res.compile() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to compile Windows resources: {}", e);
                panic!("{}", e.to_string());
            }
        }
    }
}