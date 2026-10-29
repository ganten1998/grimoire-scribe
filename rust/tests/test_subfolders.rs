use grimoire::prelude::*;
#[test]
fn nested_destinations_keep_notebooks_and_empty_folders_reachable() {
    let root=std::env::temp_dir().join(format!("grimoire-subfolders-{}",std::process::id()));
    std::fs::create_dir_all(&root).unwrap();assert_eq!(store_init(root.to_str().unwrap()),0);
    let mut nb=NotebookInfo::default();assert_eq!(store_create("Notes","School/Math/Algebra",PAPER_BLANK,&mut nb),0);
    assert_eq!(store_add_folder("School/Physics/Empty"),0);
    let mut folders=Vec::new();assert_eq!(store_list_folders(&mut folders),0);
    for name in ["School","School/Math","School/Math/Algebra","School/Physics","School/Physics/Empty"] {assert!(folders.iter().any(|f|f==name));}
    assert_eq!(store_folder_parent("School/Math/Algebra"),"School/Math");
    assert_eq!(store_folder_leaf("School/Math/Algebra"),"Algebra");
    assert!(store_folder_contains("School","School/Math/Algebra"));assert!(!store_folder_contains("School","Schoolwork"));
    let mut joined=FixedStr::new();assert!(store_folder_join("School/Math","Geometry",&mut joined));assert_eq!(joined.as_str(),"School/Math/Geometry");
    for bad in ["/School","School/","School//Math","School/../Math",".",".."] {assert!(!store_folder_valid(bad));assert_eq!(store_set_folder(nb.id.as_str(),bad),-1);}
    assert!(!store_folder_join("School","../escape",&mut joined));
    assert!(!store_folder_join(&"x".repeat(90),"longer",&mut joined));
    let mut loaded=NotebookInfo::default();assert_eq!(store_load_info(nb.id.as_str(),&mut loaded),0);assert_eq!(loaded.folder.as_str(),"School/Math/Algebra");
    assert_eq!(store_set_folder(nb.id.as_str(),"School/Physics"),0);assert_eq!(store_load_info(nb.id.as_str(),&mut loaded),0);assert_eq!(loaded.folder.as_str(),"School/Physics");
    std::fs::remove_dir_all(root).unwrap();
}
