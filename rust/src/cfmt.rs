//! cfmt: printf, as the C's snprintf/fprintf format, for the three places where the format string is
//! the contract: log lines (tests grep grimoire.log), translations (po/*.mo hold printf formats,
//! positional ones too: "%2$d %1$s"), and every other text whose bytes a test or a file holds.
//! A Rust port keeps the C's format string as it is and passes it here (`cformat!`, `qlog!`,
//! `csnprintf!`), instead of rewriting it as a Rust format string.
//!
//! Supported: %d %i %u %x %X %o %c %s %f %F %e %E %g %G %%, flags - + space 0 #, width and
//! precision (numbers or *), positional %N$, and the length modifiers hh h l ll z j t (accepted;
//! the argument's own Rust type decides its width, as C's promotions would: an i32 printed with %u
//! is its u32 bits, as in C). Not %n or %p.

use std::fmt::Write as _;

/// One printf argument. The macros build these from Rust values with `Arg::from`.
#[derive(Clone, Copy, Debug)]
pub enum Arg<'a> {
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    F64(f64),
    Str(&'a str),
}

macro_rules! from_int {
    ($($t:ty => $v:ident as $c:ty),*) => { $(impl From<$t> for Arg<'_> { fn from(x: $t) -> Self { Arg::$v(x as $c) } })* };
}
// C's default promotions: everything narrower than int is passed as int, float as double.
from_int!(i8 => I32 as i32, i16 => I32 as i32, i32 => I32 as i32, u8 => I32 as i32, u16 => I32 as i32,
          i64 => I64 as i64, isize => I64 as i64, u32 => U32 as u32, u64 => U64 as u64, usize => U64 as u64);
impl From<bool> for Arg<'_> {
    fn from(x: bool) -> Self {
        Arg::I32(x as i32)
    }
}
impl From<char> for Arg<'_> {
    fn from(x: char) -> Self {
        Arg::I32(x as i32)
    }
}
impl From<f32> for Arg<'_> {
    fn from(x: f32) -> Self {
        Arg::F64(x as f64)
    }
}
impl From<f64> for Arg<'_> {
    fn from(x: f64) -> Self {
        Arg::F64(x)
    }
}
impl<'a> From<&'a str> for Arg<'a> {
    fn from(x: &'a str) -> Self {
        Arg::Str(x)
    }
}
impl<'a> From<&'a String> for Arg<'a> {
    fn from(x: &'a String) -> Self {
        Arg::Str(x.as_str())
    }
}
impl<'a, const N: usize> From<&'a crate::util::FixedStr<N>> for Arg<'a> {
    fn from(x: &'a crate::util::FixedStr<N>) -> Self {
        Arg::Str(x.as_str())
    }
}

