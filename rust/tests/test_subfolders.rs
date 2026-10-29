use grimoire::prelude::*;
#[test]
fn nested_destinations_keep_notebooks_and_empty_folders_reachable() {
    let root = std::env::temp_dir().join(format!("grimoire-subfolders-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Notes", "School/Math/Algebra", PAPER_BLANK, &mut nb), 0);
    assert_eq!(store_add_folder("School/Physics/Empty"), 0);
    let mut folders = Vec::new();
    assert_eq!(store_list_folders(&mut folders), 0);
    for name in ["School", "School/Math", "School/Math/Algebra", "School/Physics", "School/Physics/Empty"] {
        assert!(folders.iter().any(|f| f == name));
    }
    assert_eq!(store_folder_parent("School/Math/Algebra"), "School/Math");
    assert_eq!(store_folder_leaf("School/Math/Algebra"), "Algebra");
    assert!(store_folder_contains("School", "School/Math/Algebra"));
    assert!(!store_folder_contains("School", "Schoolwork"));
    let mut joined = FixedStr::new();
    assert!(store_folder_join("School/Math", "Geometry", &mut joined));
    assert_eq!(joined.as_str(), "School/Math/Geometry");
    for bad in ["/School", "School/", "School//Math", "School/../Math", ".", ".."] {
        assert!(!store_folder_valid(bad));
        assert_eq!(store_set_folder(nb.id.as_str(), bad), -1);
    }
    assert!(!store_folder_join("School", "../escape", &mut joined));
    assert!(!store_folder_join(&"x".repeat(90), "longer", &mut joined));
    let mut loaded = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut loaded), 0);
    assert_eq!(loaded.folder.as_str(), "School/Math/Algebra");
    assert_eq!(store_set_folder(nb.id.as_str(), "School/Physics"), 0);
    assert_eq!(store_load_info(nb.id.as_str(), &mut loaded), 0);
    assert_eq!(loaded.folder.as_str(), "School/Physics");
    assert_eq!(store_add_folder("Elsewhere"), 0);
    assert_eq!(store_rename_folder("School", "Elsewhere"), -1);
    assert_eq!(store_rename_folder("School", "School/Inside"), -1);
    let id = loaded.id.as_str().to_string();
    let ini = root.join("notebooks").join(&id).join("notebook.ini");
    let before = std::fs::read(&ini).unwrap();
    assert_eq!(store_rename_folder("School", &"z".repeat(94)), -1);
    assert_eq!(std::fs::read(&ini).unwrap(), before);
    assert_eq!(store_rename_folder("School", "College"), 0);
    assert_eq!(store_load_info(&id, &mut loaded), 0);
    assert_eq!(loaded.folder.as_str(), "College/Physics");
    assert_eq!(store_list_folders(&mut folders), 0);
    assert!(folders.iter().any(|f| f == "College/Physics/Empty"));
    assert!(!folders.iter().any(|f| store_folder_contains("School", f)));
    // Folder ZIPs keep descendants and do not include a similarly named sibling.
    let mut outside=NotebookInfo::default();assert_eq!(store_create("Outside","Collegework",PAPER_BLANK,&mut outside),0);
    ui_init(300.0);assert_eq!(font_init(),0);
    let mut archive=Buf::default();assert_eq!(export_folder_zip("College",25.0,&mut archive),0);
    let data=&archive.data;let end=data.len()-22;assert_eq!(&data[end..end+4],b"PK\x05\x06");
    let u16_at=|at:usize|u16::from_le_bytes(data[at..at+2].try_into().unwrap()) as usize;
    let mut at=u32::from_le_bytes(data[end+16..end+20].try_into().unwrap()) as usize;
    let mut entries=Vec::new();
    while at<end {
        assert_eq!(&data[at..at+4],b"PK\x01\x02");let n=u16_at(at+28);
        entries.push(std::str::from_utf8(&data[at+46..at+46+n]).unwrap());
        at+=46+n+u16_at(at+30)+u16_at(at+32);
    }
    assert!(entries.iter().any(|n|n.starts_with("Physics/Notes/")),"{entries:?}");
    assert!(!entries.iter().any(|n|n.contains("Outside")||n.contains("Collegework")));
    std::fs::remove_dir_all(root).unwrap();
}
