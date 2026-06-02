use omninote_core::vault::Vault;
use std::fs;
use tempfile::tempdir;

fn setup_vault() -> (Vault, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let vault = Vault::open(dir.path().to_path_buf()).unwrap();
    (vault, dir)
}

#[test]
fn test_attachment_path_backslash_separators() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");
    fs::create_dir_all(&attach_dir).unwrap();

    // 1. When the file does not exist, it should return None
    assert!(vault.attachment_path("sub\\foto.png").is_none());
    assert!(vault.attachment_path("..\\secret.md").is_none());
    assert!(vault.attachment_path("sub\\..\\secret.md").is_none());

    // On Unix, backslash is not a path separator. It is a normal character in the filename.
    // If we write a file containing a backslash, Unix will treat the backslash as part of the filename itself.
    // 2. Create files with backslashes in their names inside _attachments
    let backslash_file = attach_dir.join("sub\\foto.png");
    fs::write(&backslash_file, b"test").unwrap();

    let path_result = vault.attachment_path("sub\\foto.png");
    assert!(path_result.is_some());
    assert_eq!(path_result.unwrap(), backslash_file.canonicalize().unwrap());

    // Test with "..\\secret.md" as a filename
    let dotdot_backslash = attach_dir.join("..\\secret.md");
    fs::write(&dotdot_backslash, b"test").unwrap();
    let path_result2 = vault.attachment_path("..\\secret.md");
    assert!(path_result2.is_some());
    assert_eq!(
        path_result2.unwrap(),
        dotdot_backslash.canonicalize().unwrap()
    );

    // Test with "sub\\..\\secret.md" as a filename
    let multi_backslash = attach_dir.join("sub\\..\\secret.md");
    fs::write(&multi_backslash, b"test").unwrap();
    let path_result3 = vault.attachment_path("sub\\..\\secret.md");
    assert!(path_result3.is_some());
    assert_eq!(
        path_result3.unwrap(),
        multi_backslash.canonicalize().unwrap()
    );
}

#[test]
fn test_attachment_path_nul_byte() {
    let (vault, _dir) = setup_vault();
    // A string with a NUL byte is passed as filename.
    // It should be rejected and return None without panicking.
    assert!(vault.attachment_path("foto\0.png").is_none());
    assert!(vault.attachment_path("\0").is_none());
    assert!(vault.attachment_path("foto.png\0").is_none());
}

#[test]
fn test_attachment_path_only_dots() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");
    fs::create_dir_all(&attach_dir).unwrap();

    // "." and ".." are component types (CurDir and ParentDir) and should return None
    assert!(vault.attachment_path(".").is_none());
    assert!(vault.attachment_path("..").is_none());

    // "..." is not a special component, it is a normal filename of three dots.
    // Before creating it, it doesn't exist, so should return None.
    assert!(vault.attachment_path("...").is_none());
    assert!(vault.attachment_path("....").is_none());

    // Create the "..." file
    let dot3 = attach_dir.join("...");
    fs::write(&dot3, b"dots").unwrap();
    let res3 = vault.attachment_path("...");
    assert!(res3.is_some());
    assert_eq!(res3.unwrap(), dot3.canonicalize().unwrap());

    // Create the "...." file
    let dot4 = attach_dir.join("....");
    fs::write(&dot4, b"more dots").unwrap();
    let res4 = vault.attachment_path("....");
    assert!(res4.is_some());
    assert_eq!(res4.unwrap(), dot4.canonicalize().unwrap());
}

#[test]
fn test_attachment_path_spaces_and_unicode() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");
    fs::create_dir_all(&attach_dir).unwrap();

    // Test spaces inside filename
    let spaced_file = attach_dir.join("foto do gato.png");
    fs::write(&spaced_file, b"spaced").unwrap();
    let res1 = vault.attachment_path("foto do gato.png");
    assert!(res1.is_some());
    assert_eq!(res1.unwrap(), spaced_file.canonicalize().unwrap());

    // Test unicode characters (emojis, accented characters)
    let emoji_file = attach_dir.join("foto_📷.png");
    fs::write(&emoji_file, b"emoji").unwrap();
    let res2 = vault.attachment_path("foto_📷.png");
    assert!(res2.is_some());
    assert_eq!(res2.unwrap(), emoji_file.canonicalize().unwrap());

    let accented_file = attach_dir.join("café.png");
    fs::write(&accented_file, b"accented").unwrap();
    let res3 = vault.attachment_path("café.png");
    assert!(res3.is_some());
    assert_eq!(res3.unwrap(), accented_file.canonicalize().unwrap());

    // Test trimmed whitespace
    let res4 = vault.attachment_path("  café.png  ");
    assert!(res4.is_some());
    assert_eq!(res4.unwrap(), accented_file.canonicalize().unwrap());
}

