use crate::errors::*;
use crate::git::GitSource;
use std::borrow::Cow;
use std::fmt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use url::Url;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Input {
    Url(Source),
    UrlWithFilename((Source, String)),
}

impl Input {
    pub fn filename(&self) -> Result<Cow<str>> {
        match self {
            Input::Url(source) => source.filename(),
            Input::UrlWithFilename((_, filename)) => Ok(Cow::Borrowed(filename)),
        }
    }

    pub fn source(&self) -> &Source {
        match self {
            Input::Url(url) => url,
            Input::UrlWithFilename((url, _file)) => url,
        }
    }

    pub fn source_mut(&mut self) -> &mut Source {
        match self {
            Input::Url(url) => url,
            Input::UrlWithFilename((url, _file)) => url,
        }
    }

    pub fn take_source(self) -> Source {
        match self {
            Input::Url(url) => url,
            Input::UrlWithFilename((url, _file)) => url,
        }
    }
}

impl fmt::Display for Input {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Input::Url(url) => write!(w, "{}", url),
            Input::UrlWithFilename((url, file)) => write!(w, "{}::{}", file, url),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Source {
    File(String),
    Url(String),
    Git(GitSource),
    /*
    Svn(SvnSource),
    Hg(HgSource),
    Bzr(BzrSource),
    */
}

impl Source {
    pub fn filename(&self) -> Result<Cow<str>> {
        let filename = match self {
            Source::File(path) => {
                let p = Path::new(path);
                let filename = p.file_name().context("Missing filename")?;
                let filename = filename.to_str().context("Filename is invalid utf8")?;
                Cow::Borrowed(filename)
            }
            Source::Url(url) => {
                let url = url.parse::<Url>()?;
                let filename = url
                    .path_segments()
                    .context("Url contains no path")?
                    .last()
                    .context("Path has no filename")?;
                Cow::Owned(filename.to_string())
            }
            Source::Git(git) => {
                let url = git.url.parse::<Url>()?;
                let filename = url
                    .path_segments()
                    .context("Url contains no path")?
                    .last()
                    .context("Path has no filename")?;
                Cow::Owned(filename.to_string())
            }
        };
        if filename.is_empty() {
            bail!("Filename can't be empty");
        }
        Ok(filename)
    }
}

impl fmt::Display for Source {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Source::File(s) => write!(w, "{}", s),
            Source::Url(s) => write!(w, "{}", s),
            Source::Git(s) => write!(w, "{}", s),
        }
    }
}

impl FromStr for Source {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let scheme = s.split_once("://").map(|x| x.0);
        Ok(match scheme {
            Some("https") => Source::Url(s.to_string()),
            Some("http") => Source::Url(s.to_string()),
            Some("ftp") => Source::Url(s.to_string()),
            Some(scheme) if scheme.starts_with("git") => Source::Git(s.parse()?),
            Some(scheme) => bail!("Unknown scheme: {:?}", scheme),
            None => Source::File(s.to_string()),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedPin {
    pub commit_hash: String,
    pub tag_hash: String,
    pub source: Source,
}

fn exec_sh(path: &Path, cmd: &str) -> Result<Vec<String>> {
    let pkgbuild = path.canonicalize()?;
    let child = Command::new("bash")
        .arg("-c")
        .arg(format!("source \"{}\";{}", pkgbuild.display(), cmd))
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to run bash")?;

    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "Process (bash, {:?}) exited with error: {:?}",
            cmd,
            out.status
        );
    }

    let buf = String::from_utf8(out.stdout).context("Shell output contains invalid utf8")?;
    Ok(buf.lines().map(String::from).collect())
}

pub fn list_variable(folder: &Path, var: &str) -> Result<Vec<String>> {
    exec_sh(
        folder,
        &format!("for x in ${{{}[@]}}; do echo \"$x\"; done", var),
    )
}

pub fn list_source_list_from_var(path: &Path, var: &str) -> Result<Vec<Input>> {
    let sources = list_variable(path, var)?;
    let sources = sources
        .into_iter()
        .map(|line| {
            if let Some((file, url)) = line.split_once("::") {
                let source = url.parse()?;
                Ok(Input::UrlWithFilename((source, file.to_string())))
            } else {
                let source = line.parse()?;
                Ok(Input::Url(source))
            }
        })
        .collect::<Result<_>>()?;
    Ok(sources)
}

pub fn list_pins(path: &Path) -> Result<Vec<Input>> {
    list_source_list_from_var(path, "vcspins")
}

pub fn list_sources(path: &Path) -> Result<Vec<Input>> {
    list_source_list_from_var(path, "source")
}
