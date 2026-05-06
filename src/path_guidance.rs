use std::env;
use std::path::Path;

use anyhow::Result;

use crate::paths::Paths;

pub fn run() -> Result<()> {
    let paths = Paths::from_env()?;
    print_guidance(&paths);
    Ok(())
}

pub fn print_guidance(paths: &Paths) {
    let bin = paths.managed_bin_dir();
    println!(
        "Managed llm-wiki binary: {}",
        paths.managed_binary().display()
    );
    println!("Add this directory to PATH for terminal convenience:");
    for line in guidance_lines(&bin) {
        println!("{line}");
    }
}

fn guidance_lines(bin: &Path) -> Vec<String> {
    let Some(shell) = env::var_os("SHELL") else {
        return all_shell_lines(bin);
    };
    let shell = Path::new(&shell)
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    match shell.as_ref() {
        "zsh" => vec![zsh_line(bin)],
        "bash" => vec![bash_line(bin)],
        "fish" => vec![fish_line(bin)],
        _ => all_shell_lines(bin),
    }
}

fn all_shell_lines(bin: &Path) -> Vec<String> {
    vec![zsh_line(bin), bash_line(bin), fish_line(bin)]
}

fn zsh_line(bin: &Path) -> String {
    format!(
        "zsh:  echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc",
        bin.display()
    )
}

fn bash_line(bin: &Path) -> String {
    format!(
        "bash: echo 'export PATH=\"{}:$PATH\"' >> ~/.bashrc",
        bin.display()
    )
}

fn fish_line(bin: &Path) -> String {
    format!("fish: fish_add_path {}", bin.display())
}
