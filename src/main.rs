use std::{
    collections::HashSet,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

use clap::Parser;
use git2::{
    AnnotatedCommit, Config, ObjectType, Oid, RebaseOptions, Reference, ReferenceNames, Repository,
    Signature,
};
use serde::Serialize;

use crate::pkjson::{PkExport, PkMember};
mod pkjson;

#[derive(Parser, Debug)]
struct Args {
    mode: HookMode,
    tempfile_path: Option<PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Parser, Debug)]
enum HookMode {
    CommitMsg,
    PostCommit,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // We check the configs to see if there is a system export file from pluralkit configured
    let cfg = Config::open_default().expect("No global git config detected.");
    let export_path = cfg.get_path("pluralgit.pk-export-path").expect("Please configure \"pluralgit.pk-export-path\" with the path to a PluralKit system export JSON.");

    let pk_export_file = File::open(export_path)?;
    let pk_export: PkExport =
        serde_json::from_reader(pk_export_file).expect("Invalid pluralkit export file.");
    let repo = Repository::discover(std::env::current_dir().unwrap())
        .expect("Current folder was not inside a git repository");

    let _ = match args.mode {
        HookMode::CommitMsg => commit_msg_hook(
            args.tempfile_path
                .expect("This commit hook requires an argument (temporary commitmsg path)"),
            pk_export,
            repo,
        ),
        HookMode::PostCommit => post_commit_hook(repo).map_err(|_| io::Error::last_os_error()),
    };

    Ok(())
}

fn commit_msg_hook(path: PathBuf, pk_export: PkExport, repo: Repository) -> io::Result<()> {
    // We need to open it and see the first non-empty, non-comment line.
    let mut temp_file = File::open(path.clone())?;
    let mut commit_msg = String::new();

    // Skip all lines that start with an #, but if we find that skip the rest
    const IGNORE_STRING: &'static str = "# ------------------------ >8 ------------------------";

    temp_file.read_to_string(&mut commit_msg)?;

    let mut members: HashSet<String> = HashSet::new();

    let mut nlines: Vec<String> = vec![];

    let mut first_member = None;

    for line in commit_msg.lines() {
        if line.contains(IGNORE_STRING) {
            break;
        }
        if line.starts_with("#") {
            continue;
        }

        // !!Did something like this.
        match pk_export.match_text(line.trim().to_owned()) {
            Ok((member, message)) => {
                if first_member.is_none() {
                    first_member = Some(member.clone());
                }
                members.insert(format!("{} (id={})", member.name, member.id));
                nlines.push(message);
            }
            Err(message) => nlines.push(message),
        }
    }

    if !members.is_empty() {
        let mut ivec: Vec<_> = members.iter().cloned().collect();
        ivec.sort();

        // Add blank line padding.
        nlines.push(String::new());
        nlines.push(String::new());

        if let Some(first_member) = first_member {
            nlines.push(format!("First-Member: {}", first_member.name));
            ivec.retain(|x| !x.contains(&first_member.id));
        }

        nlines.push(format!("System-Pluralkit-Id: {}", pk_export.id));
        for member in ivec {
            nlines.push(format!("Co-authored-by: {}", member));
        }
    }

    let _ = std::fs::write(path, nlines.join("\n").as_bytes());

    Ok(())
}

fn post_commit_hook(repo: Repository) -> Result<(), git2::Error> {
    let commit = repo.head()?.resolve()?.peel_to_commit()?;
    let author = commit.author();

    let message = commit.message().unwrap_or_default();

    let mut new_author_name = None;

    let mut new_msg = vec![];

    for line in message.lines() {
        if line.starts_with("First-Member:") {
            new_author_name = line.strip_prefix("First-Member: ");
        } else {
            new_msg.push(line);
        }
    }

    let new_sig = Signature::new(
        &new_author_name.unwrap_or(author.name().unwrap_or_default()),
        author.email().unwrap_or_default(),
        &commit.time(),
    )?;
    let new_author = Some(&new_sig);

    let _ = commit.amend(Some("HEAD"), new_author, new_author, None, Some(&new_msg.join("\n")), None);

    Ok(())
}
