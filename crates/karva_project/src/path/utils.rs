use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

pub fn absolute(path: impl AsRef<Utf8Path>, cwd: impl AsRef<Utf8Path>) -> Utf8PathBuf {
    let path = path.as_ref();
    let cwd = cwd.as_ref();

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ (Utf8Component::Prefix(..) | Utf8Component::RootDir)) =
        components.peek().copied()
    {
        components.next();
        Utf8PathBuf::from(c.as_str())
    } else {
        cwd.to_path_buf()
    };

    for component in components {
        match component {
            Utf8Component::Prefix(..) => {}

            Utf8Component::RootDir => {
                ret.push(component.as_str());
            }
            Utf8Component::CurDir => {}
            Utf8Component::ParentDir => {
                ret.pop();
            }
            Utf8Component::Normal(c) => {
                ret.push(c);
            }
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;

    /// Render the path with forward slashes so the same snapshot pins down
    /// behaviour on Windows (which would otherwise emit backslashes).
    fn posix(path: &Utf8Path) -> String {
        path.as_str().replace('\\', "/")
    }

    #[test]
    fn relative_path_is_joined_to_cwd() {
        assert_snapshot!(posix(&absolute("foo/bar", "/home/user")), @"/home/user/foo/bar");
    }

    #[test]
    fn absolute_path_ignores_cwd() {
        assert_snapshot!(posix(&absolute("/absolute/path", "/home/user")), @"/absolute/path");
    }

    #[test]
    fn parent_dir_pops_component() {
        assert_snapshot!(posix(&absolute("../sibling", "/home/user")), @"/home/sibling");
    }

    #[test]
    fn current_dir_is_ignored() {
        assert_snapshot!(posix(&absolute("./foo", "/home/user")), @"/home/user/foo");
    }

    #[test]
    fn mixed_components() {
        assert_snapshot!(posix(&absolute("foo/../bar/./baz", "/cwd")), @"/cwd/bar/baz");
    }

    #[test]
    fn empty_relative_path_returns_cwd() {
        assert_snapshot!(posix(&absolute("", "/home/user")), @"/home/user");
    }

    #[test]
    fn leading_parent_pops_cwd() {
        assert_snapshot!(posix(&absolute("../../other", "/home/user")), @"/other");
    }

    /// `Utf8PathBuf::pop` on `/` returns false, so extra `..` components
    /// must not escape the filesystem root.
    #[test]
    fn parent_past_root_stays_at_root() {
        assert_snapshot!(posix(&absolute("../..", "/")), @"/");
    }

    #[test]
    fn unicode_path_components_are_preserved() {
        assert_snapshot!(posix(&absolute("カルヴァ/tests", "/home/ユーザー")), @"/home/ユーザー/カルヴァ/tests");
    }

    #[test]
    fn path_with_spaces_is_preserved() {
        assert_snapshot!(posix(&absolute("my tests/file.py", "/home/my user")), @"/home/my user/my tests/file.py");
    }

    /// `camino` components strip trailing slashes, so the result should
    /// match the same input without one.
    #[test]
    fn trailing_slash_on_relative_input_is_normalized() {
        let with = absolute("foo/bar/", "/cwd");
        let without = absolute("foo/bar", "/cwd");
        assert_eq!(with, without);
        assert_snapshot!(posix(&with), @"/cwd/foo/bar");
    }

    #[test]
    fn dot_only_path_is_cwd() {
        assert_snapshot!(posix(&absolute(".", "/home/user")), @"/home/user");
    }
}
