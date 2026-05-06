#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Collision {
    FreshInstall,
    RestoreMissing,
    UpToDate,
    Upgrade,
    UserEdited,
    UserEditedAndUpgrade,
    UnknownFile,
    Symlink,
}

pub fn classify(
    current: Option<&str>,
    manifest: Option<&str>,
    bundled: &str,
    is_symlink: bool,
) -> Collision {
    if is_symlink {
        return Collision::Symlink;
    }
    match (current, manifest) {
        (None, None) => Collision::FreshInstall,
        (None, Some(_)) => Collision::RestoreMissing,
        (Some(current), None) => {
            if current == bundled {
                Collision::UpToDate
            } else {
                Collision::UnknownFile
            }
        }
        (Some(current), Some(manifest)) if current == manifest && manifest == bundled => {
            Collision::UpToDate
        }
        (Some(current), Some(manifest)) if current == manifest && manifest != bundled => {
            Collision::Upgrade
        }
        (Some(current), Some(manifest)) if current != manifest && manifest == bundled => {
            Collision::UserEdited
        }
        (Some(_), Some(_)) => Collision::UserEditedAndUpgrade,
    }
}

#[cfg(test)]
mod tests {
    use super::{Collision, classify};

    #[test]
    fn classifies_collision_policy_cases() {
        assert_eq!(classify(None, None, "b", false), Collision::FreshInstall);
        assert_eq!(
            classify(None, Some("m"), "b", false),
            Collision::RestoreMissing
        );
        assert_eq!(
            classify(Some("b"), Some("b"), "b", false),
            Collision::UpToDate
        );
        assert_eq!(
            classify(Some("m"), Some("m"), "b", false),
            Collision::Upgrade
        );
        assert_eq!(
            classify(Some("u"), Some("b"), "b", false),
            Collision::UserEdited
        );
        assert_eq!(
            classify(Some("u"), Some("m"), "b", false),
            Collision::UserEditedAndUpgrade
        );
        assert_eq!(
            classify(Some("u"), None, "b", false),
            Collision::UnknownFile
        );
        assert_eq!(classify(Some("u"), None, "b", true), Collision::Symlink);
    }
}
