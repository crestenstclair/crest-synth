use std::path::{Path, PathBuf};

/// Resolve installed assets without changing persisted asset identities or cwd.
/// Once inside an app bundle, missing resources remain errors in that bundle.
pub(crate) fn path(relative: &str) -> std::io::Result<PathBuf> {
    Ok(path_for_executable(&std::env::current_exe()?, relative))
}

fn path_for_executable(executable: &Path, relative: &str) -> PathBuf {
    if let Some(macos) = executable.parent() {
        if macos.file_name().is_some_and(|name| name == "MacOS") {
            if let Some(contents) = macos.parent() {
                if contents.file_name().is_some_and(|name| name == "Contents")
                    && contents
                        .parent()
                        .and_then(Path::extension)
                        .is_some_and(|extension| extension == "app")
                {
                    return contents.join("Resources").join(relative);
                }
            }
        }
    }
    PathBuf::from(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_assets_resolve_inside_relocated_bundle_even_when_missing() {
        for relative in ["./sf2/HiDef.sf2", "./midi/song.mid"] {
            assert_eq!(
                path_for_executable(
                    Path::new("/Volumes/External Apps/crest-synth.app/Contents/MacOS/crest-synth"),
                    relative,
                ),
                Path::new("/Volumes/External Apps/crest-synth.app/Contents/Resources")
                    .join(relative),
            );
        }
    }

    #[test]
    fn development_and_unbundled_tools_keep_checkout_relative_paths() {
        for executable in [
            "/checkout/target/release/crest-synth",
            "/tmp/Contents/MacOS/crest-synth",
        ] {
            assert_eq!(
                path_for_executable(Path::new(executable), "./sf2/HiDef.sf2"),
                Path::new("./sf2/HiDef.sf2"),
            );
        }
    }
}
