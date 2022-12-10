#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io;
use std::path::PathBuf;
use path_dedot::ParseDot;
use regex::Regex;

use parse_rules::ParseRules;

pub mod marked_text;
pub mod id_based_vec;
pub mod parse_rules;

use marked_text::MarkedText;
use crate::glsl_expand::parse_rules::SameIncludes;


#[derive(Debug, Clone)]
pub struct ShaderFile {
    content: MarkedText<PathBuf>,

    #[allow(dead_code)]
    warnings: Vec<Warning>,
}
impl ShaderFile {
    pub fn current_text(&self) -> &String { &self.content.text() }
}

#[derive(Debug)]
pub enum ContextInitError {
    UnableToFindExePath { io_error: io::Error },
    NoExeParentPath,
    PathBufParseErr { io_error: io::Error },
    RegexCompileError { error: regex::Error },
}
pub struct ShaderContext {
    main_dir: PathBuf,
    data: HashMap<PathBuf, ShaderFile>,

    def_parse_rules: ParseRules,

    include_regex: Regex,
    comment_regexes: Vec<(Regex, usize)>,
}
impl ShaderContext {
    pub fn new() -> Result<ShaderContext, ContextInitError> {
        let exe_path = std::env::current_exe()
            .map_err(|io_error| ContextInitError::UnableToFindExePath { io_error })?;

        let exe_path = exe_path.parent()
            .ok_or(ContextInitError::NoExeParentPath)?;

        Self::_from_dir(exe_path.to_path_buf())
    }

    pub fn from_dir<P: Into<PathBuf>>(dir: P) -> Result<ShaderContext, ContextInitError> {
        let dir: PathBuf = dir.into();
        if dir.is_absolute() {
            Self::_from_dir(dir)
        } else {
            let mut context = ShaderContext::new()?;

            context.main_dir = context.main_dir
                .join(dir).parse_dot()
                .map_err(|io_error| ContextInitError::PathBufParseErr{io_error})?
                .into_owned();
            Ok(context)
        }
    }

