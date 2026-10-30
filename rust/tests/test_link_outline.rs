use grimoire::prelude::*;

#[test]
fn shape_outline_survives_save_copy_redraw_and_restoration_without_corner_hits() {
    let root=std::env::temp_dir().join(format!("link-outline-{}",std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()),0);store_set_device(300.0,800,1000);
    let mut nb=NotebookInfo::default();assert_eq!(store_create("Outlines","",PAPER_NARROW_NM,&mut nb),0);
    assert_eq!(store_add_page(&mut nb,1,PAPER_NARROW_NM),0);
    let pid=nb.page_ids[0];let target=format!("p{}",nb.page_ids[1]);
    let points:Vec<_>=(0..=360).map(|i|{let a=i as f32*std::f32::consts::TAU/360.0;(300.0+200.0*a.cos(),400.0+200.0*a.sin())}).collect();
    let outline=LinkOutline::from_points(&points,true,800,1000);
    assert!(outline.n>=16 && outline.n as usize<=LINK_OUTLINE_MAX);
    assert_eq!(LinkOutline::parse(&outline.text()),Some(outline));
    let mut nav=Nav::default();nav_init(&mut nav);
    assert_eq!(nav_link_set(&mut nav,pid,7,&target),0);
    assert_eq!(nav_link_set_area(&mut nav,pid,7,Some(Rect::new(100,200,400,400))),0);
    assert_eq!(nav_link_set_outline(&mut nav,pid,7,outline),0);
    assert_eq!(nav_save_links(nb.id.as_str(),&mut nav,&nb),0);
    let original=nav_link_get(&nav,pid,7).unwrap();
    let mut loaded=Nav::default();assert_eq!(nav_load(nb.id.as_str(),&mut loaded),0);
    assert_eq!(nav_link_get(&loaded,pid,7),Some(original));
    let p=page_new(800,1000,PAPER_NARROW_NM);let mut hit=NavLink::default();
    assert!(nav_link_at(&loaded,&p,pid,300.0,400.0,84.0,&mut hit));
    assert!(!nav_link_at(&loaded,&p,pid,110.0,210.0,84.0,&mut hit),"a circle must not hit its bounding-box corner");
    assert_eq!(nav_link_copy(&mut loaded,pid,nb.page_ids[1]),0);
    assert_eq!(nav_link_get(&loaded,nb.page_ids[1],7).unwrap().outline,outline);
    assert_eq!(nav_link_set_area(&mut loaded,pid,7,Some(Rect::new(200,300,300,400))),0);
    assert_eq!(nav_link_get(&loaded,pid,7).unwrap().outline.n,0,"a freehand rectangular redraw clears the previous shape");
    assert_eq!(nav_link_put(&mut loaded,&original),0);
    assert_eq!(nav_link_get(&loaded,pid,7),Some(original),"undo's full record restores geometry");
    let triangle=LinkOutline::from_points(&[(100.0,600.0),(300.0,200.0),(500.0,600.0),(100.0,600.0)],true,800,1000);
    assert_eq!(triangle.n,3);
    assert_eq!(nav_link_set_outline(&mut loaded,pid,7,triangle),0);
    assert!(nav_link_at(&loaded,&p,pid,300.0,450.0,84.0,&mut hit));
    assert!(!nav_link_at(&loaded,&p,pid,110.0,210.0,84.0,&mut hit));
    let line=LinkOutline::from_points(&[(100.0,200.0),(500.0,200.0)],false,800,1000);
    assert!(line.hit(Rect::new(100,200,400,1),Rect::new(100,160,400,84),300.0,200.0,84.0));
    for bad in ["c,0:0,1:1","o,6001:0,0:0","o,-1:0,0:0","c,0:0,1:1,2:nan"] {assert!(LinkOutline::parse(bad).is_none());}
    // Sidecar order is independent; malformed/unknown metadata survives, and bad geometry
    // cannot enlarge a live link beyond its valid rectangular fallback.
    let path=root.join("notebooks").join(nb.id.as_str()).join("links.ini");
    let text=format!("outline.{pid}=7,c,100:200,500:200,300:600\nlink.{pid}=7,{}\narea.{pid}=7,100,200,400,400\noutline.{pid}=7,c,0:0,1:1\nfuture=untouched\n",target);
    std::fs::write(&path,&text).unwrap();
    assert_eq!(nav_load(nb.id.as_str(),&mut loaded),0);
    assert_eq!(nav_link_get(&loaded,pid,7).unwrap().outline.n,3);
    assert_eq!(nav_save_links(nb.id.as_str(),&mut loaded,&nb),0);
    let saved=std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("future=untouched\n") && saved.contains(&format!("outline.{pid}=7,c,0:0,1:1\n")));
    // A geometrically valid but out-of-area outline remains opaque, not attached.
    std::fs::write(&path,format!("link.{pid}=7,{}\narea.{pid}=7,100,200,400,400\noutline.{pid}=7,c,0:0,20:0,20:20\n",target)).unwrap();
    assert_eq!(nav_load(nb.id.as_str(),&mut loaded),0);
    assert_eq!(nav_link_get(&loaded,pid,7).unwrap().outline.n,0);
    assert_eq!(nav_save_links(nb.id.as_str(),&mut loaded,&nb),0);
    assert!(std::fs::read_to_string(&path).unwrap().contains("c,0:0,20:0,20:20"));
    // A newer or malformed record beyond the unknown-line bound protects the file.
    std::fs::write(&path,"outline.1=1,".to_string()+&"x".repeat(256)).unwrap();
    let protected=std::fs::read(&path).unwrap();
    assert_eq!(nav_load(nb.id.as_str(),&mut loaded),-1);
    assert!(!loaded.links_writable);
    assert_eq!(nav_save_links(nb.id.as_str(),&mut loaded,&nb),-1);
    assert_eq!(std::fs::read(&path).unwrap(),protected);
    std::fs::remove_dir_all(root).unwrap();
}
