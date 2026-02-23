// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::File,
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
};
use walkdir::DirEntry;

use crate::DynError;

const REQUIRED_TEXT: &str = "Licensed under the Apache-2.0 license";
const EXTENSIONS: &[&str] = &[
    "rs", "h", "c", "cpp", "cc", "toml", "sh", "py", "ld", "go", "yml", "yaml", "S", "s",
];
const IGNORED_PATHS: &[&str] = &[
    ".github/dependabot.yml",
    "target/",
    "docs/book.toml",
    "docs/src/",
];
const IGNORED_DIRS: &[&str] = &[".git", "target", "out", "dist", "book"];

pub(crate) fn fix() -> Result<(), DynError> {
    println!("Running: license header fix");

    let project_root = crate::project_root();
    let files = find_files(&project_root, EXTENSIONS, false)?;
    let mut failed = false;

    for file in files.iter() {
        if check_file(file).is_err() {
            println!("Fixing header in {}", remove_root(file, &project_root));
            fix_file(file)?;
        }
        if let Err(e) = check_file(file) {
            println!("{e}");
            failed = true;
        }
    }

    if failed {
        return Err("License header fix failed; please fix the above files manually.".into());
    }

    Ok(())
}

pub(crate) fn check() -> Result<(), DynError> {
    println!("Running: license header check");

    let project_root = crate::project_root();
    let files = find_files(&project_root, EXTENSIONS, false)?;
    let mut failed = false;

    for file in files.iter() {
        if let Err(e) = check_file(file) {
            println!("{e}");
            failed = true;
        }
    }

    if failed {
        return Err("Some files failed to have the correct license header; to fix, run \"cargo xtask header-fix\" from the repo root".into());
    }

    Ok(())
}

fn remove_root(path: &Path, project_root: &Path) -> String {
    let root = project_root.to_str().unwrap_or_default().to_owned() + "/";
    let path = path.to_str().unwrap_or_default();
    path.strip_prefix(&root).unwrap_or(path).into()
}

fn add_path_walkdir_error<'a>(
    path: &'a Path,
    project_root: &'a Path,
) -> impl Fn(walkdir::Error) -> Error + Copy + 'a {
    move |e: walkdir::Error| {
        let path = remove_root(path, project_root);
        match e.io_error() {
            Some(e) => Error::new(e.kind(), format!("{path:?}: {e}")),
            None => Error::other(format!("{path:?}: {e}")),
        }
    }
}

fn add_path<'a>(path: &'a Path, project_root: &'a Path) -> impl Fn(Error) -> Error + Copy + 'a {
    move |e: Error| {
        let path = remove_root(path, project_root);
        Error::new(e.kind(), format!("{path:?}: {e}"))
    }
}

fn check_file_contents(
    path: &Path,
    contents: impl BufRead,
    project_root: &Path,
) -> Result<(), Error> {
    const N: usize = 3;
    let wrap_err = add_path(path, project_root);

    for line in contents.lines().take(N) {
        if line.map_err(wrap_err)?.contains(REQUIRED_TEXT) {
            return Ok(());
        }
    }
    let path = remove_root(path, project_root);
    Err(Error::other(format!(
        "File {path:?} doesn't contain {REQUIRED_TEXT:?} in the first {N} lines"
    )))
}

fn check_file(path: &Path) -> Result<(), Error> {
    let project_root = crate::project_root();
    let wrap_err = add_path(path, &project_root);
    check_file_contents(
        path,
        BufReader::new(File::open(path).map_err(wrap_err)?),
        &project_root,
    )
}

fn fix_file(path: &Path) -> Result<(), Error> {
    let project_root = crate::project_root();
    let wrap_err = add_path(path, &project_root);

    let mut contents = Vec::from(match path.extension().and_then(|s| s.to_str()) {
        Some("rs" | "h" | "c" | "cpp" | "cc" | "go") => format!("// {REQUIRED_TEXT}\n"),
        Some("toml" | "sh" | "py" | "yaml" | "yml") => format!("# {REQUIRED_TEXT}\n"),
        Some("ld" | "s" | "S") => format!("/* {REQUIRED_TEXT} */\n"),
        other => {
            return Err(std::io::Error::other(format!(
                "Unknown extension {other:?}"
            )))
        }
    });

    let mut prev_contents = std::fs::read(path).map_err(wrap_err)?;
    if prev_contents.first() != Some(&b'\n') {
        contents.push(b'\n');
    }
    contents.append(&mut prev_contents);
    std::fs::write(path, contents)?;
    Ok(())
}

fn allow(file: &DirEntry, project_root: &Path) -> bool {
    let file_path = remove_root(file.path(), project_root);

    if IGNORED_PATHS
        .iter()
        .any(|ignored| file_path.starts_with(ignored))
    {
        return false;
    }

    let file_type = file.file_type();
    if file_type.is_dir() {
        if let Some(file_name) = file.file_name().to_str() {
            if IGNORED_DIRS.contains(&file_name) {
                return false;
            }
        }
    }

    true
}

pub(crate) fn find_files(
    dir: &Path,
    extensions: &[&str],
    ignore_none: bool,
) -> Result<Vec<PathBuf>, Error> {
    let mut result = vec![];
    let wrap_err = add_path_walkdir_error(dir, dir);
    let walker = walkdir::WalkDir::new(dir).into_iter();

    for file in walker.filter_entry(|f| ignore_none || allow(f, dir)) {
        let file = file.map_err(wrap_err)?;
        let file_path = &file.path();
        let file_type = file.file_type();

        if let Some(Some(extension)) = file.path().extension().map(|s| s.to_str()) {
            if file_type.is_file() && extensions.contains(&extension) {
                result.push(file_path.into());
            }
        }
    }

    result.sort();
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_check_success() {
        let project_root = PathBuf::from("/tmp");
        assert!(
            check_file_contents(
                Path::new("foo/bar.rs"),
                "# Licensed under the Apache-2.0 license".as_bytes(),
                &project_root,
            )
            .is_ok(),
            "Expected license check to pass for shell comment"
        );
        assert!(
            check_file_contents(
                Path::new("foo/bar.rs"),
                "/*\n * Licensed under the Apache-2.0 license\n */".as_bytes(),
                &project_root,
            )
            .is_ok(),
            "Expected license check to pass for C-style comment"
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_check_failures() {
        let project_root = PathBuf::from("/tmp");
        let result = check_file_contents(
            Path::new("foo/bar.rs"),
            "int main()\n {\n // foobar\n".as_bytes(),
            &project_root,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "File \"foo/bar.rs\" doesn't contain \"Licensed under the Apache-2.0 license\" in the first 3 lines"
        );

        let result = check_file_contents(Path::new("bar/foo.sh"), "".as_bytes(), &project_root);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "File \"bar/foo.sh\" doesn't contain \"Licensed under the Apache-2.0 license\" in the first 3 lines"
        );
    }
}