    fn _from_dir(dir: PathBuf) -> Result<ShaderContext, ContextInitError> {
        let include_regex =
            Regex::new(r#"\s*(#(?:pragma)? ?include *[ <"](?P<filename>[^\n\r"<>]*)[>"\n\r]?)"#)
                .map_err(|error| ContextInitError::RegexCompileError { error } )?;

        let comment_expressions = [
            ("/\\*.*\\*/", 0usize),
            ("(//.*)\n?", 1),
            ("#\\[del\\][^#]*#", 0)
        ];
        let mut comment_regexes: Vec<(Regex, usize)> = Vec::with_capacity(comment_expressions.len());
        for (expr, match_id) in comment_expressions.into_iter() {
            comment_regexes.push((
                Regex::new(expr).map_err(|error| ContextInitError::RegexCompileError { error } )?,
                match_id
                ));
        }



        Ok(ShaderContext {
            main_dir: dir,
            data: HashMap::new(),
            def_parse_rules: ParseRules::new(),

            include_regex,
            comment_regexes,
        })
    }

    pub fn set_main_dir<P: Into<PathBuf>>(&mut self, dir: P) -> io::Result<()> {
        self.main_dir = self.to_absolute(dir)?;
        Ok(())
    }


    // Main functionality
    pub fn get_file_processed<P: Into<PathBuf>>(&mut self, path: P) -> Result<&ShaderFile, ExpandError> {
        let path_buf = path.into();
        let path = self.to_absolute(path_buf.clone())
            .map_err(|io_error| self.err_path_parse_error(path_buf, io_error) )?;

        self._get_file_processed(&path, ParseLog::from_rules(self.def_parse_rules.clone()))
    }

    fn _get_file_processed(&mut self, path: &PathBuf, log: ParseLog) -> Result<&ShaderFile, ExpandError> {
        if self.data.contains_key(path) {
            let file = self.data.get(path).unwrap();
            Ok(file)
        } else {
            self._load_file(path, log)
        }
    }

    fn _load_file(&mut self, path: &PathBuf, log: ParseLog) -> Result<&ShaderFile, ExpandError> {
        let mut log = log;

        let file_text = self.preprocess_text(self.read_file(path.clone())?, &mut log)?;
        let mut file_text = self.find_replaces(file_text, path)?;
        log.file(path.clone());


        if file_text.marks().empty() {
            let shader_file = ShaderFile {
                content: file_text,
                warnings: log.warnings,
            };
            self.data.insert(path.clone(), shader_file);
            return Ok(self.data.get(path).unwrap());
        }

        let initial_replaces = file_text.marks().current_elements();
        for id in initial_replaces {
            let replace_filepath = file_text.marks().get(id)
                .ok_or(self.err_text_expanding_error(path.clone()))?
                .flag();

            let slice = &log.files_hierarchy[..(log.files_hierarchy.len()-1)];
            let _ = self.check_recursion(&replace_filepath, slice, path)?;

            let replace_to  = self._get_file_processed(replace_filepath, log.no_warns())?;

            file_text.replace_mark_content(id, replace_to.content.clone());
        }
        file_text = self.remove_repeats(path, file_text, &mut log);
        file_text = self.postprocess_text(file_text, &mut log)?;

        let shader_file = ShaderFile {
            content: file_text,
            warnings: log.warnings,
        };
        self.data.insert(path.clone(), shader_file);

        Ok(self.data.get(path).unwrap())
    }

    fn check_recursion(&self, check_file: &PathBuf, prev_files: &[PathBuf], origin_file: &PathBuf) -> Result<(), ExpandError> {
        for (i, prev) in prev_files.into_iter().enumerate() {
            if check_file == prev {
                let mut prev_files = prev_files[i..].to_vec();
                prev_files.push(origin_file.clone());
                return Err(self.err_infinite_recursion(prev_files));
            }
        }
        Ok(())
    }


    fn preprocess_text(&self, text: String, _log: &mut ParseLog) -> Result<String, ExpandError> {
        let mut comments = MarkedText::<()>::new(text);
        for (regex, match_id) in self.comment_regexes.iter() {
            let marks_to_place: Vec<(usize, usize)> = regex
                .captures_iter(comments.text())
                .map(|cap| (cap.get(*match_id).unwrap().start(), cap.get(*match_id).unwrap().end()))
                .collect();

            marks_to_place
                .into_iter()
                .for_each(|(start, end)| { comments.set_mark((), start, end); })
        }

        let elements_to_delete = comments.marks().current_elements();
        for e in elements_to_delete {
            comments.delete_mark_and_content(e);
        }

        Ok(comments.text_move())
    }
    fn postprocess_text(&self, text: MarkedText<PathBuf>, _log: &mut ParseLog) -> Result<MarkedText<PathBuf>, ExpandError> {
        Ok(text)
    }

    fn remove_repeats(&self, file: &PathBuf, file_text: MarkedText<PathBuf>, log: &mut ParseLog) -> MarkedText<PathBuf> {
        let rule = log.parse_rules.same_includes().value();
        if rule == SameIncludes::IgnoreAll {
            return file_text;
        }

        let mut file_text = file_text;
        let current_marks = file_text.marks().current_elements();

        for id in current_marks {
            let include = file_text.marks().get(id);
            if include.is_none() { continue; }
            let cur_include = include.unwrap();

            let mut same =
                file_text.marks().find_elements(|m| m.flag() == cur_include.flag());

            if same.len() <= 1 { continue; }
            //Сортировка по позиции в тексте. Самые ранние находятся в начале
            same.sort_by(|m1, m2|
                file_text.marks().get(*m1).unwrap().start().cmp(
                    &file_text.marks().get(*m2).unwrap().start())
            );

            match rule {
                SameIncludes::DeleteRepeats => {
                    let warn = Warning::MultipleSameIncludes {
                        main_file: file.clone(),
                        included_file: cur_include.flag().clone(),
                        times: same.len(),
                        action_done: SameIncludes::DeleteRepeats
                    };

                    let mut same_iter = same.iter();
                    let _ = same_iter.next();
                    for id in same_iter {
                        file_text.delete_mark_content(*id);
                        file_text.remove_mark(*id, false);
                    }

                    self.warn(warn, log);
                }
                SameIncludes::ThrowAnError => {

                }
                _ => {}
            }
        }

        file_text
    }

    fn find_replaces(&self, text: String, filepath: &PathBuf) -> Result<MarkedText<PathBuf>, ExpandError> {
        let main_file_parent = filepath.parent()
            .ok_or( self.err_unable_to_get_file_parent(filepath.clone()) )?;

        let mut replaces: MarkedText<PathBuf> = MarkedText::new(text.clone());

        for cap in self.include_regex.captures_iter(&text) {
            let full_match = cap.get(1).unwrap();
            let filename = cap.get(2).unwrap();

            let filepath_not_parsed = main_file_parent.join(PathBuf::from(filename.as_str()));
            let filepath = filepath_not_parsed.parse_dot();

            match filepath {
                Ok(path) =>
                    replaces.set_mark(path.into_owned(), full_match.start(), full_match.end()),
                Err(io_error) => return Err(
                    self.err_path_parse_error(PathBuf::from(filename.as_str()), io_error)
                ),
            };
        }
        Ok(replaces)
    }

    // Utility functions
    fn read_file(&self, path: PathBuf) -> Result<String, ExpandError> {
        let string = std::fs::read_to_string(path.clone())
            .map_err(|io_error| match io_error.kind() {
                io::ErrorKind::NotFound => self.err_file_not_found(path),
                _ => self.err_file_read_error(path, io_error),
            })?;
        Ok(string.replace("\r",""))
    }

    fn get_relative_path(&self, path: PathBuf) -> PathBuf {
        get_relative_path(self.main_dir.clone(), path)
    }
    fn get_relative_path_string(&self, path: PathBuf) -> String {
        let relative = self.get_relative_path(path);
        path_to_string_guaranteed(&relative)
    }

    fn to_absolute<P: Into<PathBuf>>(&self, path: P) -> io::Result<PathBuf> {
        let path: PathBuf = path.into();
        if path.is_absolute() {
            Ok(path.parse_dot()?.into_owned())
        } else {
            Ok(self.main_dir.join(path).parse_dot()?.into_owned())
        }
    }
}
impl ShaderContext {
    // Errors
    fn err_path_parse_error(&self, path: PathBuf, io_error: io::Error) -> ExpandError {
        ExpandError::PathParseError { path: self.get_relative_path(path), io_error }
    }
    fn err_file_not_found(&self, path: PathBuf) -> ExpandError {
        ExpandError::FileNotFound{ path: self.get_relative_path(path) }
    }
    fn err_file_read_error(&self, path: PathBuf, io_error: io::Error) -> ExpandError {
        ExpandError::FileReadError { path: self.get_relative_path(path), io_error }
    }
    fn err_infinite_recursion(&self, files: Vec<PathBuf>) -> ExpandError {
        let formatted = files.iter()
            .map(|p| self.get_relative_path(p.clone()))
            .collect();
        ExpandError::InfiniteRecursion{ files: formatted }
    }
    fn err_unable_to_get_file_parent(&self, file: PathBuf) -> ExpandError {
        ExpandError::UnableToGetFileParent { file: self.get_relative_path(file) }
    }
    fn err_text_expanding_error(&self, filepath: PathBuf) -> ExpandError {
        ExpandError::TextExpandingError{ filepath: self.get_relative_path(filepath) }
    }
}
impl ShaderContext {
    fn warn(&self, warn: Warning, _log: &mut ParseLog) {
        match &warn {
            Warning::MultipleSameIncludes { main_file, included_file, times, action_done } => {
                println!("GLSL Expand | \x1b[93mWarning\x1b[0m: File {} was included \x1b[93m{} times\x1b[0m in file {}",
                         self.get_relative_path_string(included_file.clone()),
                         times,
                         self.get_relative_path_string(main_file.clone())
                );
                match action_done {
                    SameIncludes::DeleteRepeats => println!("Every include but first was deleted"),
                    _ => {}
                }
                println!("To disable this warning, please set default behaviour using")
            }
        }
    }
}


#[derive(Debug)]
pub enum ExpandError {
    PathParseError      { path: PathBuf, io_error: io::Error },
    FileNotFound            { path: PathBuf },
    FileReadError       { path: PathBuf, io_error: io::Error },
    InfiniteRecursion   { files: Vec<PathBuf> },

