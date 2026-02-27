use std::os::raw::c_char;
use std::ffi::CString;
unsafe extern "C" { 
    fn make_request(url: *const c_char) -> *mut c_char;
}
fn main() {
    let res = unsafe{make_request(c"gemini://geminiprotocol.net/".as_ptr())};
    let res = unsafe {CString::from_raw(res)};
    println!("{res:?}");
}
