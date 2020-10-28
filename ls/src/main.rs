use std::{
    fs,
    io::{self, Write},
    path, process,
    string::String,
    time::SystemTime,
};

use pad::{Alignment, PadStr};

extern crate chrono;

mod cli;
mod file;
mod flags;

use file::File;
use flags::Flags;

fn main() -> io::Result<()> {
    let matches = cli::create_app().get_matches();

    let files = matches.values_of("FILE").unwrap();
    let flags = Flags::from_matches(&matches);

    let mut exit_code = 0;

    let mut writer: Box<dyn Write> = Box::new(io::stdout());

    let multiple = files.len() > 1;

    for file in files {
        match fs::read_dir(file) {
            Ok(dir) => {
                let mut dir: Vec<_> = dir
                    // Collect information about the file or directory
                    .map(|entry| File::from(entry.unwrap().path(), flags).unwrap())
                    // Hide hidden files and directories if `-a` or `-A` flags
                    // weren't provided
                    .filter(|file| !File::is_hidden(&file.name) || flags.show_hidden())
                    .collect();

                if !flags.no_sort {
                    if flags.time {
                        if flags.last_accessed {
                            dir.sort_by_key(sort_by_access_time);
                        } else {
                            dir.sort_by_key(sort_by_time);
                        }
                        dir.reverse();
                    } else if flags.sort_size {
                        dir.sort_by_key(sort_by_size);
                        dir.reverse();
                    } else {
                        // Sort the directory entries by file name by default
                        dir.sort_by_key(sort_by_name);
                    }

                    if flags.reverse {
                        dir.reverse();
                    }
                }

                if flags.all || flags.no_sort {
                    // Retrieve the current directories information. This must
                    // be canonicalize incase the path is relative
                    let current = path::PathBuf::from(file).canonicalize().unwrap();

                    let dot = File::from_name(".".to_string(), current.clone(), flags)
                        .expect("Failed to read .");

                    // Retrieve the parent path. Default to the current path if the parent doesn't
                    // exist
                    let parent_path =
                        path::PathBuf::from(dot.path.parent().unwrap_or_else(|| current.as_path()));

                    let dot_dot = File::from_name("..".to_string(), parent_path, flags)
                        .expect("Failed to read ..");

                    dir.insert(0, dot);
                    dir.insert(1, dot_dot);
                }

                if multiple {
                    writeln!(writer, "\n{}:", file)?;
                }

                if !flags.comma_separate && flags.show_list() {
                    if print_list(dir, &mut writer, flags).is_err() {
                        exit_code = 1
                    }
                } else if print_default(dir, &mut writer, flags).is_err() {
                    exit_code = 1;
                }
            },
            Err(err) => {
                eprintln!("ls: cannot access '{}': {}", file, err);
                exit_code = 1;
            },
        }
    }

    if exit_code != 0 {
        process::exit(exit_code);
    }

    Ok(())
}

/// Prints information about a file in the default format
fn print_default<W: Write>(files: Vec<File>, writer: &mut W, flags: Flags) -> io::Result<()> {
    for file in files {
        let file_name = file.file_name();

        if flags.comma_separate {
            write!(writer, "{}, ", file_name)?;
        } else {
            writeln!(writer, "{}", file_name)?;
        }
    }
    if flags.comma_separate {
        writeln!(writer)?;
    }

    Ok(())
}

/// Prints information about the provided file in the long (`-l`) format
fn print_list<W: Write>(files: Vec<File>, writer: &mut W, flags: Flags) -> io::Result<()> {
    let mut inode_width = 1;
    let mut block_width = 1;
    let mut hard_links_width = 1;
    let mut user_width = 1;
    let mut group_width = 1;
    let mut size_width = 1;

    for file in &files {
        if flags.inode {
            let inode = file.inode().len();

            if inode > inode_width {
                inode_width = inode;
            }
        }

        if flags.size {
            let block = file.blocks().len();

            if block > block_width {
                block_width = block;
            }
        }

        let hard_links = file.hard_links().len();

        if hard_links > hard_links_width {
            hard_links_width = hard_links;
        }

        let user: usize;

        match file.user() {
            Ok(file_user) => {
                user = file_user.len();
            },
            Err(err) => {
                eprintln!("ls: {}", err);
                process::exit(1);
            },
        }

        if user > user_width {
            user_width = user;
        }

        if !flags.no_owner {
            let group: usize;

            match file.group() {
                Ok(file_group) => {
                    group = file_group.len();
                },
                Err(err) => {
                    eprintln!("ls: {}", err);
                    process::exit(1);
                },
            }

            if group > group_width {
                group_width = group;
            }
        }

        let size = file.size().len();

        if size > size_width {
            size_width = size;
        }
    }

    for file in &files {
        if flags.inode {
            write!(
                writer,
                "{} ",
                file.inode().pad_to_width_with_alignment(inode_width, Alignment::Right)
            )?;
        }

        if flags.size {
            write!(
                writer,
                "{} ",
                file.blocks().pad_to_width_with_alignment(block_width, Alignment::Right)
            )?;
        }

        write!(writer, "{} ", file.permissions())?;

        write!(
            writer,
            "{} ",
            file.hard_links().pad_to_width_with_alignment(hard_links_width, Alignment::Right)
        )?;

        match file.user() {
            Ok(user) => {
                write!(writer, "{} ", user.pad_to_width(user_width))?;
            },
            Err(err) => {
                eprintln!("ls: {}", err);
                process::exit(1);
            },
        }

        if !flags.no_owner {
            match file.group() {
                Ok(group) => {
                    write!(writer, "{} ", group.pad_to_width(group_width))?;
                },
                Err(err) => {
                    eprintln!("ls: {}", err);
                    process::exit(1);
                },
            }
        }

        write!(
            writer,
            "{} ",
            file.size().pad_to_width_with_alignment(size_width, Alignment::Right)
        )?;

        write!(writer, "{} ", file.time()?)?;

        write!(writer, "{}", file.file_name())?;

        writeln!(writer)?;
    }

    Ok(())
}

/// Sort a list of files by last accessed time
fn sort_by_access_time(file: &File) -> SystemTime { file.metadata.accessed().unwrap() }

/// Sort a list of files by file name alphabetically
fn sort_by_name(file: &File) -> String { file.name.to_lowercase() }

/// Sort a list of files by size
fn sort_by_size(file: &File) -> u64 { file.metadata.len() }

/// Sort a list of directories by modification time
fn sort_by_time(file: &File) -> SystemTime { file.metadata.modified().unwrap() }