    UnableToGetFileParent { file: PathBuf },
    TextExpandingError { filepath: PathBuf },

    MultipleSameIncludes {
        main_file: PathBuf,
        included_file: PathBuf,
        times: usize,
    }
}
impl Display for ExpandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpandError::PathParseError { path, io_error } => {
                f.write_str( &format!("ExpandError - Unable to parse path: \"{}\" because of {:?}",
                                      path_to_string_guaranteed(path), io_error)  )?
            }
            ExpandError::FileNotFound{ path } => {
                f.write_str(&format!("File not found: \"{}\"", path_to_string_guaranteed(path)))?;
            },
            ExpandError::FileReadError{ path, io_error } => {
                f.write_str(&format!("Failed to read file \"{}\" because of: {:?}",
                                     path_to_string_guaranteed(path), io_error))?;
            },
            ExpandError::InfiniteRecursion{ files } => {
                if files.len() == 0 {
                    f.write_str(&format!("Infinite recursion error (no data)"))?;
                } else {
                    f.write_str(&format!("Infinite recursion error:\n"))?;
                    for e in files {
                        f.write_str(&format!("{} -> ", path_to_string_guaranteed(e)))?;
                    }
                    f.write_str(&format!("{}", path_to_string_guaranteed(&files[0]) ))?;
                }
            },
            ExpandError::UnableToGetFileParent { file } => {
                f.write_str( &format!("ExpandError - Unable to get file's parent: \"{}\"",
                                      path_to_string_guaranteed(file))  )?
            }
            ExpandError::TextExpandingError{ filepath } => {
                f.write_str(&format!("Failed to expand text: \"{}\"", path_to_string_guaranteed(filepath)))?;
            },
            ExpandError::MultipleSameIncludes { main_file, included_file, times } => {
                f.write_str(&format!(
                    "File {} was included {} times in file {}",
                    path_to_string_guaranteed(included_file),
                    times,
                    path_to_string_guaranteed(main_file)
                ))?;
            }
        }
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub enum Warning {
    MultipleSameIncludes {
        main_file: PathBuf,
        included_file: PathBuf,
        times: usize,
        action_done: SameIncludes,
    },
}

