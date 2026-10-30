//! Bounded, ink-free link outlines. Rectangles remain the fallback for older links/readers.
use crate::prelude::*;

pub const LINK_OUTLINE_MAX: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkOutline {
    pub n: u8,
    pub closed: bool,
    pub xy: [(i16,i16);LINK_OUTLINE_MAX],
}
impl Default for LinkOutline {
    fn default()->Self {Self{n:0,closed:false,xy:[(0,0);LINK_OUTLINE_MAX]}}
}
fn distance_sq(q:(f32,f32),a:(f32,f32),b:(f32,f32))->f32 {
    let (dx,dy)=(b.0-a.0,b.1-a.1);
    let t=if dx*dx+dy*dy>0.0 {((q.0-a.0)*dx+(q.1-a.1)*dy)/(dx*dx+dy*dy)}else{0.0}.clamp(0.0,1.0);
    (q.0-a.0-t*dx).powi(2)+(q.1-a.1-t*dy).powi(2)
}
impl LinkOutline {
    pub fn valid(&self)->bool {
        self.n==0 || ((self.n as usize)<=LINK_OUTLINE_MAX && self.n>=if self.closed{3}else{2}
            && self.xy[..self.n as usize].iter().all(|&(x,y)|x>=0 && y>=0 && x as i32<=NAV_AREA_MAX && y as i32<=NAV_AREA_MAX))
    }
    pub fn within(&self,r:Rect)->bool {
        self.valid() && (self.n==0 || self.xy[..self.n as usize].iter().all(|&(x,y)|
            x as i32>=r.x && y as i32>=r.y && x as i32<=r.x+r.w && y as i32<=r.y+r.h))
    }
    pub fn from_points(points:&[(f32,f32)],closed:bool,w:i32,h:i32)->Self {
        if points.len()<2 || points.len()>8192 || points.iter().any(|p|!p.0.is_finite() || !p.1.is_finite()){return Self::default();}
        let mut p:Vec<(f32,f32)>=points.iter().map(|&(x,y)|(x.clamp(0.0,w.min(NAV_AREA_MAX).max(0) as f32).round(),y.clamp(0.0,h.min(NAV_AREA_MAX).max(0) as f32).round())).collect();
        p.dedup();
        if closed && p.len()>1 && p.first()!=p.last(){p.push(p[0]);}
        if p.len()<if closed{4}else{2}{return Self::default();}
        // Douglas-Peucker preserves polygon corners; increase tolerance only for dense curves.
        let mut tolerance=0.75f32;
        loop {
            let mut keep=vec![false;p.len()];keep[0]=true;keep[p.len()-1]=true;
            let mut pending=vec![(0,p.len()-1)];
            while let Some((a,b))=pending.pop() {
                let mut far=tolerance*tolerance;let mut at=None;
                for i in a+1..b {let d=distance_sq(p[i],p[a],p[b]);if d>far{far=d;at=Some(i);}}
                if let Some(i)=at {keep[i]=true;pending.push((a,i));pending.push((i,b));}
            }
            if closed {keep[p.len()-1]=false;}
            let count=keep.iter().filter(|&&v|v).count();
            if count<=LINK_OUTLINE_MAX {
                if count<if closed{3}else{2}{return Self::default();}
                let mut out=Self{n:count as u8,closed,..Default::default()};let mut n=0;
                for (i,&v) in keep.iter().enumerate(){if v{out.xy[n]=(p[i].0 as i16,p[i].1 as i16);n+=1;}}
                return out;
            }
            tolerance*=1.5;
        }
    }
    /// Map the saved outline into the same accessible area used by the hit test.
    pub fn points(&self,source:Rect,target:Rect)->Vec<(f32,f32)> {
        if self.n==0 || !self.valid(){return Vec::new();}
        let mut out:Vec<_>=self.xy[..self.n as usize].iter().map(|&(x,y)|(
            target.x as f32+if source.w<=1{target.w as f32*0.5}else{(x as f32-source.x as f32)*target.w as f32/source.w as f32},
            target.y as f32+if source.h<=1{target.h as f32*0.5}else{(y as f32-source.y as f32)*target.h as f32/source.h as f32})).collect();
        if self.closed{out.push(out[0]);}out
    }
    pub fn hit(&self,source:Rect,target:Rect,x:f32,y:f32,min_px:f32)->bool {
        let points=self.points(source,target);
        if points.len()<2{return true;}
        if !self.closed {
            return points.windows(2).any(|p|distance_sq((x,y),p[0],p[1])<=(min_px*0.5).max(1.0).powi(2));
        }
        let mut inside=false;
        for edge in points.windows(2) {
            let (a,b)=(edge[0],edge[1]);
            if distance_sq((x,y),a,b)<=1.0{return true;}
            if (a.1>y)!=(b.1>y) && x<(b.0-a.0)*(y-a.1)/(b.1-a.1)+a.0 {inside=!inside;}
        }
        inside
    }
    pub fn text(&self)->String {
        if !self.valid(){return String::new();}
        let mut text=if self.closed{"c"}else{"o"}.to_string();
        for &(x,y) in &self.xy[..self.n as usize]{text.push_str(&format!(",{x}:{y}"));}text
    }
    pub fn parse(text:&str)->Option<Self> {
        let mut values=text.split(',');let closed=match values.next()?{"c"=>true,"o"=>false,_=>return None};
        let mut out=Self{closed,..Default::default()};
        for value in values {
            if out.n as usize==LINK_OUTLINE_MAX{return None;}
            let (x,y)=value.split_once(':')?;
            if x.is_empty() || y.is_empty() || x.len()>4 || y.len()>4 || !x.bytes().chain(y.bytes()).all(|b|b.is_ascii_digit()){return None;}
            out.xy[out.n as usize]=(x.parse().ok()?,y.parse().ok()?);out.n+=1;
        }
        (out.n>0 && out.valid()).then_some(out)
    }
}