#[test]
#[cfg(unix)]
fn test_attachment_path_symlink_traversal() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");
    fs::create_dir_all(&attach_dir).unwrap();

    // 1. Create a secret file OUTSIDE the attachments directory (in the vault root)
    let secret_file = vault.root.join("secret.md");
    fs::write(&secret_file, b"super secret content").unwrap();

    // 2. Create a symlink inside _attachments pointing to that secret file outside
    let sym_outside = attach_dir.join("sym_outside");
    std::os::unix::fs::symlink(&secret_file, &sym_outside).unwrap();

    // 3. Trying to access via the symlink name should be rejected (CWE-22 defense)
    // because its canonical path resolves outside of _attachments.
    assert!(vault.attachment_path("sym_outside").is_none());

    // 4. Create a file INSIDE _attachments
    let legit_file = attach_dir.join("legit.png");
    fs::write(&legit_file, b"legit image").unwrap();

    // 5. Create a symlink inside _attachments pointing to the file INSIDE _attachments
    let sym_inside = attach_dir.join("sym_inside");
    std::os::unix::fs::symlink(&legit_file, &sym_inside).unwrap();

    // 6. Accessing via symlink pointing inside should be ALLOWED
    let res = vault.attachment_path("sym_inside");
    assert!(res.is_some());
    assert_eq!(res.unwrap(), legit_file.canonicalize().unwrap());
}

#[test]
fn test_attachment_path_existence() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");
    fs::create_dir_all(&attach_dir).unwrap();

    // Legitimate name that exists
    let file = attach_dir.join("exists.png");
    fs::write(&file, b"data").unwrap();
    assert!(vault.attachment_path("exists.png").is_some());

    // Legitimate name that does not exist
    assert!(vault.attachment_path("does_not_exist.png").is_none());
}

#[test]
fn test_attachment_path_nonexistent_attachments_dir() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");

    // Completely remove the _attachments directory if it exists
    if attach_dir.exists() {
        fs::remove_dir_all(&attach_dir).unwrap();
    }

    // Now, even if we query a standard filename, it should return None
    // because canonicalize() on the base directory will fail.
    assert!(vault.attachment_path("foto.png").is_none());
}

#[test]
fn test_attachment_path_case_sensitivity() {
    let (vault, _dir) = setup_vault();
    let attach_dir = vault.root.join("_attachments");
    fs::create_dir_all(&attach_dir).unwrap();

    let lowercase_file = attach_dir.join("foto.png");
    fs::write(&lowercase_file, b"lowercase").unwrap();

    // Determine if the filesystem is case-insensitive.
    // If we can access FOTO.PNG on disk via metadata/exists, the filesystem is case-insensitive.
    let is_case_insensitive = attach_dir.join("FOTO.PNG").exists();
    let res = vault.attachment_path("FOTO.PNG");

    if is_case_insensitive {
        // On macOS (default APFS is case-insensitive), FOTO.PNG resolves to foto.png
        assert!(res.is_some());
        assert_eq!(res.unwrap(), lowercase_file.canonicalize().unwrap());
    } else {
        // On case-sensitive filesystems, FOTO.PNG should be None
        assert!(res.is_none());
    }
}

#[test]
fn test_attachment_path_very_long_name() {
    let (vault, _dir) = setup_vault();

    // Extremely long name (e.g. 500 characters, and 5000 characters)
    let long_name_500 = "a".repeat(500);
    let long_name_5000 = "a".repeat(5000);

    assert!(vault.attachment_path(&long_name_500).is_none());
    assert!(vault.attachment_path(&long_name_5000).is_none());
}