#[derive(Debug, Clone)]
pub struct ParseLog {
    warnings: Vec<Warning>,
    files_hierarchy: Vec<PathBuf>,
    parse_rules: ParseRules,
}
impl ParseLog {
    pub fn new() -> ParseLog {
        ParseLog {
            warnings: Vec::new(),
            files_hierarchy: Vec::new(),
            parse_rules: ParseRules::new(),
        }
    }
    pub fn from_rules(parse_rules: ParseRules) -> ParseLog {
        ParseLog {
            warnings: Vec::new(),
            files_hierarchy: Vec::new(),
            parse_rules,
        }
    }

    pub fn file(&mut self, file: PathBuf) {
        self.files_hierarchy.push(file);
    }
    pub fn warn(&mut self, warn: Warning) {
        self.warnings.push(warn);
    }
    pub fn warns(&mut self, warns: Vec<Warning>) {
        for w in warns {
            self.warn(w);
        }
    }

    pub fn no_warns(&self) -> ParseLog {
        ParseLog {
            warnings: Vec::new(),
            files_hierarchy: self.files_hierarchy.clone(),
            parse_rules: self.parse_rules.clone(),
        }
    }

    pub fn rules(&self) -> &ParseRules {
        &self.parse_rules
    }
    pub fn rules_mut(&mut self) -> &mut ParseRules {
        &mut self.parse_rules
    }
}


fn path_to_string_guaranteed(path: &PathBuf) -> String {
    match path.to_str() {
        Some(s) => s.to_string(),
        None => format!("{:?}", path),
    }
}

pub fn get_relative_path(abs_from: PathBuf, abs_to: PathBuf) -> PathBuf {
    if let Ok(res) = abs_to.strip_prefix(abs_from.clone()) {
        return res.to_path_buf();
    }

    let mut different = false;
    let mut abs_to_iter = abs_to.iter();

    let mut base_path = abs_from.clone();
    let mut result_path = PathBuf::new();

    for f in abs_from.iter() {
        let t = abs_to_iter.next();
        if Some(f) != t  { different = true; }

        if different {
            result_path.push(PathBuf::from(".."));
            base_path.pop();
        }
    }
    result_path.push(abs_to.strip_prefix(base_path.clone()).unwrap());
    result_path
}