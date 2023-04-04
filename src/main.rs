use hp::{Parser, Template};
use html2text::render::text_renderer::{PlainDecorator, RichDecorator, TrivialDecorator};
use std::io::Write;
use zip::read::ZipArchive;

const EXTRACT_PATH: &'static str = "/tmp/rusty-html-extractor/";

#[derive(Clone, PartialEq, Eq)]
struct Options {
    width: u32,
    file_artifacts: bool,
    output_format: String,
    output_text_file: String,
    output_dir: String,
    input_file: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            width: 80,
            file_artifacts: false,
            output_format: String::from("trivial"),
            output_text_file: String::from("./html_text.txt"),
            output_dir: String::from("./rest"),
            input_file: String::from(""),
        }
    }
}

impl Options {
    fn new() -> Self {
        Default::default()
    }

    fn set_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    fn set_artifacts(mut self, a: bool) -> Self {
        self.file_artifacts = a;
        self
    }

    fn set_format(mut self, fmt: impl AsRef<str>) -> Self {
        self.output_format = fmt.as_ref().into();
        self
    }
}

#[derive(Debug)]
enum MyError {
    Io(std::io::Error),
    Msg(&'static str),
}

impl From<zip::result::ZipError> for MyError {
    fn from(_: zip::result::ZipError) -> Self {
        Self::Msg("Zip error occured")
    }
}

impl From<&'static str> for MyError {
    fn from(value: &'static str) -> Self {
        Self::Msg(value)
    }
}

impl From<std::io::Error> for MyError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<std::num::ParseIntError> for MyError {
    fn from(_: std::num::ParseIntError) -> Self {
        Self::Msg("Not a number.")
    }
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::Msg(e) => write!(f, "{}", e),
        }
    }
}

fn parse(opts: Options) -> Result<(), MyError> {
    if !std::path::PathBuf::from(opts.input_file.clone()).exists() {
        return Err("Input file does not exist".into());
    }
    let file = std::fs::File::open(opts.input_file)?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(std::path::PathBuf::from(&EXTRACT_PATH))?;
    let mut output_text = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&opts.output_text_file)?;
    std::fs::create_dir_all(opts.output_dir.clone())?;
    let output_dir = std::path::PathBuf::from(opts.output_dir);
    for i in 0..archive.len() {
        let mut archive_file = archive.by_index(i)?;
        let name = archive_file.name().to_string();
        let fname = match archive_file.enclosed_name() {
            Some(p) => p,
            None => continue,
        };

        if fname.to_str().unwrap().ends_with('/') {
            let new_path = output_dir.clone().join(fname);
            std::fs::create_dir_all(new_path)?;
        } else if String::from_utf8(
            std::process::Command::new("file")
                .arg(std::path::PathBuf::from(EXTRACT_PATH).join(&name))
                .output()?
                .stdout,
        )
        .unwrap_or("file".into())
        .to_lowercase()
        .contains("html document")
        {
            opts.file_artifacts
                .then(|| {
                    output_text
                        .write(format!("# begin {}\n", name).as_bytes())
                        .ok()
                })
                .flatten();
            let html_text = match &opts.output_format[..] {
                "trivial" => html2text::from_read_with_decorator(
                    archive_file,
                    opts.width.try_into().unwrap_or(80),
                    TrivialDecorator::new(),
                ),
                "plain" => html2text::from_read_with_decorator(
                    archive_file,
                    opts.width.try_into().unwrap_or(80),
                    PlainDecorator::new(),
                ),
                "rich" => html2text::from_read_with_decorator(
                    archive_file,
                    opts.width.try_into().unwrap_or(80),
                    RichDecorator::new(),
                ),
                _ => unreachable!(),
            };
            let txt = html_text
                .lines()
                .filter(|l| !l.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<String>>();
            output_text.write_all(txt.join("\n").as_bytes())?;
            opts.file_artifacts
                .then(|| {
                    output_text
                        .write(format!("# end {}\n", name).as_bytes())
                        .ok()
                })
                .flatten();
        } else {
            let mut outfile = std::fs::File::create(output_dir.clone().join(fname))?;
            std::io::copy(&mut archive_file, &mut outfile)?;
        }
    }
    Ok(())
}

fn main() -> Result<(), MyError> {
    let mut parser = Parser::new()
        .with_description("Extract text from zipped html files.")
        .exit_on_help(true);
    let input = parser.add_template(
        Template::new()
            .matches("-i")
            .matches("--input")
            .with_help("Input file.")
            .optional_values(false)
            .number_of_values(1),
    );
    let output = parser.add_template(
        Template::new()
            .matches("-o")
            .matches("--output")
            .with_help("Two output files. First is the path to the extracted html, second is the directory where the rest of the files will be stored. Defaults are `html_text.txt` and `rest/`")
            .number_of_values(2)
            .optional_values(false),
    );
    let wh = parser.add_template(
        Template::new()
            .matches("-w")
            .matches("--width")
            .number_of_values(1)
            .optional_values(true)
            .with_help("Allign the output text file to the given width."),
    );
    let af = parser.add_template(Template::new()
        .matches("-a")
        .matches("--artifacts")
        .number_of_values(0)
        .optional_values(true)
        .with_help("This will insert an information about what file the given snippet originated from in the output text file."));
    let ff = parser.add_template(
        Template::new()
            .matches("-f")
            .matches("--format")
            .number_of_values(1)
            .optional_values(true)
            .with_help("Given one of `plain`, `trivial`(default), `rich`, format the html in the given format."),
    );

    let res = parser.parse(None);
    match res {
        Ok(pargs) => {
            if pargs.has_with_id(input) {
                let mut opts: Options = Options::new();
                opts.input_file = pargs.get_with_id(input).unwrap().values()[0].clone();
                if pargs.has_with_id(output) {
                    let output_files = pargs.get_with_id(output).unwrap();
                    opts.output_text_file = output_files.values()[0].clone();
                    opts.output_dir = output_files.values()[1].clone();
                }
                if pargs.has_with_id(wh) {
                    opts = opts.set_width(pargs.get_with_id(wh).unwrap().values()[0].parse()?)
                }
                if pargs.has_with_id(af) {
                    opts = opts.set_artifacts(true);
                }
                if pargs.has_with_id(ff) {
                    let str = pargs.get_with_id(ff).unwrap().values()[0].clone();
                    match &str[..] {
                        "plain" => opts = opts.set_format("plain"),
                        "trivial" => opts = opts.set_format("trivial"),
                        "rich" => opts = opts.set_format("rich"),
                        _ => return Err("Unrecognized format.".into()),
                    };
                }
                if let Err(error) = parse(opts) {
                    println!("ERROR OCCURED: {}", error);
                }
                std::fs::remove_dir_all(std::path::PathBuf::from(&EXTRACT_PATH))?;
            }
        }
        Err(e) => println!("{e}"),
    }
    Ok(())
}
