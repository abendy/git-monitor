use git2::Repository;

mod branches;
mod command_type;
mod commit;
mod diff;
mod history;
mod repo;
mod status;
mod time;
mod types;

pub use time::format_relative_time;
pub use types::{
    BranchInfo, CommandType, CommitDetail, CommitFile, FileState, FileStatus, GitCommand,
    GitStatus, RefDecoration,
};

/// Git repository wrapper
pub struct GitRepo {
    repo: Repository,
}
