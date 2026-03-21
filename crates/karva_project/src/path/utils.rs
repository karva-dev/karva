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
    use super::*;

    #[test]
    fn relative_path_is_joined_to_cwd() {
        let result = absolute("foo/bar", "/home/user");
        assert_eq!(result, Utf8PathBuf::from("/home/user/foo/bar"));
    }

    #[test]
    fn absolute_path_ignores_cwd() {
        let result = absolute("/absolute/path", "/home/user");
        assert_eq!(result, Utf8PathBuf::from("/absolute/path"));
    }

    #[test]
    fn parent_dir_pops_component() {
        let result = absolute("../sibling", "/home/user");
        assert_eq!(result, Utf8PathBuf::from("/home/sibling"));
    }

    #[test]
    fn current_dir_is_ignored() {
        let result = absolute("./foo", "/home/user");
        assert_eq!(result, Utf8PathBuf::from("/home/user/foo"));
    }

    #[test]
    fn mixed_components() {
        let result = absolute("foo/../bar/./baz", "/cwd");
        assert_eq!(result, Utf8PathBuf::from("/cwd/bar/baz"));
    }
}
