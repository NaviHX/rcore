use std::fs;
use std::env;
use std::collections::HashMap;
use std::io::Write;

fn main() {
    println!("cargo:rerun-if-changed=src/linker.ld");
    println!("cargo:rerun-if-env-changed=BASE_ADDRESS");
    generate_link_script();
}

fn generate_link_script() {
    let env_context: HashMap<String, String> = env::vars().collect();
    let link_script_pattern = fs::read_to_string("src/linker.ld").unwrap();

    let linker_script = envsubst::substitute(link_script_pattern, &env_context).unwrap();
    let mut output = fs::OpenOptions::new().create(true).write(true).open("src/tmp-linker.ld").unwrap();
    output.write_all(linker_script.as_bytes()).unwrap();
}
