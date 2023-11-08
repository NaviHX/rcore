const USER_LIB: &str = "../user/";
const BIN_DIR: &str = "../user/target/riscv64gc-unknown-none-elf/release/";
const SRC: &str = "./src/";
const LINK_APP_ASM: &str = "./src/link_app.asm";
const DOT: &str = ".";

fn main() {
    // build link_app.asm for the batch system
    println!("cargo:rerun-if-changed={USER_LIB}");
    println!("cargo:rerun-if-changed={SRC}");
    build_link_app();
}

use std::io::Write;

fn build_link_app() {
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(LINK_APP_ASM)
        .unwrap();

    let mut apps: Vec<_> = std::fs::read_dir(format!("{USER_LIB}/src/bin"))
        .unwrap()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find(DOT).unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    apps.sort();

    writeln!(
        output,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}
    "#,
        apps.len()
    )
    .unwrap();

    for i in 0..apps.len() {
        writeln!(
            output,
            r#"
        .quad app_{i}_start
        "#
        )
        .unwrap();
    }
    writeln!(output, ".quad app_{}_end", apps.len() - 1).unwrap();

    writeln!(output, r#"
    .global app_names
app_names:
        "#).unwrap();

    for app in apps.iter() {
        writeln!(output, r#"    .string "{}""#, app).unwrap();
    }

    for (i, app) in apps.iter().enumerate() {
        writeln!(
            output,
            r#"
        .section .data
        .global app_{i}_start
        .global app_{i}_end
app_{i}_start:
        .incbin "{BIN_DIR}/{app}.bin"
app_{i}_end:
        "#
        )
        .unwrap();
    }
}
