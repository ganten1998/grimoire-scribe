use grimoire::prelude::*;
fn contact(slot:i32,x:f32)->TgContact {TgContact{id:100+slot,slot,x,y:900.0+slot as f32*280.0,..Default::default()}}
fn frame(k:u32,contacts:&[TgContact],ended:&[TgContact])->TgFrame {
    let mut f=TgFrame{k_ms:k,n:contacts.len(),ended_n:ended.len(),..Default::default()};
    f.c[..contacts.len()].copy_from_slice(contacts);f.ended[..ended.len()].copy_from_slice(ended);f
}
fn feed(g:&mut TouchGest,f:TgFrame)->Vec<InEvent> {
    let mut o=TgOut::default();touchgest_frame(g,&f,&TgPen::default(),&mut o);o.ev[..o.n].to_vec()
}
fn init()->TouchGest {let mut g=TouchGest::default();touchgest_init(&mut g,300.0);touchgest_set_view(&mut g,TgView{gestures_on:true,zoom_ok:true,..Default::default()});g}
#[test]
fn fast_swipe_uses_coordinates_from_the_release_report() {
    let mut g=init();feed(&mut g,frame(100000,&[contact(0,700.0),contact(1,700.0)],&[]));
    feed(&mut g,frame(100016,&[contact(0,740.0),contact(1,740.0)],&[]));
    let ev=feed(&mut g,frame(100032,&[],&[contact(0,1000.0),contact(1,1000.0)]));
    assert_eq!(ev.iter().filter(|e|e.type_==InType::Gesture && e.value==G_2_RIGHT).count(),1);
    assert!(!feed(&mut g,frame(100048,&[],&[])).iter().any(|e|e.type_==InType::Gesture));
}
#[test]
fn deliberate_slow_swipe_works_but_a_prolonged_drag_does_not_turn() {
    for (duration,want) in [(1600,true),(2400,true),(4000,false)] {
        let mut g=init();feed(&mut g,frame(100000,&[contact(0,700.0),contact(1,700.0)],&[]));
        for i in 1..=10 {let x=700.0+i as f32*30.0;feed(&mut g,frame(100000+duration*i/10,&[contact(0,x),contact(1,x)],&[]));}
        let ev=feed(&mut g,frame(100000+duration+10,&[],&[contact(0,1000.0),contact(1,1000.0)]));
        assert_eq!(ev.iter().any(|e|e.type_==InType::Gesture && e.value==G_2_RIGHT),want,"duration {duration}");
    }
}

#[test]
fn release_movement_keeps_pen_palm_pinch_and_stale_contact_guards() {
    for mode in 0..6 {
        let mut g=init();
        let start=[contact(0,700.0),contact(1,700.0)];
        feed(&mut g,frame(100000,&start,&[]));
        let mut end=[contact(0,1000.0),contact(1,1000.0)];
        let mut pen=TgPen::default();
        match mode {
            0=>pen.touching=true,
            1=>pen.prox=true,
            2=>end[0].tool=2,
            3=>end[0].tool=1,
            4=>{end[0].x=500.0;end[1].x=900.0;}, // separation is pinch, not a page turn
            _=>{end[0].id+=10;end[1].id+=10;}, // no matching active tracking IDs
        }
        let mut out=TgOut::default();
        touchgest_frame(&mut g,&frame(100032,&[],&end),&pen,&mut out);
        assert!(!out.ev[..out.n].iter().any(|e|e.type_==InType::Gesture && matches!(e.value,G_2_RIGHT|G_2_LEFT|G_2_UP|G_2_DOWN)),"mode {mode}");
        assert!(!feed(&mut g,frame(100048,&[],&[])).iter().any(|e|e.type_==InType::Gesture));
    }
}

#[test]
fn landing_window_accepts_a_slight_stagger_but_rejects_a_late_second_finger() {
    for (gap,want) in [(180,true),(240,false)] {
        let mut g=init();feed(&mut g,frame(100000,&[contact(0,700.0)],&[]));
        feed(&mut g,frame(100000+gap,&[contact(0,700.0),contact(1,700.0)],&[]));
        feed(&mut g,frame(100020+gap,&[contact(0,1000.0),contact(1,1000.0)],&[]));
        let end=feed(&mut g,frame(100040+gap,&[],&[contact(0,1000.0),contact(1,1000.0)]));
        assert_eq!(end.iter().any(|e|e.type_==InType::Gesture && e.value==G_2_RIGHT),want,"gap {gap}");
    }
}