impl Arg<'_> {
    fn as_i64(&self) -> i64 {
        match *self {
            Arg::I32(v) => v as i64,
            Arg::I64(v) => v,
            Arg::U32(v) => v as i32 as i64, // %d of an unsigned int: its bits as int
            Arg::U64(v) => v as i64,
            Arg::F64(v) => v as i64,
            Arg::Str(_) => 0,
        }
    }
    fn as_u64(&self) -> u64 {
        match *self {
            Arg::I32(v) => v as u32 as u64, // %u of an int: its bits as unsigned int
            Arg::I64(v) => v as u64,
            Arg::U32(v) => v as u64,
            Arg::U64(v) => v,
            Arg::F64(v) => v as u64,
            Arg::Str(_) => 0,
        }
    }
    fn as_f64(&self) -> f64 {
        match *self {
            Arg::F64(v) => v,
            Arg::I32(v) => v as f64,
            Arg::I64(v) => v as f64,
            Arg::U32(v) => v as f64,
            Arg::U64(v) => v as f64,
            Arg::Str(_) => 0.0,
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    zero: bool,
    alt: bool,
    width: Option<usize>,
    prec: Option<usize>,
}

/// The text printf(fmt, args...) writes.
pub fn cformat(fmt: &str, args: &[Arg]) -> String {
    let mut out = String::with_capacity(fmt.len() + 16);
    let b = fmt.as_bytes();
    let mut i = 0;
    let mut next = 0usize; // the next sequential argument
    let mut lit = 0;
    let take = |n: usize| -> Option<&Arg> { args.get(n) };
    while i < b.len() {
        if b[i] != b'%' {
            i += 1;
            continue;
        }
        out.push_str(&fmt[lit..i]);
        let start = i;
        i += 1;
        if i < b.len() && b[i] == b'%' {
            out.push('%');
            i += 1;
            lit = i;
            continue;
        }
        // %N$
        let mut pos: Option<usize> = None;
        let mut j = i;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > i && j < b.len() && b[j] == b'$' {
            pos = fmt[i..j].parse::<usize>().ok().map(|n| n.saturating_sub(1));
            i = j + 1;
        }
        let mut s = Spec::default();
        while i < b.len() {
            match b[i] {
                b'-' => s.left = true,
                b'+' => s.plus = true,
                b' ' => s.space = true,
                b'0' => s.zero = true,
                b'#' => s.alt = true,
                _ => break,
            }
            i += 1;
        }
        if i < b.len() && b[i] == b'*' {
            let w = take(next).map_or(0, |a| a.as_i64());
            next += 1;
            if w < 0 {
                s.left = true;
            }
            s.width = Some(w.unsigned_abs() as usize);
            i += 1;
        } else {
            let st = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i > st {
                s.width = fmt[st..i].parse().ok();
            }
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            if i < b.len() && b[i] == b'*' {
                let p = take(next).map_or(0, |a| a.as_i64());
                next += 1;
                s.prec = if p < 0 { None } else { Some(p as usize) };
                i += 1;
            } else {
                let st = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                s.prec = Some(fmt[st..i].parse().unwrap_or(0));
            }
        }
        while i < b.len() && matches!(b[i], b'h' | b'l' | b'z' | b'j' | b't' | b'L' | b'q') {
            i += 1;
        }
        if i >= b.len() {
            out.push_str(&fmt[start..]);
            lit = b.len();
            break;
        }
        let conv = b[i];
        i += 1;
        lit = i;
        let idx = pos.unwrap_or_else(|| {
            next += 1;
            next - 1
        });
        let arg = take(idx).copied().unwrap_or(Arg::I32(0));
        let body = match conv {
            b'd' | b'i' => int_text(arg.as_i64() < 0, arg.as_i64().unsigned_abs().to_string(), &s),
            b'u' => int_text(false, arg.as_u64().to_string(), &s),
            b'x' | b'X' | b'o' => {
                let v = arg.as_u64();
                let v = match arg {
                    Arg::I32(_) | Arg::U32(_) => v as u32 as u64,
                    _ => v,
                };
                let mut d = match conv {
                    b'x' => format!("{v:x}"),
                    b'X' => format!("{v:X}"),
                    _ => format!("{v:o}"),
                };
                if s.alt && v != 0 {
                    d = match conv {
                        b'x' => format!("0x{d}"),
                        b'X' => format!("0X{d}"),
                        _ => format!("0{d}"),
                    };
                }
                let mut s2 = s;
                s2.plus = false;
                s2.space = false;
                int_text(false, d, &s2)
            }
            b'c' => {
                let c = arg.as_i64() as u8;
                pad(String::from_utf8_lossy(&[c]).into_owned(), &Spec { zero: false, ..s })
            }
            b's' => {
                let t = match arg {
                    Arg::Str(t) => t,
                    _ => "",
                };
                let t = match s.prec {
                    Some(p) => crate::util::cut_at_bytes(t, p),
                    None => t,
                };
                pad(t.to_string(), &Spec { zero: false, ..s })
            }
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => float_text(arg.as_f64(), conv, &s),
            _ => {
                // Unknown conversion: printed as written.
                fmt[start..i].to_string()
            }
        };
        out.push_str(&body);
    }
    if lit < b.len() {
        out.push_str(&fmt[lit..]);
    }
    out
}

// An integer's digits with its sign, precision (minimum digits) and width.
fn int_text(neg: bool, mut digits: String, s: &Spec) -> String {
    if let Some(p) = s.prec {
        if p == 0 && digits == "0" {
            digits.clear();
        }
        while digits.len() < p {
            digits.insert(0, '0');
        }
    }
    let sign = if neg {
        "-"
    } else if s.plus {
        "+"
    } else if s.space {
        " "
    } else {
        ""
    };
    let zero = s.zero && !s.left && s.prec.is_none();
    finish(sign, digits, s, zero)
}

fn finish(sign: &str, body: String, s: &Spec, zero: bool) -> String {
    let w = s.width.unwrap_or(0);
    let len = sign.len() + body.len();
    if len >= w {
        return format!("{sign}{body}");
    }
    let fill = w - len;
    if s.left {
        format!("{sign}{body}{}", " ".repeat(fill))
    } else if zero {
        format!("{sign}{}{body}", "0".repeat(fill))
    } else {
        format!("{}{sign}{body}", " ".repeat(fill))
    }
}

fn pad(t: String, s: &Spec) -> String {
    finish("", t, s, false)
}

fn float_text(v: f64, conv: u8, s: &Spec) -> String {
    let upper = conv.is_ascii_uppercase();
    // C prints the sign of -0.0 too ("-0.0"). A NaN's sign is the C library's to decide: musl (the
    // Kindle) and glibc print it as any number's ("-nan", "+nan" with the + flag), Apple's libc
    // (the host the tests run on) never prints one, whatever the flags.
    let nan_unsigned = cfg!(target_vendor = "apple") && v.is_nan();
    let neg = v.is_sign_negative();
    let sign = if nan_unsigned {
        ""
    } else if neg {
        "-"
    } else if s.plus {
        "+"
    } else if s.space {
        " "
    } else {
        ""
    };
    let a = v.abs();
    if !a.is_finite() {
        let t = if a.is_nan() { "nan" } else { "inf" };
        let t = if upper { t.to_uppercase() } else { t.to_string() };
        return finish(sign, t, s, false);
    }
    let prec = s.prec.unwrap_or(6);
    let body = match conv.to_ascii_lowercase() {
        b'f' => {
            let mut t = format!("{a:.prec$}");
            if s.alt && prec == 0 {
                t.push('.');
            }
            t
        }
        b'e' => exp_text(a, prec, s.alt),
        _ => {
            // %g: precision P (0 means 1); exponent X of the value as %e would round it.
            let p = if prec == 0 { 1 } else { prec };
            let e = exp_of(a, p - 1);
            let mut t = if e < -4 || e >= p as i32 {
                exp_text(a, p - 1, s.alt)
            } else {
                let fp = (p as i32 - 1 - e).max(0) as usize;
                format!("{a:.fp$}")
            };
            if !s.alt {
                t = strip_zeros(&t);
            }
            t
        }
    };
    let body = if upper { body.to_uppercase() } else { body };
    finish(sign, body, s, s.zero && !s.left)
}

// The exponent %e prints for a at this precision (after rounding).
fn exp_of(a: f64, prec: usize) -> i32 {
    if a == 0.0 {
        return 0;
    }
    let t = format!("{a:.prec$e}");
    t.rsplit('e').next().and_then(|x| x.parse().ok()).unwrap_or(0)
}

// C's %e: d.ddde+XX, at least two exponent digits.
fn exp_text(a: f64, prec: usize, alt: bool) -> String {
    let t = format!("{a:.prec$e}");
    let (m, e) = t.split_once('e').unwrap_or((&t, "0"));
    let e: i32 = e.parse().unwrap_or(0);
    let mut m = m.to_string();
    if alt && prec == 0 {
        m.push('.');
    }
    let mut out = String::new();
    let _ = write!(out, "{m}e{}{:02}", if e < 0 { '-' } else { '+' }, e.unsigned_abs());
    out
}

// %g without '#': no trailing zeros in the fraction, nor a trailing point.
fn strip_zeros(t: &str) -> String {
    let (m, e) = match t.find('e') {
        Some(i) => (&t[..i], &t[i..]),
        None => (t, ""),
    };
    let m = if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.') } else { m };
    format!("{m}{e}")
}

/// `cformat!("%s %d, %02d:%02d", month, day, h, m)`: the C's printf, to a String.
#[macro_export]
macro_rules! cformat {
    ($fmt:expr $(, $a:expr)* $(,)?) => {
        $crate::cfmt::cformat($fmt, &[$($crate::cfmt::Arg::from($a)),*])
    };
}

/// snprintf(out, cap, fmt, args...): out holds the text cut to cap - 1 bytes (between two characters,
/// see PORTING.md, Strings); returns the length of the whole text, as snprintf does.
pub fn csnprintf(out: &mut String, cap: usize, fmt: &str, args: &[Arg]) -> i32 {
    let t = cformat(fmt, args);
    crate::util::strlcpy_q(out, &t, cap);
    t.len() as i32
}

#[macro_export]
macro_rules! csnprintf {
    ($out:expr, $cap:expr, $fmt:expr $(, $a:expr)* $(,)?) => {
        $crate::cfmt::csnprintf($out, $cap, $fmt, &[$($crate::cfmt::Arg::from($a)),*])
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn like_printf() {
        assert_eq!(cformat!("%s %d, %02d:%02d", "Oct", 10, 4, 5), "Oct 10, 04:05");
        assert_eq!(cformat!("%2$d %1$s", "Oct", 10), "10 Oct");
        assert_eq!(cformat!("%.0f dpi", 300.0f32), "300 dpi");
        assert_eq!(cformat!("%.2f|%.1f|%.0f", 0.125, 2.25, 2.5), "0.12|2.2|2");
        assert_eq!(cformat!("%5.1f|%-5d|%05d|%+d", 3.14159, 42, -42, 7), "  3.1|42   |-0042|+7");
        assert_eq!(cformat!("%u %x %X %08x", -1i32, 255, 255u32, 0xbeefu32), "4294967295 ff FF 0000beef");
        assert_eq!(cformat!("%06x %016llx", 0xabcu32, 1u64), "000abc 0000000000000001");
        assert_eq!(cformat!("%*s|%-*s|%.*s", 4, "ab", 4, "ab", 2, "abc"), "  ab|ab  |ab");
        assert_eq!(cformat!("%g %g %g %g", 100000.0, 1000000.0, 0.0001, 0.5), "100000 1e+06 0.0001 0.5");
        assert_eq!(cformat!("%e", 12345.678), "1.234568e+04");
        assert_eq!(cformat!("100%% %c", 'x'), "100% x");
        assert_eq!(cformat!("%zu %lld %ld", 7usize, -3i64, 9i64), "7 -3 9");
    }

    #[test]
    fn nan_and_inf_as_the_c_library_prints_them() {
        let (n, p) = (-f64::NAN, f64::NAN);
        let got = cformat!("[%f][%+f][% f][%g][%5.1f][%-6f|][%F][%+f][%05f]", n, n, n, n, n, n, n, p, -f64::INFINITY);
        // (what this Mac's clang build of printf and musl's fmt_fp print)
        if cfg!(target_vendor = "apple") {
            assert_eq!(got, "[nan][nan][nan][nan][  nan][nan   |][NAN][nan][ -inf]");
        } else {
            assert_eq!(got, "[-nan][-nan][-nan][-nan][ -nan][-nan  |][-NAN][+nan][ -inf]");
        }
    }

    #[test]
    fn snprintf_cuts() {
        let mut s = String::new();
        assert_eq!(csnprintf!(&mut s, 4, "%s", "abcdef"), 6);
        assert_eq!(s, "abc");
    }
}
