use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings::ColoredHelp, Arg,
};

pub(crate) fn create_app<'a, 'b>() -> App<'a, 'b> {
    App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .help_message("Display help information.")
        .version_message("Display version information.")
        .help_short("?")
        .settings(&[ColoredHelp])
        .arg(Arg::with_name("FILE").help("File(s) to use.").multiple(true))
        .arg(
            Arg::with_name("bytes")
                .help("Display the byte counts.")
                .long("bytes")
                .short("c")
                .value_name("N")
                .conflicts_with("lines"),
        )
        .arg(
            Arg::with_name("lines")
                .help("Display the newline counts.")
                .long("lines")
                .short("n")
                .value_name("N")
                .default_value("10"),
        )
}
