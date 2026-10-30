use grimoire::prelude::*;
use std::path::PathBuf;

#[test]
fn notebook_targets_and_preview_caches_preserve_identity_and_page_data() {
    let root=PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_|std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("notebook-links-{}",std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()),0);store_set_device(300.0,600,800);
    let mut a=NotebookInfo::default();let mut b=NotebookInfo::default();
    assert_eq!(store_create("Source","",PAPER_BLANK,&mut a),0);
    assert_eq!(store_create("Destination","",PAPER_BLANK,&mut b),0);
    let target=nav_notebook_target(b.id.as_str(),b.page_ids[0]).unwrap();
    assert!(target.len()>15);assert_eq!(nav_external_target(&target),Some((b.id.as_str(),b.page_ids[0])));
    for bad in ["n:../bad:p1","n:x:p0","n:x:p01","n:x:p4294967296","n::p1","n:x:p1/../../bad"] {
        assert!(nav_external_target(bad).is_none(),"{bad}");
    }
    let mut nav=Nav::default();nav_init(&mut nav);
    assert_eq!(nav_link_set(&mut nav,a.page_ids[0],1,&target),0);
    assert_eq!(nav_link_set_area(&mut nav,a.page_ids[0],1,Some(Rect::new(30,50,200,100))),0);
    nav.links_unknown=b"link.1=000000ff,future-target-too-long-for-the-old-parser\n".to_vec();
    assert_eq!(nav_save_links(a.id.as_str(),&mut nav,&a),0);
    let mut loaded=Nav::default();assert_eq!(nav_load(a.id.as_str(),&mut loaded),0);
    assert_eq!(loaded.links[0].target.as_str(),target);
    assert_eq!(loaded.links_unknown,nav.links_unknown);
    let mut graph=GraphModel::default();assert_eq!(graph_build(&mut graph,&a,&loaded,0),0);
    assert!(graph.nodes[0].linked);assert_eq!(graph.nodes[0].links_to,1);
    assert_eq!(store_rename(b.id.as_str(),"Renamed destination"),0);
    assert_eq!(store_set_folder(b.id.as_str(),"Moved/Nested"),0);
    assert!(nav_target_label(&target).unwrap().contains("Renamed destination"));
    assert_eq!(nav_save_links(a.id.as_str(),&mut loaded,&a),0);
    assert_eq!(loaded.links[0].target.as_str(),target);

    let mut page=page_new(600,800,PAPER_BLANK);
    assert_eq!(store_save_page(&a,0,&page),0);
    let mut canvas=Canvas::default();assert_eq!(canvas_init(&mut canvas,600,800),0);canvas_fill(&mut canvas,Rect::new(0,0,600,800),255);
    store_save_page_preview(&a,a.page_ids[0],&canvas);
    let mut preview=Canvas::default();assert!(store_load_page_preview(&a,a.page_ids[0],&mut preview));
    assert_eq!((preview.w,preview.h),(300,400));assert!(preview.px.iter().all(|&b|b==255));
    // Even an identical page rewrite invalidates the old cache: identity includes the source inode.
    assert_eq!(store_save_page(&a,0,&page),0);
    assert!(!store_load_page_preview(&a,a.page_ids[0],&mut preview));
    canvas_fill(&mut canvas,Rect::new(100,100,200,200),0);store_save_page_preview(&a,a.page_ids[0],&canvas);
    assert!(store_load_page_preview(&a,a.page_ids[0],&mut preview));assert!(preview.px.contains(&0));
    let before=store_page_preview_identity(&a,a.page_ids[0]).unwrap();
    let mut source_dir=String::new();store_notebook_dir(a.id.as_str(),&mut source_dir,600);
    let source_bytes=std::fs::read(format!("{source_dir}/p{}.qpg",a.page_ids[0])).unwrap();
    store_invalidate_page_previews(a.id.as_str());
    store_retag_compacted_preview(&a,a.page_ids[0],Some(&before),&source_bytes);
    assert!(!store_load_page_preview(&a,a.page_ids[0],&mut preview),"compaction must not undo explicit invalidation");
    assert_eq!(store_build_page_previews(a.id.as_str(),"0:1",300.0),0);
    assert!(store_load_page_preview(&a,a.page_ids[0],&mut preview));
    assert_eq!(store_build_page_previews(a.id.as_str(),"0:1000",300.0),-1);
    let mut dir=String::new();store_notebook_dir(a.id.as_str(),&mut dir,600);
    std::fs::write(format!("{dir}/.page-previews/p{}.pgm",a.page_ids[0]),b"P5\n8000 8000\n255\nx").unwrap();
    assert!(!store_load_page_preview(&a,a.page_ids[0],&mut preview));
    // Preview failures and invalidation have never changed the actual page.
    let mut read=None;assert_eq!(store_read_page(&a,0,&mut read),0);assert!(read.unwrap().strokes.is_empty());
    page_clear_strokes(&mut page);
    std::fs::remove_dir_all(root).unwrap();
}
