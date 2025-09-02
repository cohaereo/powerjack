use std::borrow::Cow;

pub fn ensure_path_has_extension<'s>(path: &'s str, extension: &str) -> Cow<'s, str> {
    if path.ends_with(&format!(".{extension}")) {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(format!("{}.{}", path, extension))
    }
}
