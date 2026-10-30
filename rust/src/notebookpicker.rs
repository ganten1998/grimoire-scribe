//! A link destination browser. Selecting a notebook never replaces the editor's unsaved origin.
use crate::prelude::*;

#[derive(Default)]
pub struct NotebookPicker {
    pub open: bool,
    pub folder: String,
    books: Vec<NotebookInfo>,
    folders: Vec<String>,
    pub selected: Option<NotebookInfo>,
    nav: Nav,
    screen: usize,
    hits: HitList,
    press: Press,
    previews: PagePreviewRequests,
}
pub enum NotebookPick { None, Cancel, Target(String) }
impl NotebookPicker {
    pub fn start(&mut self, folder:&str) -> bool {
        *self=Self::default();
        if store_list(&mut self.books)!=0 || store_list_folders(&mut self.folders)!=0 {return false;}
        self.folder=folder.to_string();self.open=true;true
    }
    pub fn draw(&mut self,a:&mut App) {
        hits_clear(&mut self.hits);
        let w=a.w;let h=a.h;let u=ui_px;
        canvas_fill(&mut a.screen,Rect::new(0,0,w,h),255);
        let back=Rect::new(u(30.0),u(20.0),u(150.0),u(150.0));
        ui_button(&mut a.screen,back,IconKind::Back,"",ButtonStyle::Plain);
        hits_add(&mut self.hits,back,1,0);
        let title=self.selected.as_ref().map_or_else(||if self.folder.is_empty(){tr("Notebooks").to_string()}else{self.folder.clone()},|nb|nb.name.as_str().to_string());
        font_draw_left_name(&mut a.screen,FontFace::Bold,ui_title_size(),Rect::new(u(210.0),u(20.0),w-u(250.0),u(150.0)),&title,0);
        let per;
        let total;
        if let Some(nb)=&self.selected {
            let cols=if w>h {6}else{4};let gap=u(35.0);let cw=(w-gap*(cols+1))/cols;
            let ch=u(340.0);let rows=((h-u(400.0))/ch).max(1);per=(cols*rows) as usize;total=nb.page_ids.len();
            self.screen=self.screen.min(total.saturating_sub(1)/per);
            let first=self.screen*per;let last=(first+per).min(total);
            for i in first..last {
                let n=(i-first) as i32;let r=Rect::new(gap+n%cols*(cw+gap),u(220.0)+n/cols*ch,cw,u(235.0));
                let mut thumb=Canvas::default();
                if store_load_page_preview(nb,nb.page_ids[i],&mut thumb){canvas_blit_fit(&mut a.screen,&thumb,r);}
                else {font_draw_centered(&mut a.screen,FontFace::Bold,ui_title_size(),r,&(i+1).to_string(),0);}
                ui_frame(&mut a.screen,r,u(8.0),0,false,u(3.0));
                let name=nav_name(&self.nav,nb.page_ids[i]);
                let label=if name.is_empty(){cformat!(tr("Page %d"),i as i32+1)}else{format!("{} · {}",i+1,name)};
                font_draw_left_name(&mut a.screen,FontFace::Regular,ui_small_size(),Rect::new(r.x,r.y+r.h+u(12.0),r.w,u(65.0)),&label,0);
                hits_add(&mut self.hits,Rect::new(r.x,r.y,r.w,ch-u(15.0)),5,i as i32);
            }
            bg_request_page_previews(&mut self.previews,nb,first,last,a.dpi);
        } else {
            let row=u(150.0);per=((h-u(400.0))/row).max(1) as usize;
            let dirs:Vec<&str>=self.folders.iter().map(|s|s.as_str()).filter(|s|store_folder_parent(s)==self.folder).collect();
            let books:Vec<usize>=self.books.iter().enumerate().filter(|(_,nb)|nb.folder==self.folder.as_str()).map(|(i,_)|i).collect();
            let up=usize::from(!self.folder.is_empty());total=up+dirs.len()+books.len();
            self.screen=self.screen.min(total.saturating_sub(1)/per);
            if total==0 {font_draw_left(&mut a.screen,FontFace::Regular,ui_text_size(),Rect::new(u(80.0),u(250.0),w-u(160.0),u(100.0)),tr("No notebooks here."),0);}
            for k in self.screen*per..((self.screen+1)*per).min(total) {
                let (label,icon,id,arg)=if k<up {("..".to_string(),IconKind::Folder,2,0)}
                    else if k-up<dirs.len(){(store_folder_leaf(dirs[k-up]).to_string(),IconKind::Folder,3,(k-up) as i32)}
                    else {let ix=books[k-up-dirs.len()];(self.books[ix].name.as_str().to_string(),IconKind::Notebook,4,ix as i32)};
                let r=Rect::new(u(50.0),u(220.0)+(k%per) as i32*row,w-u(100.0),row-u(12.0));
                ui_frame(&mut a.screen,r,u(12.0),0,false,u(3.0));
                ui_icon(&mut a.screen,icon,Rect::new(r.x+u(20.0),r.y+u(20.0),u(90.0),r.h-u(40.0)),0);
                font_draw_left_name(&mut a.screen,FontFace::Regular,ui_text_size(),Rect::new(r.x+u(140.0),r.y,r.w-u(170.0),r.h),&label,0);hits_add(&mut self.hits,r,id,arg);
            }
        }
        let bottom=h-u(170.0);
        for (id,enabled,x,icon) in [(6,self.screen>0,u(50.0),IconKind::Prev),(7,(self.screen+1)*per<total,w-u(250.0),IconKind::Next)] {
            let r=Rect::new(x,bottom,u(200.0),u(140.0));ui_button(&mut a.screen,r,icon,"",if enabled{ButtonStyle::Plain}else{ButtonStyle::Disabled});
            if enabled {hits_add(&mut self.hits,r,id,0);}
        }
        let count=format!("{} / {}",self.screen+1,total.div_ceil(per).max(1));
        font_draw_centered(&mut a.screen,FontFace::Regular,ui_text_size(),Rect::new(w/2-u(200.0),bottom,u(400.0),u(140.0)),&count,0);
    }
    pub fn pointer(&mut self,a:&mut App,ph:i32,e:&InEvent)->NotebookPick {
        if ph==0 {press_begin(&mut self.press,e.x,e.y,e.t_ms);if let Some(hit)=hits_find(&self.hits,e.x,e.y){app_press_feedback(a,hit.r);}return NotebookPick::None;}
        press_move(&mut self.press,e.x,e.y);if ph!=2{return NotebookPick::None;}
        let Some(hit)=hits_find(&self.hits,self.press.x0,self.press.y0).copied() else{return NotebookPick::None};
        if !press_is_tap(&self.press,hit.r){return NotebookPick::None;}
        match hit.id {
            1=>{if self.selected.take().is_some(){self.screen=0;}else{self.open=false;return NotebookPick::Cancel;}},
            2=>{self.folder=store_folder_parent(&self.folder).to_string();self.screen=0;},
            3=>{let dirs:Vec<_>=self.folders.iter().filter(|s|store_folder_parent(s)==self.folder).collect();if let Some(dir)=dirs.get(hit.arg as usize){self.folder=(*dir).clone();self.screen=0;}},
            4=>{if let Some(nb)=self.books.get(hit.arg as usize){self.selected=Some(nb.clone());nav_load(nb.id.as_str(),&mut self.nav);self.screen=0;}},
            5=>{if let Some(nb)=&self.selected {if let Some(&pid)=nb.page_ids.get(hit.arg as usize){if let Some(t)=nav_notebook_target(nb.id.as_str(),pid){self.open=false;return NotebookPick::Target(t);}}}},
            6=>self.screen=self.screen.saturating_sub(1),7=>self.screen+=1,_=>{}
        }
        self.draw(a);app_invalidate_all(a,RefreshMode::Ui);NotebookPick::None
    }
    pub fn point(&self,name:&str)->Option<(f32,f32)> {
        let (id,arg)=if name=="back"{(1,0)}else if name=="up"{(2,0)}else if name=="next"{(7,0)}else if name=="previous"{(6,0)}
        else if let Some(n)=name.strip_prefix("page."){(5,n.parse::<i32>().ok()?-1)}
        else if let Some(n)=name.strip_prefix("folder."){(3,self.folders.iter().filter(|s|store_folder_parent(s)==self.folder).position(|s|s==n)? as i32)}
        else if let Some(n)=name.strip_prefix("book."){(4,self.books.iter().position(|b|b.id==n)? as i32)}
        else{return None};
        self.hits.h.iter().find(|h|h.id==id&&h.arg==arg).map(|h|((h.r.x+h.r.w/2)as f32,(h.r.y+h.r.h/2)as f32))
    }
}
