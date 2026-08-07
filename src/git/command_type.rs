//! Git command types from reflog entries.

/// Types of git commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Commit,
    Checkout,
    Merge,
    Rebase,
    Pull,
    #[allow(dead_code)]
    Push,
    Reset,
    CherryPick,
    Revert,
    Branch,
    Clone,
    Init,
    #[allow(dead_code)]
    Fetch,
    #[allow(dead_code)]
    Stash,
    Other,
}

impl CommandType {
    /// Parse command type from reflog message
    pub fn from_message(msg: &str) -> Self {
        let msg_lower = msg.to_lowercase();

        if msg_lower.starts_with("commit") {
            Self::Commit
        } else if msg_lower.starts_with("checkout") {
            Self::Checkout
        } else if msg_lower.starts_with("merge") {
            Self::Merge
        } else if msg_lower.starts_with("rebase") {
            Self::Rebase
        } else if msg_lower.starts_with("pull") {
            Self::Pull
        } else if msg_lower.starts_with("reset") {
            Self::Reset
        } else if msg_lower.starts_with("cherry-pick") {
            Self::CherryPick
        } else if msg_lower.starts_with("revert") {
            Self::Revert
        } else if msg_lower.starts_with("branch") {
            Self::Branch
        } else if msg_lower.starts_with("clone") {
            Self::Clone
        } else if msg_lower.contains("initial") {
            Self::Init
        } else {
            Self::Other
        }
    }

    /// Icon for this command type
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Commit => "●",
            Self::Checkout => "⎇",
            Self::Merge => "⑂",
            Self::Rebase => "↺",
            Self::Pull => "↓",
            Self::Push => "↑",
            Self::Fetch => "⟳",
            Self::Reset => "↩",
            Self::CherryPick => "❋",
            Self::Revert => "⊗",
            Self::Stash => "□",
            Self::Branch => "⌥",
            Self::Clone => "⊕",
            Self::Init => "★",
            Self::Other => "•",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_message_parses_commit() {
        assert_eq!(
            CommandType::from_message("commit: initial commit"),
            CommandType::Commit
        );
        assert_eq!(
            CommandType::from_message("Commit (amend): fix typo"),
            CommandType::Commit
        );
    }

    #[test]
    fn from_message_parses_checkout() {
        assert_eq!(
            CommandType::from_message("checkout: moving from main to feature"),
            CommandType::Checkout
        );
    }

    #[test]
    fn from_message_parses_merge() {
        assert_eq!(
            CommandType::from_message("merge feature-branch: Fast-forward"),
            CommandType::Merge
        );
    }

    #[test]
    fn from_message_parses_rebase() {
        assert_eq!(
            CommandType::from_message("rebase (finish): refs/heads/main onto abc123"),
            CommandType::Rebase
        );
    }

    #[test]
    fn from_message_parses_pull() {
        assert_eq!(
            CommandType::from_message("pull: Fast-forward"),
            CommandType::Pull
        );
    }

    #[test]
    fn from_message_parses_reset() {
        assert_eq!(
            CommandType::from_message("reset: moving to HEAD~1"),
            CommandType::Reset
        );
    }

    #[test]
    fn from_message_parses_cherry_pick() {
        assert_eq!(
            CommandType::from_message("cherry-pick: picked commit abc123"),
            CommandType::CherryPick
        );
    }

    #[test]
    fn from_message_parses_revert() {
        assert_eq!(
            CommandType::from_message("revert: reverting abc123"),
            CommandType::Revert
        );
    }

    #[test]
    fn from_message_parses_branch() {
        assert_eq!(
            CommandType::from_message("branch: created from HEAD"),
            CommandType::Branch
        );
    }

    #[test]
    fn from_message_parses_clone() {
        assert_eq!(
            CommandType::from_message("clone: from https://github.com/user/repo"),
            CommandType::Clone
        );
    }

    #[test]
    fn from_message_parses_init() {
        assert_eq!(
            CommandType::from_message("initial commit"),
            CommandType::Init
        );
    }

    #[test]
    fn from_message_returns_other_for_unknown() {
        assert_eq!(
            CommandType::from_message("unknown operation"),
            CommandType::Other
        );
        assert_eq!(
            CommandType::from_message(""),
            CommandType::Other
        );
    }

    #[test]
    fn icon_returns_non_empty_string() {
        let types = [
            CommandType::Commit,
            CommandType::Checkout,
            CommandType::Merge,
            CommandType::Rebase,
            CommandType::Pull,
            CommandType::Push,
            CommandType::Fetch,
            CommandType::Reset,
            CommandType::CherryPick,
            CommandType::Revert,
            CommandType::Branch,
            CommandType::Clone,
            CommandType::Init,
            CommandType::Stash,
            CommandType::Other,
        ];
        for cmd_type in types {
            assert!(
                !cmd_type.icon().is_empty(),
                "{cmd_type:?} should have non-empty icon"
            );
        }
    }
}
